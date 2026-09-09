//! Causal process-level acceptance for Pingora's one-shot HTTP/1 shutdown notification.
//!
//! A separate sentinel connection proves that cleanup has already interrupted a waiter that existed
//! before SIGTERM. Only then is the held response on the subject connection released, forcing its
//! next keep-alive waiter to be created after the one-shot notification.

#![cfg(unix)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::runtime_policy::{V1_GRACE_PERIOD_SECONDS, V1_TERMINATION_BUDGET_SECONDS};
use tempfile::NamedTempFile;

const MAX_FIXTURE_EVIDENCE_BYTES: usize = 64 * 1024;
const SENTINEL_PARK_SETTLE: Duration = Duration::from_millis(250);
const PRE_FALLBACK_CLOSE_WINDOW: Duration = Duration::from_secs(1);

/// Owns the child process so failed assertions cannot leak a gateway into later integration tests.
struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    /// Forces bounded cleanup when the normal SIGTERM path has not yet reaped the process.
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Reserves two distinct loopback authorities while both sockets remain simultaneously bound.
fn reserve_distinct_loopback_addresses() -> (SocketAddr, SocketAddr) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be reservable");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    let addresses = (
        traffic
            .local_addr()
            .expect("traffic reservation should expose an address"),
        metrics
            .local_addr()
            .expect("metrics reservation should expose an address"),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

/// Writes generic-v1 config whose origin read budget stays longer than the shutdown grace window.
fn write_generic_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000"
    )
    .expect("generic config should be written");
    file
}

/// Writes pg-erd config with explicit backend/frontend transport authority and the same long read budget.
fn write_pg_erd_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000"
    )
    .expect("pg-erd config should be written");
    file
}

/// Waits for the public traffic listener without allowing an early process exit to masquerade as readiness.
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

/// Waits until SIGTERM has stopped new public accepts while requiring the draining process to remain alive.
fn wait_until_listener_stops_accepting(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before graceful drain completed: {status}");
        }
        match TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            Err(_) => return,
            Ok(stream) => drop(stream),
        }
        assert!(
            Instant::now() < deadline,
            "gateway listener stayed open after SIGTERM"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

/// Reads one finite HTTP/1 header block and rejects unbounded fixture evidence.
fn read_headers(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("fixture read timeout should be configurable");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("HTTP headers should arrive before their fixture timeout");
        assert!(read > 0, "connection closed before HTTP headers completed");
        bytes.extend_from_slice(&buffer[..read]);
        assert!(
            bytes.len() <= MAX_FIXTURE_EVIDENCE_BYTES,
            "HTTP header evidence exceeded 64 KiB"
        );
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return bytes;
        }
    }
}

/// Reads exactly far enough to prove the held admitted body completed without waiting for connection EOF.
fn read_response_through_body(stream: &mut TcpStream, expected_body: &[u8]) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("downstream response timeout should be configurable");
    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("downstream response should arrive before its fixture timeout");
        assert!(
            read > 0,
            "gateway closed before completing the admitted response"
        );
        response.extend_from_slice(&buffer[..read]);
        assert!(
            response.len() <= MAX_FIXTURE_EVIDENCE_BYTES,
            "downstream response evidence exceeded 64 KiB"
        );
        let Some(header_end) = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
        else {
            continue;
        };
        if response.len() < header_end + expected_body.len() {
            continue;
        }
        assert_eq!(
            &response[header_end..header_end + expected_body.len()],
            expected_body,
            "gateway should forward the admitted response body exactly"
        );
        return response;
    }
}

/// Requires EOF inside the pre-runtime-fallback evidence window and rejects timeout-based false GREEN.
fn require_prompt_eof(stream: &mut TcpStream, context: &str) {
    stream
        .set_read_timeout(Some(PRE_FALLBACK_CLOSE_WINDOW))
        .expect("shutdown evidence timeout should be configurable");
    let mut probe = [0_u8; 1];
    match stream.read(&mut probe) {
        Ok(0) => {}
        Ok(read) => panic!("{context} received {read} unexpected byte(s) instead of EOF"),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            panic!("{context} survived until the one-second shutdown evidence bound")
        }
        Err(error) => panic!("{context} returned an unexpected shutdown error: {error}"),
    }
}

