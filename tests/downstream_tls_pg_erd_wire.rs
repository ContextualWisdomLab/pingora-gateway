//! Real-wire downstream TLS acceptance for the characterized pg-erd composition root.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use pingora::tls::ssl::{SslConnector, SslMethod, SslVerifyMode};
use tempfile::{tempdir, NamedTempFile};

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct LocalCertificates {
    _directory: tempfile::TempDir,
    ca_cert: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
}

fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

fn issue_gateway_certificate() -> LocalCertificates {
    let directory = tempdir().expect("certificate workspace should be available");
    let ca_key = directory.path().join("ca.key");
    let ca_cert = directory.path().join("ca.crt");
    let server_key = directory.path().join("server.key");
    let server_csr = directory.path().join("server.csr");
    let server_cert = directory.path().join("server.crt");
    let server_ext = directory.path().join("server.ext");

    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-out",
        ca_cert.to_str().expect("UTF-8 CA certificate path"),
        "-subj",
        "/CN=CWL Downstream TLS Test CA",
        "-days",
        "1",
        "-sha256",
    ]);
    run_openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        server_key.to_str().expect("UTF-8 server key path"),
        "-out",
        server_csr.to_str().expect("UTF-8 server CSR path"),
        "-subj",
        "/CN=gateway.test",
        "-sha256",
    ]);
    fs::write(
        &server_ext,
        "subjectAltName=DNS:gateway.test\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n",
    )
    .expect("server certificate extension file should be writable");
    run_openssl(&[
        "x509",
        "-req",
        "-in",
        server_csr.to_str().expect("UTF-8 server CSR path"),
        "-CA",
        ca_cert.to_str().expect("UTF-8 CA certificate path"),
        "-CAkey",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-CAcreateserial",
        "-out",
        server_cert.to_str().expect("UTF-8 server certificate path"),
        "-days",
        "1",
        "-sha256",
        "-extfile",
        server_ext.to_str().expect("UTF-8 extension path"),
    ]);

    LocalCertificates {
        _directory: directory,
        ca_cert,
        server_cert,
        server_key,
    }
}

fn reserve_distinct_loopback_addresses() -> (SocketAddr, SocketAddr) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    let addresses = (
        traffic
            .local_addr()
            .expect("traffic reservation has an address"),
        metrics
            .local_addr()
            .expect("metrics reservation has an address"),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

fn write_pg_erd_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
    certificates: &LocalCertificates,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 3\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nmax_upstream_response_body_ms: 5000\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificates.server_cert.display(),
        certificates.server_key.display(),
    )
    .expect("pg-erd config should be written");
    file
}

fn spawn_gateway(config: &NamedTempFile) -> Child {
    Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 config path"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("compiled pg-erd gateway binary should start")
}

fn connect_http1(
    address: SocketAddr,
    certificates: &LocalCertificates,
    process: &mut Child,
) -> pingora::tls::ssl::SslStream<TcpStream> {
    let mut builder =
        SslConnector::builder(SslMethod::tls_client()).expect("TLS client should build");
    builder
        .set_ca_file(&certificates.ca_cert)
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
            panic!("pg-erd gateway exited before downstream TLS handshake: {status}");
        }
        if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("downstream read timeout should be set");
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .expect("downstream write timeout should be set");
            if let Ok(tls) = connector.connect("gateway.test", stream) {
                return tls;
            }
        }
        assert!(
            Instant::now() < deadline,
            "pg-erd gateway did not accept verified TLS within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn pg_erd_version_three_tls_listener_routes_healthz_over_real_http1_fallback() {
    let certificates = issue_gateway_certificate();
    let backend_listener = TcpListener::bind("127.0.0.1:0").expect("backend should bind");
    let backend = backend_listener.local_addr().expect("backend address");
    let frontend_listener = TcpListener::bind("127.0.0.1:0").expect("frontend should bind");
    let frontend = frontend_listener.local_addr().expect("frontend address");

    let backend_fixture = thread::spawn(move || {
        let (mut stream, _) = backend_listener
            .accept()
            .expect("gateway should connect to characterized backend");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("backend read timeout should be set");
        let mut request = [0_u8; 4096];
        let read = stream
            .read(&mut request)
            .expect("backend request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(
            request.starts_with("GET /healthz HTTP/1.1\r\n"),
            "version-3 TLS listener must preserve the characterized backend route: {request:?}"
        );
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\npg-erd-tls-ok",
            )
            .expect("backend response should be writable");
    });

    let (listener, metrics_listener) = reserve_distinct_loopback_addresses();
    let config = write_pg_erd_config(
        listener,
        metrics_listener,
        backend,
        frontend,
        &certificates,
    );
    let mut child = spawn_gateway(&config);
    let mut tls = connect_http1(listener, &certificates, &mut child);
    let _process = GatewayProcess(child);

    assert_eq!(
        tls.ssl().selected_alpn_protocol(),
        Some(b"http/1.1".as_slice()),
        "pg-erd version 3 must retain HTTP/1.1 fallback under h2_http1"
    );

    tls.write_all(
        b"GET /healthz HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n",
    )
    .expect("pg-erd HTTPS request should write");
    tls.flush().expect("pg-erd HTTPS request should flush");

    let mut response = Vec::new();
    tls.read_to_end(&mut response)
        .expect("pg-erd HTTPS response should be readable");
    let response = String::from_utf8(response).expect("fixture response must be UTF-8");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.ends_with("pg-erd-tls-ok"));

    backend_fixture
        .join()
        .expect("characterized backend fixture should complete");
    drop(frontend_listener);
}
