//! Compiled-process regression for dependency diagnostic logging at broad operator verbosity.
//!
//! A production operator may request broad diagnostics through `RUST_LOG`. The gateway's
//! payload-minimization invariant must still prevent request URI/header secrets from entering
//! process stderr through Pingora dependency diagnostics. The origin must receive the sentinels so
//! the test cannot pass by rejecting or stripping the request before proxy delivery.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const REDACTED_PINGORA_DIAGNOSTIC: &str =
    "Pingora diagnostic message redacted by gateway payload-minimization policy";

struct GatewayProcess {
    child: Option<Child>,
    stderr: NamedTempFile,
}

impl GatewayProcess {
    fn stderr_occurrences(&self, needle: &str) -> usize {
        fs::read_to_string(self.stderr.path())
            .expect("gateway stderr capture should remain readable")
            .matches(needle)
            .count()
    }

    fn wait_until_stderr_line_ends_with(&mut self, suffix: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let captured = fs::read_to_string(self.stderr.path())
                .expect("gateway stderr capture should remain readable");
            if captured.lines().any(|line| line.ends_with(suffix)) {
                return;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .expect("gateway child should still be owned")
                .try_wait()
                .expect("gateway process state should be readable")
            {
                panic!(
                    "gateway exited before expected log suffix {suffix:?}: {status}; stderr={captured:?}"
                );
            }
            assert!(
                Instant::now() < deadline,
                "gateway did not emit expected log suffix {suffix:?} within 10s; stderr={captured:?}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_until_stderr_occurrences_exceed(&mut self, needle: &str, baseline: usize) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let captured = fs::read_to_string(self.stderr.path())
                .expect("gateway stderr capture should remain readable");
            if captured.matches(needle).count() > baseline {
                return;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .expect("gateway child should still be owned")
                .try_wait()
                .expect("gateway process state should be readable")
            {
                panic!(
                    "gateway exited before {needle:?} increased beyond {baseline}: {status}; stderr={captured:?}"
                );
            }
            assert!(
                Instant::now() < deadline,
                "gateway did not emit a new {needle:?} after the secret-bearing request within 10s; stderr={captured:?}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn capture_stderr(mut self) -> String {
        let mut child = self
            .child
            .take()
            .expect("gateway child should still be owned");
        child
            .kill()
            .expect("gateway should be terminable after traffic");
        child
            .wait()
            .expect("gateway should terminate after traffic capture");
        fs::read_to_string(self.stderr.path()).expect("gateway log output should be UTF-8")
    }
}

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Holds both ephemeral gateway listeners simultaneously so their addresses cannot be reused.
fn reserve_gateway_listeners() -> (TcpListener, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("traffic port should be reservable");
    let metrics_listener =
        TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    assert_ne!(
        listener.local_addr().expect("traffic address should exist"),
        metrics_listener
            .local_addr()
            .expect("metrics address should exist"),
        "traffic and metrics reservations must remain distinct"
    );
    (listener, metrics_listener)
}

fn write_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: origin\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
    )
    .expect("generic config should be written");
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

/// Accepts the characterized origin connection inside a finite fixture deadline.
fn accept_origin(origin: &TcpListener) -> TcpStream {
    origin
        .set_nonblocking(true)
        .expect("origin listener should become nonblocking");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match origin.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                assert!(
                    Instant::now() < deadline,
                    "proxied request did not reach origin within 5s"
                );
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("unexpected origin accept failure: {error}"),
        }
    }
}

/// Reads one origin request header inside one whole-header deadline and finite byte budget.
fn read_request_headers(stream: &mut TcpStream) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let now = Instant::now();
        assert!(now < deadline, "origin request header exceeded 5s deadline");
        stream
            .set_read_timeout(Some(deadline.saturating_duration_since(now)))
            .expect("origin read timeout should be configurable");
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable");
        assert!(
            read > 0,
            "gateway closed origin request before headers completed"
        );
        bytes.extend_from_slice(&buffer[..read]);
        assert!(
            bytes.len() <= MAX_REQUEST_HEADER_BYTES,
            "origin request header exceeded {MAX_REQUEST_HEADER_BYTES} bytes"
        );
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

/// Returns values only for semantically exact HTTP field names, ignoring field-name case and OWS.
fn header_values<'a>(request: &'a str, expected_name: &str) -> Vec<&'a str> {
    request
        .split("\r\n")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .filter_map(|(name, value)| {
            name.eq_ignore_ascii_case(expected_name)
                .then_some(value.trim())
        })
        .collect()
}