/// Waits against one SIGTERM-relative deadline so downstream work cannot reset termination allowance.
fn wait_for_exit(process: &mut Child, deadline: Instant) -> std::process::ExitStatus {
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not terminate before the external hard-kill budget"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

/// Runs the causal one-shot-notification contract for one compiled composition root.
fn prove_waiter_created_after_cleanup_observes_shutdown(
    binary: &str,
    config: &NamedTempFile,
    gateway_address: SocketAddr,
    origin_listener: TcpListener,
    expected_origin_prefix: &'static [u8],
    held_request: &'static [u8],
) {
    let (request_seen_tx, request_seen_rx) = mpsc::channel();
    let (release_response_tx, release_response_rx) = mpsc::channel();
    let origin = thread::spawn(move || {
        let (mut stream, _) = origin_listener
            .accept()
            .expect("held request should reach its characterized origin");
        let request = read_headers(&mut stream);
        assert!(
            request.starts_with(expected_origin_prefix),
            "held request reached the wrong origin contract: {:?}",
            String::from_utf8_lossy(&request)
        );
        request_seen_tx
            .send(())
            .expect("test should observe origin admission");
        release_response_rx
            .recv_timeout(Duration::from_secs(V1_GRACE_PERIOD_SECONDS))
            .expect("test should release the held response during the grace period");
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: keep-alive\r\n\r\ndrained",
            )
            .expect("held origin response should be writable");
    });

    let child = Command::new(binary)
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled gateway binary should start");
    let mut process = GatewayProcess(child);
    wait_until_listening(gateway_address, &mut process.0);

    let mut subject = TcpStream::connect(gateway_address)
        .expect("gateway should accept the held subject request");
    subject
        .write_all(held_request)
        .expect("held subject request should be writable");
    request_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("held request should reach origin before SIGTERM");

    let mut sentinel =
        TcpStream::connect(gateway_address).expect("gateway should accept the shutdown sentinel");
    sentinel
        .write_all(b"GET /livez HTTP/1.1\r\nHost: gateway.test\r\nConnection: keep-alive\r\n\r\n")
        .expect("sentinel health request should be writable");
    let sentinel_response = read_headers(&mut sentinel);
    assert!(
        sentinel_response.starts_with(b"HTTP/1.1 200 "),
        "sentinel must establish a healthy reusable connection: {:?}",
        String::from_utf8_lossy(&sentinel_response)
    );
    sentinel
        .write_all(b"G")
        .expect("sentinel partial next request should be writable");
    thread::sleep(SENTINEL_PARK_SETTLE);

    let signal_sent_at = Instant::now();
    let termination_deadline = signal_sent_at + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.0.id().to_string()])
        .status()
        .expect("system kill command should send SIGTERM");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");

    wait_until_listener_stops_accepting(gateway_address, &mut process.0);
    require_prompt_eof(
        &mut sentinel,
        "preexisting sentinel waiter did not prove cleanup notification",
    );
    assert!(
        signal_sent_at.elapsed() + PRE_FALLBACK_CLOSE_WINDOW
            < Duration::from_secs(V1_GRACE_PERIOD_SECONDS),
        "fixture must release the subject before production runtime fallback can begin"
    );

    release_response_tx
        .send(())
        .expect("held origin response should be released after the notification barrier");
    let response = read_response_through_body(&mut subject, b"drained");
    assert!(
        response.starts_with(b"HTTP/1.1 200 "),
        "admitted subject must finish before its post-notification park: {:?}",
        String::from_utf8_lossy(&response)
    );

    require_prompt_eof(
        &mut subject,
        "post-notification keep-alive waiter did not observe shutdown",
    );
    assert!(
        signal_sent_at.elapsed() < Duration::from_secs(V1_GRACE_PERIOD_SECONDS),
        "post-notification keep-alive must close before runtime fallback begins"
    );

    origin.join().expect("origin fixture should complete");
    let exit_status = wait_for_exit(&mut process.0, termination_deadline);
    assert!(
        exit_status.success(),
        "SIGTERM graceful shutdown should exit successfully: {exit_status}"
    );
}

/// Proves generic v1 creates the subject's next waiter only after an externally observed cleanup wake.
#[test]
fn generic_post_notification_keepalive_observes_shutdown() {
    let origin_listener =
        TcpListener::bind("127.0.0.1:0").expect("generic origin should bind loopback");
    let origin_address = origin_listener
        .local_addr()
        .expect("generic origin should expose its address");
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_generic_config(gateway_address, metrics_address, origin_address);

    prove_waiter_created_after_cleanup_observes_shutdown(
        env!("CARGO_BIN_EXE_cwl-pingora-gateway"),
        &config,
        gateway_address,
        origin_listener,
        b"GET /held HTTP/1.1\r\n",
        b"GET /held HTTP/1.1\r\nHost: gateway.test\r\nConnection: keep-alive\r\n\r\n",
    );
}

/// Proves the bounded pg-erd composition root obeys the same post-notification waiter invariant.
#[test]
fn pg_erd_post_notification_keepalive_observes_shutdown() {
    let backend_listener =
        TcpListener::bind("127.0.0.1:0").expect("pg-erd backend should bind loopback");
    let backend_address = backend_listener
        .local_addr()
        .expect("pg-erd backend should expose its address");
    let frontend_listener =
        TcpListener::bind("127.0.0.1:0").expect("pg-erd frontend should bind loopback");
    let frontend_address = frontend_listener
        .local_addr()
        .expect("pg-erd frontend should expose its address");
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_pg_erd_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );

    prove_waiter_created_after_cleanup_observes_shutdown(
        env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"),
        &config,
        gateway_address,
        backend_listener,
        b"GET /api/held HTTP/1.1\r\n",
        b"GET /api/held HTTP/1.1\r\nHost: app.example:8080\r\nConnection: keep-alive\r\n\r\n",
    );

    drop(frontend_listener);
}
