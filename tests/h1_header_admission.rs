//! Executable RED for missing operator-controlled downstream HTTP/1 parser admission.
//!
//! The current Pingora supplier has fixed HTTP/1 parser ceilings, but the gateway cannot lower
//! request-header bytes or field count before `ProxyHttp::request_filter()`. These real-socket
//! fixtures use commercial acceptance budgets that remain comfortably below the supplier ceilings
//! and require oversized requests to be rejected before any origin connection. The budgets are
//! test acceptance values only; they are not a hidden v1 Admin Config contract.

#![cfg(unix)]

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const DESIRED_MAX_HEADER_BYTES: usize = 16 * 1024;
const DESIRED_MAX_HEADER_FIELDS: usize = 32;
const ORIGIN_HEADER_BOUND: usize = 128 * 1024;
const ATTACKER_MARKER: &str = "h1-admission-secret-marker";

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(Debug)]
enum DownstreamOutcome {
    Http(Vec<u8>),
    Closed,
    RejectedDuringWrite,
    TimedOut,
}

struct ExerciseResult {
    origin_request: Option<Vec<u8>>,
    downstream: DownstreamOutcome,
    logs: String,
}

/// Drains trace-level gateway stderr while the child is alive so pipe capacity cannot mask RED.
fn drain_gateway_stderr(process: &mut Child) -> thread::JoinHandle<String> {
    let mut stderr = process
        .stderr
        .take()
        .expect("gateway stderr should remain captured");
    thread::spawn(move || {
        let mut logs = String::new();
        stderr
            .read_to_string(&mut logs)
            .expect("gateway logs should be readable");
        logs
    })
}

/// Reserves distinct gateway listeners before startup so the fixture cannot alias its own authority.
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

/// Builds the unchanged generic-v1 configuration; parser budgets are deliberately not fabricated.
fn write_gateway_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 5000"
    )
    .expect("gateway config should be written");
    file
}

/// Waits for a compiled gateway listener while failing fast if the child exits during startup.
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

/// Sends a bounded health request to prove parser rejection does not take the process out of service.
fn assert_ready(address: SocketAddr) {
    let mut stream = TcpStream::connect(address).expect("gateway should accept readiness traffic");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("readiness timeout should be configurable");
    stream
        .write_all(b"GET /readyz HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
        .expect("readiness request should be writable");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("readiness response should be readable");
    assert!(
        response.starts_with(b"HTTP/1.1 200 "),
        "gateway readiness should remain healthy: {:?}",
        String::from_utf8_lossy(&response)
    );
}

/// Reads one origin request header with a hard byte bound so hostile traffic cannot grow the fixture.
fn read_origin_request_header(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("origin read timeout should be configurable");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return request;
        }
        assert!(
            request.len() < ORIGIN_HEADER_BOUND,
            "origin request header exceeded fixture bound"
        );
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable");
        assert!(read > 0, "origin connection closed before headers completed");
        request.extend_from_slice(&buffer[..read]);
    }
}

/// Observes whether a complete oversized request crossed the parser/application boundary into origin.
fn observe_origin(listener: TcpListener) -> thread::JoinHandle<Option<Vec<u8>>> {
    listener
        .set_nonblocking(true)
        .expect("origin observation should be nonblocking");
    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match listener.accept() {
                Ok((mut stream, _peer)) => {
                    let request = read_origin_request_header(&mut stream);
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                        )
                        .expect("origin response should be writable");
                    return Some(request);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return None;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("unexpected origin accept error: {error}"),
            }
        }
    })
}

/// Captures an HTTP status, an early transport rejection, or a bounded no-response outcome.
fn send_candidate_request(address: SocketAddr, request: &[u8]) -> DownstreamOutcome {
    let mut stream = TcpStream::connect(address).expect("gateway should accept downstream traffic");
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .expect("downstream write timeout should be configurable");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("downstream read timeout should be configurable");

    if let Err(error) = stream.write_all(request) {
        if matches!(
            error.kind(),
            ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::BrokenPipe
                | ErrorKind::NotConnected
        ) {
            return DownstreamOutcome::RejectedDuringWrite;
        }
        panic!("unexpected downstream write error: {error}");
    }

    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                return if response.is_empty() {
                    DownstreamOutcome::Closed
                } else {
                    DownstreamOutcome::Http(response)
                };
            }
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if response.windows(2).any(|window| window == b"\r\n") {
                    return DownstreamOutcome::Http(response);
                }
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return if response.is_empty() {
                    DownstreamOutcome::TimedOut
                } else {
                    DownstreamOutcome::Http(response)
                };
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                ) =>
            {
                return if response.is_empty() {
                    DownstreamOutcome::Closed
                } else {
                    DownstreamOutcome::Http(response)
                };
            }
            Err(error) => panic!("unexpected downstream read error: {error}"),
        }
    }
}

