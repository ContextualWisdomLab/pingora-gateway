//! Real-listener upstream TLS acceptance for the bounded pg-erd migration binary.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use pingora::tls::ssl::{SslAcceptor, SslFiletype, SslMethod};
use tempfile::{tempdir, NamedTempFile};

const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const SOCKET_IO_TIMEOUT: Duration = Duration::from_secs(5);

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

/// Runs one deterministic OpenSSL fixture command and fails closed on any non-zero status.
fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

/// Issues a one-day local CA and `backend.test` certificate without checking secret material in.
fn issue_local_certificates() -> LocalCertificates {
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
        "/CN=CWL PgErd Local Test CA",
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
        "/CN=backend.test",
        "-sha256",
    ]);
    fs::write(
        &server_ext,
        "subjectAltName=DNS:backend.test\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n",
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

/// Builds the TLS server fixture from the generated certificate/key pair.
fn tls_acceptor(certificates: &LocalCertificates) -> SslAcceptor {
    let mut builder =
        SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).expect("TLS acceptor should build");
    builder
        .set_certificate_chain_file(&certificates.server_cert)
        .expect("server certificate should load");
    builder
        .set_private_key_file(&certificates.server_key, SslFiletype::PEM)
        .expect("server private key should load");
    builder
        .check_private_key()
        .expect("server certificate and key should match");
    builder.build()
}

/// Holds traffic and metrics ports simultaneously until immediately before gateway startup.
fn reserve_distinct_loopback_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    assert_ne!(
        traffic.local_addr().expect("traffic reservation address"),
        metrics.local_addr().expect("metrics reservation address")
    );
    (traffic, metrics)
}

/// Writes the bounded migration contract while keeping route authority compiled into the product profile.
fn write_migration_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
    backend_sni: &str,
    ca_cert: &Path,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: true\n    sni: {backend_sni}\n    trust_bundle_file: {}\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        ca_cert.display()
    )
    .expect("migration config should be written");
    file
}

/// Waits for the real listener while failing immediately if the process exits during activation.
fn wait_until_listening(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("migration gateway exited before accepting traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "migration gateway did not start within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

/// Starts the compiled migration binary only after its listener reservations have been released.
fn spawn_gateway(config: &NamedTempFile, listener: SocketAddr) -> GatewayProcess {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 config path"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("compiled migration gateway binary should start");
    wait_until_listening(listener, &mut child);
    GatewayProcess(child)
}

/// Sends one raw HTTP/1.1 request with finite I/O time so a framing defect cannot hang CI.
fn raw_get(address: SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("gateway should accept downstream traffic");
    stream
        .set_read_timeout(Some(SOCKET_IO_TIMEOUT))
        .expect("downstream read timeout should be configurable");
    stream
        .set_write_timeout(Some(SOCKET_IO_TIMEOUT))
        .expect("downstream write timeout should be configurable");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: pg-erd.test\r\nConnection: close\r\n\r\n"
    )
    .expect("downstream request should be writable");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    response
}

/// Reads exactly one bounded HTTP/1 header block; missing framing fails instead of waiting forever.
fn read_request(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).expect("request should be readable");
        assert!(read > 0, "gateway closed the upstream request prematurely");
        request.extend_from_slice(&buffer[..read]);
        assert!(
            request.len() <= MAX_REQUEST_HEADER_BYTES,
            "gateway request headers exceeded the 64 KiB fixture bound"
        );
    }
    String::from_utf8(request).expect("fixture request should be UTF-8")
}

/// Applies finite socket I/O deadlines before TLS handshake or clear-text request capture.
fn set_socket_deadlines(stream: &TcpStream) {
    stream
        .set_read_timeout(Some(SOCKET_IO_TIMEOUT))
        .expect("fixture read timeout should be configurable");
    stream
        .set_write_timeout(Some(SOCKET_IO_TIMEOUT))
        .expect("fixture write timeout should be configurable");
}

/// Emits one deterministic close-delimited fixture response after the request boundary is proven.
fn respond(stream: &mut impl Write, body: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("fixture response should be writable");
}

/// Parses only an exact HTTP/1.1 status token so `2000` or protocol-case lookalikes cannot pass.
fn http11_status(response: &str) -> Option<u16> {
    let mut tokens = response.lines().next()?.split_ascii_whitespace();
    if tokens.next()? != "HTTP/1.1" {
        return None;
    }
    let code = tokens.next()?;
    if code.len() != 3 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    code.parse().ok()
}