/// Parses only an exact HTTP/1.1 three-digit status token from the response status line.
fn http1_status_code(response: &str) -> Option<u16> {
    let status_line = response.split("\r\n").next()?;
    let mut fields = status_line.split_ascii_whitespace();
    if fields.next()? != "HTTP/1.1" {
        return None;
    }
    let status = fields.next()?;
    if status.len() != 3 || !status.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    status.parse().ok()
}

#[test]
fn exact_header_lookup_rejects_lookalike_fields() {
    let request = "GET / HTTP/1.1\r\nX-Host: attacker.example\r\nhOsT: expected.example\r\n\r\n";
    assert_eq!(header_values(request, "Host"), vec!["expected.example"]);
}

#[test]
fn status_parser_rejects_numeric_prefix_and_protocol_case_lookalikes() {
    assert_eq!(http1_status_code("HTTP/1.1 200 OK\r\n"), Some(200));
    assert_eq!(http1_status_code("HTTP/1.1 2000 OK\r\n"), None);
    assert_eq!(http1_status_code("http/1.1 200 OK\r\n"), None);
}

#[test]
fn broad_runtime_diagnostics_do_not_log_request_secrets() {
    let origin = TcpListener::bind("127.0.0.1:0").expect("origin fixture should bind");
    let origin_address = origin.local_addr().expect("origin address should exist");
    let origin_thread = thread::spawn(move || {
        let mut stream = accept_origin(&origin);
        let request = read_request_headers(&mut stream);
        assert_eq!(
            request.split("\r\n").next(),
            Some("GET /diagnostic-secret?token=query-secret HTTP/1.1")
        );
        assert_eq!(header_values(&request, "Host"), vec!["host-secret.example"]);
        assert_eq!(
            header_values(&request, "Authorization"),
            vec!["Bearer authorization-secret"]
        );
        assert_eq!(
            header_values(&request, "Cookie"),
            vec!["session=cookie-secret"]
        );
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("origin response should be writable");
    });

    let (listener_reservation, metrics_reservation) = reserve_gateway_listeners();
    let listener = listener_reservation
        .local_addr()
        .expect("traffic reservation should expose an address");
    let metrics_listener = metrics_reservation
        .local_addr()
        .expect("metrics reservation should expose an address");
    let config = write_config(listener, metrics_listener, origin_address);
    drop(listener_reservation);
    drop(metrics_reservation);
    let stderr = NamedTempFile::new().expect("gateway stderr capture should be writable");
    let stderr_writer = stderr
        .reopen()
        .expect("gateway stderr capture should be reopenable for child");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "trace")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_writer))
        .spawn()
        .expect("compiled gateway binary should start");
    wait_until_listening(listener, &mut child);
    wait_until_listening(metrics_listener, &mut child);
    let mut process = GatewayProcess {
        child: Some(child),
        stderr,
    };
    let redacted_before_request = process.stderr_occurrences(REDACTED_PINGORA_DIAGNOSTIC);

    let mut downstream = TcpStream::connect(listener).expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    downstream
        .write_all(
            b"GET /diagnostic-secret?token=query-secret HTTP/1.1\r\nHost: host-secret.example\r\nAuthorization: Bearer authorization-secret\r\nCookie: session=cookie-secret\r\nConnection: close\r\n\r\n",
        )
        .expect("request should be writable");
    let mut response = String::new();
    downstream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    assert_eq!(
        http1_status_code(&response),
        Some(200),
        "request should proxy successfully: {response:?}"
    );
    origin_thread
        .join()
        .expect("origin diagnostic fixture should complete");

    process.wait_until_stderr_line_ends_with(
        "gateway_request status=200 outcome=ok request_body_bytes=0",
    );
    process
        .wait_until_stderr_occurrences_exceed(REDACTED_PINGORA_DIAGNOSTIC, redacted_before_request);
    let captured = process.capture_stderr();
    for forbidden in [
        "/diagnostic-secret",
        "query-secret",
        "host-secret.example",
        "authorization-secret",
        "cookie-secret",
    ] {
        assert!(
            !captured.contains(forbidden),
            "runtime diagnostics leaked request-sensitive material {forbidden:?}: {captured:?}"
        );
    }
}