/// Parses only an exact HTTP/1.1 three-digit status token; lookalike protocol/status text is rejected.
fn parse_http11_status(response: &[u8]) -> u16 {
    let line_end = response
        .windows(2)
        .position(|window| window == b"\r\n")
        .expect("response should contain a complete HTTP status line");
    let line = std::str::from_utf8(&response[..line_end]).expect("status line should be UTF-8");
    let mut parts = line.split(' ');
    assert_eq!(parts.next(), Some("HTTP/1.1"), "unexpected protocol token");
    let status = parts.next().expect("status code should be present");
    assert_eq!(status.len(), 3, "status code must be exactly three digits");
    assert!(
        status.bytes().all(|byte| byte.is_ascii_digit()),
        "status code must be decimal"
    );
    status.parse().expect("three decimal digits should parse")
}

/// Stops the child through the same SIGTERM path used by the repository's graceful-drain fixtures.
fn terminate_gateway(process: &mut Child) {
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.id().to_string()])
        .status()
        .expect("system kill command should send SIGTERM");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");
    let exit_status = process
        .wait()
        .expect("gracefully terminated gateway should be reapable");
    assert!(
        exit_status.success(),
        "SIGTERM graceful shutdown should exit successfully: {exit_status}"
    );
}

/// Runs one complete oversized request and records origin reachability without changing gateway config.
fn exercise_request(request: Vec<u8>) -> ExerciseResult {
    let origin_listener =
        TcpListener::bind("127.0.0.1:0").expect("fixture origin should bind loopback");
    let origin_address = origin_listener
        .local_addr()
        .expect("fixture origin should expose its address");
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_gateway_config(gateway_address, metrics_address, origin_address);

    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "trace")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled gateway binary should start");
    let mut process = GatewayProcess(child);
    let log_drain = drain_gateway_stderr(&mut process.0);
    wait_until_listening(gateway_address, &mut process.0);
    wait_until_listening(metrics_address, &mut process.0);
    assert_ready(gateway_address);

    let origin = observe_origin(origin_listener);
    let downstream = send_candidate_request(gateway_address, &request);
    assert_ready(gateway_address);

    terminate_gateway(&mut process.0);
    let logs = log_drain
        .join()
        .expect("gateway stderr drain should complete");
    let origin_request = origin.join().expect("origin observer should complete");

    ExerciseResult {
        origin_request,
        downstream,
        logs,
    }
}

/// Requires parser-phase rejection before origin while permitting transport close or a 4xx/5xx status.
fn assert_pre_callback_rejection(case: &str, result: ExerciseResult) {
    assert!(
        !result.logs.contains(ATTACKER_MARKER),
        "attacker-controlled request-header content must not enter process logs"
    );

    if let Some(origin_request) = result.origin_request {
        let status = match &result.downstream {
            DownstreamOutcome::Http(response) => Some(parse_http11_status(response)),
            _ => None,
        };
        panic!(
            "{case} crossed the HTTP/1 parser/application boundary and reached origin ({} bytes, downstream status {status:?}); a callback-only rejection cannot satisfy pre-allocation admission",
            origin_request.len()
        );
    }

    match result.downstream {
        DownstreamOutcome::RejectedDuringWrite | DownstreamOutcome::Closed => {}
        DownstreamOutcome::Http(response) => {
            let status = parse_http11_status(&response);
            assert!(
                (400..600).contains(&status),
                "{case} must fail closed before origin, not return HTTP {status}"
            );
        }
        DownstreamOutcome::TimedOut => {
            panic!("{case} reached neither origin nor bounded parser rejection")
        }
    }
}

/// Builds a request that exceeds only the desired byte budget while staying far below field ceilings.
fn one_large_field_request() -> Vec<u8> {
    let value = format!(
        "{ATTACKER_MARKER}{}",
        "a".repeat(DESIRED_MAX_HEADER_BYTES + 4096)
    );
    let request = format!(
        "GET /header-bytes HTTP/1.1\r\nHost: gateway.test\r\nX-Oversized: {value}\r\nConnection: close\r\n\r\n"
    );
    assert!(request.len() > DESIRED_MAX_HEADER_BYTES);
    assert!(request.len() < ORIGIN_HEADER_BOUND);
    request.into_bytes()
}

/// Builds a low-byte request that exceeds only the desired field-count budget.
fn many_small_fields_request() -> Vec<u8> {
    let mut request = String::from("GET /header-count HTTP/1.1\r\nHost: gateway.test\r\n");
    for index in 0..=DESIRED_MAX_HEADER_FIELDS {
        if index == 0 {
            request.push_str(&format!("X-Field-{index:03}: {ATTACKER_MARKER}\r\n"));
        } else {
            request.push_str(&format!("X-Field-{index:03}: v\r\n"));
        }
    }
    request.push_str("Connection: close\r\n\r\n");
    assert!(request.len() < DESIRED_MAX_HEADER_BYTES);
    request.into_bytes()
}

#[test]
fn one_large_field_above_commercial_budget_is_rejected_before_origin() {
    assert_pre_callback_rejection(
        "one-large-field request",
        exercise_request(one_large_field_request()),
    );
}

#[test]
fn many_small_fields_above_commercial_count_are_rejected_before_origin() {
    assert_pre_callback_rejection(
        "many-small-fields request",
        exercise_request(many_small_fields_request()),
    );
}