/// Proves characterized pg-erd traffic consumes explicit CA trust and matching upstream SNI.
#[test]
fn pg_erd_migration_uses_explicit_ca_and_sni_for_characterized_tls_backend() {
    let certificates = issue_local_certificates();
    let backend_listener = TcpListener::bind("127.0.0.1:0").expect("TLS backend should bind");
    let backend_address = backend_listener.local_addr().expect("TLS backend address");
    let frontend_listener = TcpListener::bind("127.0.0.1:0").expect("frontend should bind");
    let frontend_address = frontend_listener.local_addr().expect("frontend address");
    let acceptor = tls_acceptor(&certificates);

    let backend = thread::spawn(move || {
        let (stream, _) = backend_listener.accept().expect("gateway should connect to backend");
        set_socket_deadlines(&stream);
        let mut stream = acceptor
            .accept(stream)
            .expect("matching SNI plus explicit CA should complete TLS");
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /api/tls HTTP/1.1\r\n"));
        respond(&mut stream, "pg-erd-tls-ok");
    });
    let frontend = thread::spawn(move || {
        let (mut stream, _) = frontend_listener
            .accept()
            .expect("fallback request should reach frontend");
        set_socket_deadlines(&stream);
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /projects/42 HTTP/1.1\r\n"));
        respond(&mut stream, "frontend-ok");
    });

    let (traffic_reservation, metrics_reservation) = reserve_distinct_loopback_listeners();
    let gateway_address = traffic_reservation
        .local_addr()
        .expect("traffic reservation address");
    let metrics_address = metrics_reservation
        .local_addr()
        .expect("metrics reservation address");
    let config = write_migration_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
        "backend.test",
        &certificates.ca_cert,
    );
    drop(traffic_reservation);
    drop(metrics_reservation);
    let _process = spawn_gateway(&config, gateway_address);

    let tls_response = raw_get(gateway_address, "/api/tls");
    assert_eq!(
        http11_status(&tls_response),
        Some(200),
        "matching TLS identity should reach backend: {tls_response:?}"
    );
    assert!(tls_response.ends_with("\r\n\r\npg-erd-tls-ok"));

    let fallback = raw_get(gateway_address, "/projects/42");
    assert_eq!(http11_status(&fallback), Some(200));
    assert!(fallback.ends_with("\r\n\r\nfrontend-ok"));

    backend.join().expect("TLS backend fixture should complete");
    frontend.join().expect("frontend fixture should complete");
}

/// Proves hostname verification fails closed without poisoning readiness or an independent route.
#[test]
fn pg_erd_migration_rejects_tls_hostname_mismatch_without_poisoning_other_routes() {
    let certificates = issue_local_certificates();
    let backend_listener = TcpListener::bind("127.0.0.1:0").expect("TLS backend should bind");
    let backend_address = backend_listener.local_addr().expect("TLS backend address");
    let frontend_listener = TcpListener::bind("127.0.0.1:0").expect("frontend should bind");
    let frontend_address = frontend_listener.local_addr().expect("frontend address");
    let acceptor = tls_acceptor(&certificates);

    let backend = thread::spawn(move || {
        let (stream, _) = backend_listener.accept().expect("gateway should connect to backend");
        set_socket_deadlines(&stream);
        if let Ok(mut stream) = acceptor.accept(stream) {
            let mut byte = [0_u8; 1];
            match stream.read(&mut byte) {
                Ok(0) | Err(_) => {}
                Ok(read) => panic!(
                    "hostname-mismatched TLS backend must receive no HTTP request bytes; received {read} byte(s)"
                ),
            }
        }
    });
    let frontend = thread::spawn(move || {
        let (mut stream, _) = frontend_listener
            .accept()
            .expect("independent route should still reach frontend");
        set_socket_deadlines(&stream);
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /projects/after-tls-error HTTP/1.1\r\n"));
        respond(&mut stream, "frontend-recovered");
    });

    let (traffic_reservation, metrics_reservation) = reserve_distinct_loopback_listeners();
    let gateway_address = traffic_reservation
        .local_addr()
        .expect("traffic reservation address");
    let metrics_address = metrics_reservation
        .local_addr()
        .expect("metrics reservation address");
    let config = write_migration_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
        "wrong.internal.example",
        &certificates.ca_cert,
    );
    drop(traffic_reservation);
    drop(metrics_reservation);
    let _process = spawn_gateway(&config, gateway_address);

    let mismatch = raw_get(gateway_address, "/api/tls-mismatch");
    assert_eq!(
        http11_status(&mismatch),
        Some(502),
        "TLS hostname mismatch must fail closed without route failover: {mismatch:?}"
    );

    let readiness = raw_get(gateway_address, "/readyz");
    assert_eq!(
        http11_status(&readiness),
        Some(200),
        "TLS origin failure must not poison process readiness: {readiness:?}"
    );

    let recovered = raw_get(gateway_address, "/projects/after-tls-error");
    assert_eq!(http11_status(&recovered), Some(200));
    assert!(recovered.ends_with("\r\n\r\nfrontend-recovered"));

    backend
        .join()
        .expect("hostname-mismatch TLS fixture should complete");
    frontend
        .join()
        .expect("recovery frontend fixture should complete");
}

/// Prevents response-prefix assertions from accepting protocol or status-code lookalikes.
#[test]
fn http11_status_parser_rejects_lookalikes() {
    assert_eq!(http11_status("HTTP/1.1 200 OK\r\n\r\n"), Some(200));
    assert_eq!(http11_status("HTTP/1.1 502 Bad Gateway\r\n\r\n"), Some(502));
    assert_eq!(http11_status("HTTP/1.1 2000 Weird\r\n\r\n"), None);
    assert_eq!(http11_status("http/1.1 200 OK\r\n\r\n"), None);
}
