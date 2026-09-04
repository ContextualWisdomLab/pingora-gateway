//! Real-wire supplier RED for RFC 9113 §8.2.3 H2-to-H1 Cookie normalization.
//!
//! This test deliberately exercises a test-only TLS/H2 listener built from the same `GatewayProxy`
//! and pinned Pingora supplier as production, while keeping the characterized upstream on HTTP/1.1.
//! It is expected to fail on Pingora `09696b51bc59315353d96686355861604d0bb48c` because the
//! supplier currently forwards multiple HTTP/2 Cookie fields as multiple HTTP/1.1 header lines.
//! The product listener contract is not widened by this fixture.

#![cfg(unix)]

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::edge_contract::GatewayConfig;
use cwl_pingora_gateway::gateway_proxy::GatewayProxy;
use cwl_pingora_gateway::runtime_policy::build_server_conf;
use pingora::listeners::tls::TlsSettings;
use pingora::prelude::{http_proxy_service, Server};
use pingora::server::RunArgs;
use tempfile::{tempdir, NamedTempFile};

const HELPER_MODE_ENV: &str = "CWL_H2_H1_COOKIE_HELPER";
const HELPER_LISTENER_ENV: &str = "CWL_H2_H1_COOKIE_LISTENER";
const HELPER_METRICS_ENV: &str = "CWL_H2_H1_COOKIE_METRICS";
const HELPER_UPSTREAM_ENV: &str = "CWL_H2_H1_COOKIE_UPSTREAM";
const HELPER_CERT_ENV: &str = "CWL_H2_H1_COOKIE_CERT";
const HELPER_KEY_ENV: &str = "CWL_H2_H1_COOKIE_KEY";

struct HelperProcess(Child);

impl Drop for HelperProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct LocalCertificate {
    _directory: tempfile::TempDir,
    cert: PathBuf,
    key: PathBuf,
}

fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

fn issue_local_certificate() -> LocalCertificate {
    let directory = tempdir().expect("certificate workspace should be available");
    let key = directory.path().join("h2-test.key");
    let cert = directory.path().join("h2-test.crt");
    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        key.to_str().expect("UTF-8 key path"),
        "-out",
        cert.to_str().expect("UTF-8 certificate path"),
        "-subj",
        "/CN=h2.test",
        "-addext",
        "subjectAltName=DNS:h2.test",
        "-days",
        "1",
        "-sha256",
    ]);
    LocalCertificate {
        _directory: directory,
        cert,
        key,
    }
}

fn reserve_loopback_address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .expect("loopback port should be available")
        .local_addr()
        .expect("loopback listener should expose its address")
}

fn wait_until_listening(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("helper process state should be readable")
        {
            panic!("H2 helper exited before accepting traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(Instant::now() < deadline, "H2 helper did not start within 10s");
        thread::sleep(Duration::from_millis(25));
    }
}

fn read_request_headers(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("upstream read timeout should be configurable");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2048];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).expect("upstream request should be readable");
        assert!(read > 0, "gateway closed before sending an HTTP/1.1 request header");
        request.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(request).expect("fixture request header should be UTF-8")
}

fn write_ok(stream: &mut TcpStream) {
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        .expect("fixture response should be writable");
}

fn spawn_h1_origin(listener: TcpListener) -> mpsc::Receiver<String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("gateway should connect to H1 origin");
        let request = read_request_headers(&mut stream);
        write_ok(&mut stream);
        sender
            .send(request)
            .expect("test should still be waiting for origin evidence");
    });
    receiver
}

fn helper_config(listener: SocketAddr, metrics: SocketAddr, upstream: SocketAddr) -> GatewayConfig {
    GatewayConfig::from_yaml(&format!(
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: h1-origin\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000\n"
    ))
    .expect("test-only gateway configuration should be valid")
}

fn env_socket(name: &str) -> SocketAddr {
    env::var(name)
        .unwrap_or_else(|_| panic!("missing helper environment variable {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid socket address in {name}"))
}

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(env::var_os(name).unwrap_or_else(|| panic!("missing helper path {name}")))
}

#[test]
#[ignore = "test-only Pingora TLS/H2 server process; invoked by the real-wire parent test"]
fn h2_cookie_proxy_helper() {
    assert_eq!(env::var(HELPER_MODE_ENV).as_deref(), Ok("1"));
    let listener = env_socket(HELPER_LISTENER_ENV);
    let metrics = env_socket(HELPER_METRICS_ENV);
    let upstream = env_socket(HELPER_UPSTREAM_ENV);
    let cert = env_path(HELPER_CERT_ENV);
    let key = env_path(HELPER_KEY_ENV);

    let config = helper_config(listener, metrics, upstream);
    let proxy = GatewayProxy::try_from_config(&config).expect("test-only proxy should activate");
    let mut server = Server::new_with_opt_and_conf(
        None,
        build_server_conf(config.upstream_keepalive_pool_size),
    );
    server.bootstrap();

    let mut service = http_proxy_service(&server.configuration, proxy);
    let mut tls_settings = TlsSettings::intermediate(
        cert.to_str().expect("UTF-8 certificate path"),
        key.to_str().expect("UTF-8 key path"),
    )
    .expect("test TLS settings should load");
    tls_settings.enable_h2();
    service.add_tls_with_settings(&listener.to_string(), None, tls_settings);
    server.add_service(service);
    server.run(RunArgs::default());
}

