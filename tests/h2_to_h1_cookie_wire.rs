//! Real-wire supplier RED for RFC 9113 §8.2.3 H2-to-H1 Cookie normalization.
//!
//! This test deliberately exercises a test-only TLS/H2 listener built from the same `GatewayProxy`
//! and pinned Pingora supplier as production, while keeping the characterized upstream on HTTP/1.1.
//! It is expected to fail on Pingora `09696b51bc59315353d96686355861604d0bb48c` because the
//! supplier currently forwards multiple HTTP/2 Cookie fields as multiple HTTP/1.1 header lines.
//! The product listener contract is not widened by this fixture. The fixture is Linux-only because
//! it uses Pingora's SCM_RIGHTS listener-transfer path to preserve one kernel-bound ephemeral socket
//! from parent reservation through child ownership without a port-selection race.

#![cfg(target_os = "linux")]

use std::env;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
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
use pingora::server::configuration::Opt;
use pingora::server::{Fds, RunArgs};
use tempfile::{tempdir, NamedTempFile};

const HELPER_MODE_ENV: &str = "CWL_H2_H1_COOKIE_HELPER";
const HELPER_LISTENER_ENV: &str = "CWL_H2_H1_COOKIE_LISTENER";
const HELPER_METRICS_ENV: &str = "CWL_H2_H1_COOKIE_METRICS";
const HELPER_UPSTREAM_ENV: &str = "CWL_H2_H1_COOKIE_UPSTREAM";
const HELPER_CERT_ENV: &str = "CWL_H2_H1_COOKIE_CERT";
const HELPER_KEY_ENV: &str = "CWL_H2_H1_COOKIE_KEY";
const HELPER_UPGRADE_SOCK_ENV: &str = "CWL_H2_H1_COOKIE_UPGRADE_SOCK";

/// Child process guard that terminates the test-only gateway even after assertion failure.
struct HelperProcess(Child);

impl Drop for HelperProcess {
    /// Prevents a failed RED assertion from leaving a listening gateway process behind.
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Ephemeral certificate material whose temporary directory owns both files for the test lifetime.
struct LocalCertificate {
    _directory: tempfile::TempDir,
    cert: PathBuf,
    key: PathBuf,
}

/// Runs one required OpenSSL command and fails the fixture before protocol claims are made.
fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

/// Issues a one-day `h2.test` certificate used only by the loopback TLS listener.
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

/// Binds and retains one ephemeral loopback listener so its selected port cannot be stolen.
fn reserve_loopback_listener() -> TcpListener {
    TcpListener::bind("127.0.0.1:0").expect("loopback port should be available")
}

/// Waits until the transferred listener remains reachable under child ownership or the child exits.
fn wait_until_listening(address: SocketAddr, process: &mut HelperProcess) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .0
            .try_wait()
            .expect("helper process state should be readable")
        {
            panic!("H2 helper exited before accepting traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "H2 helper did not start within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

/// Accepts the gateway's H1 origin connection within a finite fixture-owned deadline.
fn accept_h1_origin(listener: &TcpListener, timeout: Duration) -> io::Result<TcpStream> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false)?;
                return Ok(stream);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        ErrorKind::TimedOut,
                        format!("gateway did not connect to H1 origin within {timeout:?}"),
                    ));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(error),
        }
    }
}

/// Proves the raw-origin accept path returns a timeout instead of blocking a detached thread.
#[test]
fn h1_origin_accept_is_bounded_without_gateway_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("H1 origin fixture should bind");
    let started = Instant::now();
    let error = accept_h1_origin(&listener, Duration::from_millis(50))
        .expect_err("origin accept should fail when no gateway connection arrives");
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "origin accept timeout must remain bounded"
    );
}

/// Captures one complete HTTP/1 request-header block from the raw origin connection.
fn read_request_headers(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("upstream read timeout should be configurable");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2048];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream
            .read(&mut buffer)
            .expect("upstream request should be readable");
        assert!(
            read > 0,
            "gateway closed before sending an HTTP/1.1 request header"
        );
        request.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(request).expect("fixture request header should be UTF-8")
}

/// Completes the captured request with a minimal close-delimited fixture response.
fn write_ok(stream: &mut TcpStream) {
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        .expect("fixture response should be writable");
}

