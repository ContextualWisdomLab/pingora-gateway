//! Real-wire downstream TLS/H2 acceptance for the generic gateway composition root.

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

fn write_gateway_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
    certificates: &LocalCertificates,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificates.server_cert.display(),
        certificates.server_key.display(),
    )
    .expect("gateway config should be written");
    file
}

fn spawn_gateway(config: &NamedTempFile) -> Child {
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

fn connect_h2(
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
        .set_alpn_protos(b"\x02h2")
        .expect("h2 ALPN wire list should be valid");
    let connector = builder.build();

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before downstream TLS/H2 handshake: {status}");
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
            "gateway did not accept a verified TLS/H2 connection within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn write_h2_frame(
    stream: &mut impl Write,
    frame_type: u8,
    flags: u8,
    stream_id: u32,
    payload: &[u8],
) {
    assert!(payload.len() <= 0x00ff_ffff);
    let length = payload.len() as u32;
    let mut header = [0_u8; 9];
    header[0] = ((length >> 16) & 0xff) as u8;
    header[1] = ((length >> 8) & 0xff) as u8;
    header[2] = (length & 0xff) as u8;
    header[3] = frame_type;
    header[4] = flags;
    header[5..9].copy_from_slice(&(stream_id & 0x7fff_ffff).to_be_bytes());
    stream
        .write_all(&header)
        .expect("H2 frame header should write");
    stream
        .write_all(payload)
        .expect("H2 frame payload should write");
}

fn read_h2_frame(stream: &mut impl Read) -> (u8, u8, u32, Vec<u8>) {
    let mut header = [0_u8; 9];
    stream
        .read_exact(&mut header)
        .expect("H2 frame header should be readable");
    let length = ((header[0] as usize) << 16) | ((header[1] as usize) << 8) | header[2] as usize;
    let stream_id = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7fff_ffff;
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .expect("H2 frame payload should be readable");
    (header[3], header[4], stream_id, payload)
}

fn scrape_metrics(address: SocketAddr) -> String {
    let mut stream = TcpStream::connect(address).expect("metrics listener should accept traffic");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("metrics read timeout should be set");
    stream
        .write_all(b"GET /metrics HTTP/1.1\r\nHost: metrics\r\nConnection: close\r\n\r\n")
        .expect("metrics request should write");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("metrics response should be readable");
    response
}

fn contains_exact_metric_sample(metrics: &str, sample: &str) -> bool {
    metrics
        .lines()
        .any(|line| line.trim_end_matches('\r') == sample)
}

#[test]
fn generic_gateway_negotiates_h2_over_verified_downstream_tls_and_proxies_real_traffic() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");
    let upstream_fixture = thread::spawn(move || {
        let (mut stream, _) = upstream_listener
            .accept()
            .expect("gateway should connect upstream");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("upstream read timeout should be set");
        let mut request = [0_u8; 4096];
        let read = stream
            .read(&mut request)
            .expect("upstream request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(
            request.starts_with("GET / HTTP/1.1\r\n"),
            "H2 downstream request must reach the cleartext H1 fixture through the gateway: {request:?}"
        );
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\ntls-h2-ok",
            )
            .expect("upstream response should be writable");
    });

    let (listener, metrics_listener) = reserve_distinct_loopback_addresses();
    let config = write_gateway_config(listener, metrics_listener, upstream, &certificates);
    let mut process = GatewayProcess(spawn_gateway(&config));
    let mut tls = connect_h2(listener, &certificates, &mut process.0);

    assert_eq!(
        tls.ssl().selected_alpn_protocol(),
        Some(b"h2".as_slice()),
        "version-2 downstream TLS must negotiate h2 when the client offers h2"
    );

    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 connection preface should write");
    write_h2_frame(&mut tls, 0x4, 0, 0, &[]);

    // HPACK: indexed :method GET (2), indexed :scheme https (7), indexed :path / (4),
    // then a non-indexed literal :authority (static-table name index 1) = gateway.test.
    let mut headers = vec![0x82, 0x87, 0x84, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    write_h2_frame(&mut tls, 0x1, 0x5, 1, &headers);
    tls.flush().expect("H2 request should flush");

    let mut response_body = Vec::new();
    let mut stream_one_ended = false;
    for _ in 0..32 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        if frame_type == 0x4 && stream_id == 0 && flags & 0x1 == 0 {
            write_h2_frame(&mut tls, 0x4, 0x1, 0, &[]);
            tls.flush().expect("SETTINGS acknowledgement should flush");
            continue;
        }
        if stream_id != 1 {
            continue;
        }
        if frame_type == 0x0 {
            response_body.extend_from_slice(&payload);
        }
        if flags & 0x1 != 0 {
            stream_one_ended = true;
            break;
        }
    }

    assert!(
        stream_one_ended,
        "H2 response stream must terminate normally"
    );
    assert_eq!(response_body, b"tls-h2-ok");
    upstream_fixture
        .join()
        .expect("cleartext upstream fixture should complete");

    let metrics = scrape_metrics(metrics_listener);
    assert!(
        contains_exact_metric_sample(
            &metrics,
            "cwl_pingora_gateway_requests_by_transport_total{outcome=\"ok\",protocol=\"h2\",transport=\"tls\"} 1"
        ),
        "cutover telemetry must distinguish successful TLS/H2 traffic from aggregate request totals: {metrics:?}"
    );
}

#[test]
fn generic_gateway_rejects_a_client_that_offers_only_an_unsupported_alpn_protocol() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");
    let (listener, metrics_listener) = reserve_distinct_loopback_addresses();
    let config = write_gateway_config(listener, metrics_listener, upstream, &certificates);
    let mut process = GatewayProcess(spawn_gateway(&config));

    // Establish one verified supported handshake first so a later failure cannot be attributed to
    // startup timing, certificate identity, or CA materialization.
    drop(connect_h2(listener, &certificates, &mut process.0));

    let mut builder =
        SslConnector::builder(SslMethod::tls_client()).expect("TLS client should build");
    builder
        .set_ca_file(&certificates.ca_cert)
        .expect("local CA should load");
    builder.set_verify(SslVerifyMode::PEER);
    builder
        .set_alpn_protos(b"\x03foo")
        .expect("unsupported ALPN wire list should still be syntactically valid");
    let connector = builder.build();
    let stream = TcpStream::connect_timeout(&listener, Duration::from_secs(2))
        .expect("ready gateway should accept the TCP connection");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream read timeout should be set");
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("downstream write timeout should be set");

    assert!(
        connector.connect("gateway.test", stream).is_err(),
        "RFC 7301 requires a fatal no_application_protocol outcome when the client sends ALPN but offers no protocol supported by the h2_http1 contract"
    );
}
