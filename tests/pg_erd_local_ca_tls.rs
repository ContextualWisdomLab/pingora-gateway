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

fn raw_get(address: SocketAddr, path: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("gateway should accept downstream traffic");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
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

fn read_request(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).expect("request should be readable");
        assert!(read > 0, "gateway closed the upstream request prematurely");
        request.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(request).expect("fixture request should be UTF-8")
}

fn respond(stream: &mut impl Write, body: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("fixture response should be writable");
}

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
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /projects/42 HTTP/1.1\r\n"));
        respond(&mut stream, "frontend-ok");
    });

    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_migration_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
        "backend.test",
        &certificates.ca_cert,
    );
    let _process = spawn_gateway(&config, gateway_address);

    let tls_response = raw_get(gateway_address, "/api/tls");
    assert!(
        tls_response.starts_with("HTTP/1.1 200"),
        "matching TLS identity should reach backend: {tls_response:?}"
    );
    assert!(tls_response.ends_with("\r\n\r\npg-erd-tls-ok"));

    let fallback = raw_get(gateway_address, "/projects/42");
    assert!(fallback.starts_with("HTTP/1.1 200"));
    assert!(fallback.ends_with("\r\n\r\nfrontend-ok"));

    backend.join().expect("TLS backend fixture should complete");
    frontend.join().expect("frontend fixture should complete");
}

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
        assert!(
            acceptor.accept(stream).is_err(),
            "client hostname verification must abort the TLS handshake"
        );
    });
    let frontend = thread::spawn(move || {
        let (mut stream, _) = frontend_listener
            .accept()
            .expect("independent route should still reach frontend");
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /projects/after-tls-error HTTP/1.1\r\n"));
        respond(&mut stream, "frontend-recovered");
    });

    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_migration_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
        "wrong.internal.example",
        &certificates.ca_cert,
    );
    let _process = spawn_gateway(&config, gateway_address);

    let mismatch = raw_get(gateway_address, "/api/tls-mismatch");
    assert!(
        mismatch.starts_with("HTTP/1.1 502"),
        "TLS hostname mismatch must fail closed without route failover: {mismatch:?}"
    );

    let readiness = raw_get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "TLS origin failure must not poison process readiness: {readiness:?}"
    );

    let recovered = raw_get(gateway_address, "/projects/after-tls-error");
    assert!(recovered.starts_with("HTTP/1.1 200"));
    assert!(recovered.ends_with("\r\n\r\nfrontend-recovered"));

    backend
        .join()
        .expect("hostname-mismatch TLS fixture should complete");
    frontend
        .join()
        .expect("recovery frontend fixture should complete");
}
