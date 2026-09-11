#![cfg(unix)]

//! Real-wire forwarding metadata acceptance for a TLS-terminated generic gateway.
//!
//! The client deliberately supplies spoofed proxy-identity fields. The gateway must remove those
//! values and report only transport truth it can prove. In this v2 fixture the accepted downstream
//! request is TLS-secured, so RFC 7239 `proto` must describe that incoming scheme as `https` even
//! though the selected upstream transport is clear-text HTTP. No client-IP forwarding chain is
//! required by this generic contract.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use pingora::tls::ssl::{SslConnector, SslMethod, SslVerifyMode};
use tempfile::{tempdir, NamedTempFile};

const IO_BUDGET: Duration = Duration::from_secs(5);
const MAX_ORIGIN_HEADER_BYTES: usize = 64 * 1024;

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Certificates {
    _directory: tempfile::TempDir,
    ca: PathBuf,
    certificate: PathBuf,
    key: PathBuf,
}

fn openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

fn certificates() -> Certificates {
    let directory = tempdir().expect("certificate workspace should be available");
    let ca_key = directory.path().join("ca.key");
    let ca = directory.path().join("ca.crt");
    let key = directory.path().join("server.key");
    let csr = directory.path().join("server.csr");
    let certificate = directory.path().join("server.crt");
    let extensions = directory.path().join("server.ext");

    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-out",
        ca.to_str().expect("UTF-8 CA path"),
        "-subj",
        "/CN=CWL Forwarded Scheme Test CA",
        "-days",
        "1",
        "-sha256",
    ]);
    openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        key.to_str().expect("UTF-8 server key path"),
        "-out",
        csr.to_str().expect("UTF-8 CSR path"),
        "-subj",
        "/CN=gateway.test",
        "-sha256",
    ]);
    fs::write(
        &extensions,
        "subjectAltName=DNS:gateway.test\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n",
    )
    .expect("certificate extensions should be writable");
    openssl(&[
        "x509",
        "-req",
        "-in",
        csr.to_str().expect("UTF-8 CSR path"),
        "-CA",
        ca.to_str().expect("UTF-8 CA path"),
        "-CAkey",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-CAcreateserial",
        "-out",
        certificate.to_str().expect("UTF-8 certificate path"),
        "-days",
        "1",
        "-sha256",
        "-extfile",
        extensions.to_str().expect("UTF-8 extension path"),
    ]);

    Certificates {
        _directory: directory,
        ca,
        certificate,
        key,
    }
}

fn reserve_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    assert_ne!(traffic.local_addr().unwrap(), metrics.local_addr().unwrap());
    (traffic, metrics)
}

fn config(
    listener: SocketAddr,
    metrics: SocketAddr,
    upstream: SocketAddr,
    certificates: &Certificates,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificates.certificate.display(),
        certificates.key.display(),
    )
    .expect("gateway config should be written");
    file
}

fn spawn_gateway(config: &NamedTempFile, traffic: TcpListener, metrics: TcpListener) -> Child {
    drop(traffic);
    drop(metrics);
    Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 config path"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("compiled gateway binary should start")
}

fn connect_tls_http1(
    address: SocketAddr,
    certificates: &Certificates,
    process: &mut Child,
) -> pingora::tls::ssl::SslStream<TcpStream> {
    let mut builder =
        SslConnector::builder(SslMethod::tls_client()).expect("TLS client should build");
    builder
        .set_ca_file(&certificates.ca)
        .expect("local CA should load");
    builder.set_verify(SslVerifyMode::PEER);
    builder
        .set_alpn_protos(b"\x08http/1.1")
        .expect("HTTP/1.1 ALPN wire list should be valid");
    let connector = builder.build();
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before TLS handshake: {status}");
        }
        if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            stream
                .set_read_timeout(Some(IO_BUDGET))
                .expect("downstream read timeout should be set");
            stream
                .set_write_timeout(Some(IO_BUDGET))
                .expect("downstream write timeout should be set");
            if let Ok(tls) = connector.connect("gateway.test", stream) {
                return tls;
            }
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not accept verified TLS/HTTP1 within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn read_headers(stream: &mut TcpStream) -> String {
    let deadline = Instant::now() + IO_BUDGET;
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "origin headers exceeded I/O budget");
        stream
            .set_read_timeout(Some(remaining))
            .expect("origin read deadline should be set");
        let read = stream
            .read(&mut buffer)
            .expect("origin headers should read");
        assert!(read > 0, "gateway closed before origin headers completed");
        raw.extend_from_slice(&buffer[..read]);
        assert!(raw.len() <= MAX_ORIGIN_HEADER_BYTES);
        if raw.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&raw).into_owned();
        }
    }
}

fn header_lines(raw: &str) -> Vec<String> {
    raw.split("\r\n")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .map(|line| line.to_ascii_lowercase())
        .collect()
}

#[test]
fn tls_termination_rebuilds_forwarded_scheme_from_transport_and_drops_spoofed_identity() {
    let certificates = certificates();
    let origin_listener = TcpListener::bind("127.0.0.1:0").expect("origin should bind");
    let origin_address = origin_listener.local_addr().expect("origin address");
    let origin = thread::spawn(move || {
        let (mut stream, _) = origin_listener
            .accept()
            .expect("gateway should reach origin");
        let request = read_headers(&mut stream);
        let headers = header_lines(&request);
        assert!(headers.iter().any(|line| line == "forwarded: proto=https"));
        for forbidden in [
            "for=203.0.113.77",
            "x-forwarded-for:",
            "x-forwarded-host:",
            "x-forwarded-port:",
            "x-forwarded-proto:",
            "x-forwarded-server:",
            "x-real-ip:",
        ] {
            assert!(
                !headers.iter().any(|line| line.contains(forbidden)),
                "untrusted forwarding identity survived sanitization: {headers:?}"
            );
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("origin response should write");
    });

    let (traffic, metrics) = reserve_listeners();
    let listener = traffic.local_addr().expect("traffic address");
    let metrics_address = metrics.local_addr().expect("metrics address");
    let config = config(listener, metrics_address, origin_address, &certificates);
    let mut process = GatewayProcess(spawn_gateway(&config, traffic, metrics));
    let mut tls = connect_tls_http1(listener, &certificates, &mut process.0);
    assert_eq!(
        tls.ssl().selected_alpn_protocol(),
        Some(b"http/1.1".as_slice())
    );
    tls.write_all(
        b"GET /forwarded HTTP/1.1\r\nHost: gateway.test\r\nForwarded: for=203.0.113.77;proto=http\r\nX-Forwarded-For: 203.0.113.77\r\nX-Forwarded-Host: attacker.example\r\nX-Forwarded-Port: 4444\r\nX-Forwarded-Proto: http\r\nX-Forwarded-Server: attacker\r\nX-Real-IP: 203.0.113.77\r\nConnection: close\r\n\r\n",
    )
    .expect("TLS request should write");
    tls.flush().expect("TLS request should flush");
    let mut response = Vec::new();
    tls.read_to_end(&mut response)
        .expect("gateway response should be readable");
    assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
    origin.join().expect("origin fixture should complete");
}