fn assert_curl_supports_http2() {
    let output = Command::new("curl")
        .arg("--version")
        .output()
        .expect("CI must provide curl for the real-wire protocol fixture");
    assert!(output.status.success(), "curl --version should succeed");
    let version = String::from_utf8(output.stdout).expect("curl version output should be UTF-8");
    assert!(
        version.lines().any(|line| line.starts_with("Features:") && line.split_whitespace().any(|feature| feature == "HTTP2")),
        "curl must expose HTTP2 support for this fixture: {version}"
    );
}

fn spawn_helper(
    listener: SocketAddr,
    metrics: SocketAddr,
    upstream: SocketAddr,
    cert: &Path,
    key: &Path,
) -> HelperProcess {
    let mut child = Command::new(env::current_exe().expect("current test executable should resolve"))
        .args(["--ignored", "--exact", "h2_cookie_proxy_helper", "--nocapture"])
        .env(HELPER_MODE_ENV, "1")
        .env(HELPER_LISTENER_ENV, listener.to_string())
        .env(HELPER_METRICS_ENV, metrics.to_string())
        .env(HELPER_UPSTREAM_ENV, upstream.to_string())
        .env(HELPER_CERT_ENV, cert)
        .env(HELPER_KEY_ENV, key)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("test-only H2 proxy helper should start");
    wait_until_listening(listener, &mut child);
    HelperProcess(child)
}

fn traced_curl_h2_request(listener: SocketAddr, cert: &Path, trace: &NamedTempFile) -> String {
    let authority = format!("h2.test:{}", listener.port());
    let resolve = format!("{authority}:127.0.0.1");
    let url = format!("https://{authority}/cookie-wire");
    let output = Command::new("curl")
        .args([
            "--fail-with-body",
            "--silent",
            "--show-error",
            "--http2",
            "--noproxy",
            "*",
            "--resolve",
            &resolve,
            "--header",
            "Cookie: session_id=abc123",
            "--header",
            "Cookie: preferred_language=en",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_version}",
            "--trace-ascii",
        ])
        .arg(trace.path())
        .args(["--cacert"])
        .arg(cert)
        .arg(url)
        .output()
        .expect("curl H2 request should execute");
    assert!(
        output.status.success(),
        "curl H2 request failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("curl write-out should be UTF-8")
}

fn cookie_header_values(raw_request: &str) -> Vec<&str> {
    raw_request
        .split("\r\n")
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("cookie").then_some(value.trim())
        })
        .collect()
}

#[test]
fn h2_multiple_cookie_fields_are_coalesced_before_h1_upstream() {
    assert_curl_supports_http2();
    let certificate = issue_local_certificate();
    let origin = TcpListener::bind("127.0.0.1:0").expect("H1 origin should bind");
    let origin_address = origin.local_addr().expect("H1 origin address should resolve");
    let origin_request = spawn_h1_origin(origin);
    let listener = reserve_loopback_address();
    let metrics = reserve_loopback_address();
    assert_ne!(listener, metrics);
    assert_ne!(listener, origin_address);
    assert_ne!(metrics, origin_address);

    let _helper = spawn_helper(
        listener,
        metrics,
        origin_address,
        &certificate.cert,
        &certificate.key,
    );
    let trace = NamedTempFile::new().expect("curl trace file should be writable");
    let negotiated_version = traced_curl_h2_request(listener, &certificate.cert, &trace);
    assert_eq!(negotiated_version.trim(), "2", "fixture must negotiate HTTP/2");

    let client_trace = fs::read_to_string(trace.path()).expect("curl trace should be readable");
    let lower_trace = client_trace.to_ascii_lowercase();
    assert!(
        lower_trace.contains("cookie: session_id=abc123")
            && lower_trace.contains("cookie: preferred_language=en"),
        "client fixture must originate two distinct Cookie fields: {client_trace}"
    );

    let raw_request = origin_request
        .recv_timeout(Duration::from_secs(5))
        .expect("H1 origin should receive the translated request");
    assert!(
        raw_request.starts_with("GET /cookie-wire HTTP/1.1\r\n"),
        "fixture must exercise the H2-downstream to H1-upstream proxy path: {raw_request}"
    );
    assert_eq!(
        cookie_header_values(&raw_request),
        vec!["session_id=abc123; preferred_language=en"],
        "RFC 9113 §8.2.3 requires multiple H2 Cookie fields to become one H1 Cookie field joined by semicolon+space; pinned Pingora main is expected to fail this RED until cloudflare/pingora#892 is repaired"
    );
}