/// Starts the raw H1 origin and returns the channel carrying its observed request headers.
fn spawn_h1_origin(listener: TcpListener) -> mpsc::Receiver<String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut stream = accept_h1_origin(&listener, Duration::from_secs(30))
            .expect("gateway should connect to H1 origin within the fixture budget");
        let request = read_request_headers(&mut stream);
        write_ok(&mut stream);
        sender
            .send(request)
            .expect("test should still be waiting for origin evidence");
    });
    receiver
}

/// Builds the strict production-shaped v1 contract used by the isolated H2 test composition root.
fn helper_config(listener: SocketAddr, metrics: SocketAddr, upstream: SocketAddr) -> GatewayConfig {
    GatewayConfig::from_yaml(&format!(
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: h1-origin\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000\n"
    ))
    .expect("test-only gateway configuration should be valid")
}

/// Reads one required helper socket authority from the child-process environment.
fn env_socket(name: &str) -> SocketAddr {
    env::var(name)
        .unwrap_or_else(|_| panic!("missing helper environment variable {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("invalid socket address in {name}"))
}

/// Reads one required certificate/key or Unix-socket path from the child-process environment.
fn env_path(name: &str) -> PathBuf {
    PathBuf::from(env::var_os(name).unwrap_or_else(|| panic!("missing helper path {name}")))
}

/// Runs the test-only Pingora TLS/H2 listener in a separate process so `Server::run` may block.
#[test]
#[ignore = "test-only Pingora TLS/H2 server process; invoked by the real-wire parent test"]
fn h2_cookie_proxy_helper() {
    assert_eq!(env::var(HELPER_MODE_ENV).as_deref(), Ok("1"));
    let listener = env_socket(HELPER_LISTENER_ENV);
    let metrics = env_socket(HELPER_METRICS_ENV);
    let upstream = env_socket(HELPER_UPSTREAM_ENV);
    let cert = env_path(HELPER_CERT_ENV);
    let key = env_path(HELPER_KEY_ENV);
    let upgrade_sock = env_path(HELPER_UPGRADE_SOCK_ENV);

    let config = helper_config(listener, metrics, upstream);
    let proxy = GatewayProxy::try_from_config(&config).expect("test-only proxy should activate");
    let mut server_conf = build_server_conf(config.upstream_keepalive_pool_size);
    server_conf.upgrade_sock = upgrade_sock
        .to_str()
        .expect("upgrade socket path should be UTF-8")
        .to_owned();
    let options = Opt {
        upgrade: true,
        ..Opt::default()
    };
    let mut server = Server::new_with_opt_and_conf(Some(options), server_conf);
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

/// Refuses to run the wire characterization when the installed client cannot negotiate HTTP/2.
fn assert_curl_supports_http2() {
    let output = Command::new("curl")
        .arg("--version")
        .output()
        .expect("CI must provide curl for the real-wire protocol fixture");
    assert!(output.status.success(), "curl --version should succeed");
    let version = String::from_utf8(output.stdout).expect("curl version output should be UTF-8");
    assert!(
        version.lines().any(|line| {
            line.starts_with("Features:")
                && line
                    .split_whitespace()
                    .any(|feature| feature == "HTTP2")
        }),
        "curl must expose HTTP2 support for this fixture: {version}"
    );
}

/// Spawns the ignored helper, atomically hands it the reserved FD, then proves the socket survived.
fn spawn_helper(
    listener: TcpListener,
    metrics: SocketAddr,
    upstream: SocketAddr,
    cert: &Path,
    key: &Path,
    upgrade_sock: &Path,
) -> HelperProcess {
    let listener_address = listener
        .local_addr()
        .expect("reserved H2 listener address should resolve");
    // Pingora's Fds API requires a path that is both NixPath and Display; Rust Path intentionally
    // uses a Display adapter instead of implementing Display itself, so keep one exact UTF-8 string
    // for both the child configuration and the SCM_RIGHTS sender.
    let upgrade_sock = upgrade_sock
        .to_str()
        .expect("upgrade socket path should be UTF-8");
    let child = Command::new(env::current_exe().expect("current test executable should resolve"))
        .args([
            "--ignored",
            "--exact",
            "h2_cookie_proxy_helper",
            "--nocapture",
        ])
        .env(HELPER_MODE_ENV, "1")
        .env(HELPER_LISTENER_ENV, listener_address.to_string())
        .env(HELPER_METRICS_ENV, metrics.to_string())
        .env(HELPER_UPSTREAM_ENV, upstream.to_string())
        .env(HELPER_CERT_ENV, cert)
        .env(HELPER_KEY_ENV, key)
        .env(HELPER_UPGRADE_SOCK_ENV, upgrade_sock)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("test-only H2 proxy helper should start");

    // Guard immediately so transfer/bootstrap/readiness failures cannot leak or orphan the helper.
    let mut process = HelperProcess(child);
    let mut inherited = Fds::new();
    inherited.add(listener_address.to_string(), listener.as_raw_fd());
    inherited
        .send_to_sock(upgrade_sock)
        .expect("reserved H2 listener should transfer to the Pingora helper");
    // SCM_RIGHTS has duplicated the already-listening socket into the child. Drop the parent copy so
    // readiness cannot be satisfied by a listener that no child process owns.
    drop(listener);
    wait_until_listening(listener_address, &mut process);
    process
}

/// Sends the two-field Cookie request and returns curl's negotiated HTTP version evidence.
fn traced_curl_h2_request(listener: SocketAddr, cert: &Path, trace: &NamedTempFile) -> String {
    let authority = format!("h2.test:{}", listener.port());
    let resolve = format!("{authority}:127.0.0.1");
    let url = format!("https://{authority}/cookie-wire");
    let output = Command::new("curl")
        .args([
            "--fail-with-body",
            "--silent",
            "--show-error",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
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

/// Extracts each outbound `Cookie` field from curl's `=> Send header` trace blocks.
fn outbound_trace_cookie_values(trace: &str) -> Vec<String> {
    let mut in_send_header = false;
    let mut values = Vec::new();
    for line in trace.lines() {
        if line.starts_with("=> Send header,") {
            in_send_header = true;
            continue;
        }
        if line.starts_with("=> ") || line.starts_with("<= ") || line.starts_with("== Info:") {
            in_send_header = false;
            continue;
        }
        if !in_send_header {
            continue;
        }
        let Some((offset, payload)) = line.split_once(": ") else {
            continue;
        };
        if offset.len() != 4 || !offset.chars().all(|character| character.is_ascii_hexdigit()) {
            continue;
        }
        let Some((name, value)) = payload.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("cookie") {
            values.push(value.trim().to_string());
        }
    }
    values
}

/// Extracts every raw H1 `Cookie` field value without hiding duplicate wire fields.
fn cookie_header_values(raw_request: &str) -> Vec<&str> {
    raw_request
        .split("\r\n")
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("cookie").then_some(value.trim())
        })
        .collect()
}

/// Proves that H2 multiple-Cookie input is coalesced exactly once before the H1 origin boundary.
#[test]
fn h2_multiple_cookie_fields_are_coalesced_before_h1_upstream() {
    assert_curl_supports_http2();
    let certificate = issue_local_certificate();
    let origin = TcpListener::bind("127.0.0.1:0").expect("H1 origin should bind");
    let origin_address = origin
        .local_addr()
        .expect("H1 origin address should resolve");
    let origin_request = spawn_h1_origin(origin);
    let listener_reservation = reserve_loopback_listener();
    let metrics_reservation = reserve_loopback_listener();
    let listener = listener_reservation
        .local_addr()
        .expect("H2 listener reservation should expose its address");
    let metrics = metrics_reservation
        .local_addr()
        .expect("metrics reservation should expose its address");
    assert_ne!(listener, metrics);
    assert_ne!(listener, origin_address);
    assert_ne!(metrics, origin_address);

    let upgrade_directory = tempdir().expect("upgrade socket workspace should be available");
    let upgrade_sock = upgrade_directory.path().join("pingora-upgrade.sock");
    let _helper = spawn_helper(
        listener_reservation,
        metrics,
        origin_address,
        &certificate.cert,
        &certificate.key,
        &upgrade_sock,
    );
    let trace = NamedTempFile::new().expect("curl trace file should be writable");
    let negotiated_version = traced_curl_h2_request(listener, &certificate.cert, &trace);
    assert_eq!(
        negotiated_version.trim(),
        "2",
        "fixture must negotiate HTTP/2"
    );

    let client_trace = fs::read_to_string(trace.path()).expect("curl trace should be readable");
    assert_eq!(
        outbound_trace_cookie_values(&client_trace),
        vec![
            "session_id=abc123".to_string(),
            "preferred_language=en".to_string()
        ],
        "client fixture must originate exactly two distinct outbound Cookie header records before the Pingora boundary: {client_trace}"
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
