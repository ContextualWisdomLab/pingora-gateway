//! Executable TLS trust and hostname-verification contract through the compiled gateway.

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
        "/CN=CWL Local Test CA",
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
        "/CN=upstream.test",
        "-sha256",
    ]);
    fs::write(
        &server_ext,
        "subjectAltName=DNS:upstream.test\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n",
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

fn write_gateway_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
    sni: &str,
    ca_cert: &Path,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: local-tls-fixture\n    address: {upstream}\n    tls: true\n    sni: {sni}\n    trust_bundle_file: {}\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        ca_cert.display()
    )
    .expect("gateway config should be written");
    file
}

fn wait_until_listening(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before accepting traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not start within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn raw_get(address: SocketAddr) -> String {
    let mut stream = TcpStream::connect(address).expect("gateway should accept downstream traffic");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    stream
        .write_all(
            b"GET /through-local-tls HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n",
        )
        .expect("downstream request should be writable");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    response
}

fn spawn_gateway(config: &NamedTempFile) -> GatewayProcess {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 config path"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("compiled gateway binary should start");
    let listener = extract_listener(config);
    wait_until_listening(listener, &mut child);
    GatewayProcess(child)
}

fn extract_listener(config: &NamedTempFile) -> SocketAddr {
    let source = fs::read_to_string(config.path()).expect("gateway config should be readable");
    source
        .lines()
        .find_map(|line| line.strip_prefix("listener: "))
        .expect("listener must be present")
        .parse()
        .expect("listener must be a socket address")
}

#[test]
fn compiled_gateway_trusts_an_explicit_local_ca_and_rejects_hostname_mismatch() {
    let certificates = issue_local_certificates();

    let valid_listener = TcpListener::bind("127.0.0.1:0").expect("TLS fixture should bind");
    let valid_upstream = valid_listener.local_addr().expect("TLS fixture address");
    let valid_acceptor = tls_acceptor(&certificates);
    let valid_fixture = thread::spawn(move || {
        let (stream, _) = valid_listener.accept().expect("gateway should connect");
        let mut stream = valid_acceptor
            .accept(stream)
            .expect("trusted CA plus matching hostname should complete TLS");
        let mut request = [0_u8; 4096];
        let read = stream
            .read(&mut request)
            .expect("TLS request should be readable");
        assert!(read > 0);
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\nlocal-tls-ok",
            )
            .expect("TLS fixture response should be writable");
    });

    let (valid_gateway, valid_metrics) = reserve_distinct_loopback_addresses();
    let valid_config = write_gateway_config(
        valid_gateway,
        valid_metrics,
        valid_upstream,
        "upstream.test",
        &certificates.ca_cert,
    );
    let _valid_process = spawn_gateway(&valid_config);
    let valid_response = raw_get(valid_gateway);
    assert!(
        valid_response.starts_with("HTTP/1.1 200"),
        "explicit local CA should be trusted: {valid_response:?}"
    );
    assert!(valid_response.ends_with("\r\n\r\nlocal-tls-ok"));
    valid_fixture
        .join()
        .expect("valid TLS fixture should complete");

    let mismatch_listener = TcpListener::bind("127.0.0.1:0").expect("TLS fixture should bind");
    let mismatch_upstream = mismatch_listener.local_addr().expect("TLS fixture address");
    let mismatch_acceptor = tls_acceptor(&certificates);
    let mismatch_fixture = thread::spawn(move || {
        let (stream, _) = mismatch_listener.accept().expect("gateway should connect");
        assert!(
            mismatch_acceptor.accept(stream).is_err(),
            "hostname mismatch must abort the TLS handshake"
        );
    });

    let (mismatch_gateway, mismatch_metrics) = reserve_distinct_loopback_addresses();
    let mismatch_config = write_gateway_config(
        mismatch_gateway,
        mismatch_metrics,
        mismatch_upstream,
        "wrong.internal.example",
        &certificates.ca_cert,
    );
    let _mismatch_process = spawn_gateway(&mismatch_config);
    let mismatch_response = raw_get(mismatch_gateway);
    assert!(
        mismatch_response.starts_with("HTTP/1.1 502"),
        "hostname mismatch must fail closed at the gateway: {mismatch_response:?}"
    );
    mismatch_fixture
        .join()
        .expect("hostname-mismatch fixture should complete");
}
