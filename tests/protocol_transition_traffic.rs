//! Real-listener acceptance for the fail-closed HTTP/1 protocol-transition boundary.
//!
//! WebSocket/Upgrade is intentionally outside the current generic and pg-erd contracts. These
//! tests prove an Upgrade attempt is rejected before either composition root contacts an origin,
//! while gateway-local readiness remains available.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const MAX_RESPONSE_HEADER_BYTES: usize = 64 * 1024;
const ORIGIN_CONTACT_OBSERVATION_WINDOW: Duration = Duration::from_millis(500);

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Holds both ephemeral gateway listeners at once so the OS cannot reuse one reservation for both.
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

fn write_generic_config(
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

fn write_migration_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
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
            panic!("gateway exited before accepting traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(Instant::now() < deadline, "gateway did not start within 10s");
        thread::sleep(Duration::from_millis(25));
    }
}

fn start_gateway(
    binary: &str,
    config: &NamedTempFile,
    listener: SocketAddr,
    metrics_listener: SocketAddr,
) -> GatewayProcess {
    let mut child = Command::new(binary)
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled gateway binary should start");
    wait_until_listening(listener, &mut child);
    wait_until_listening(metrics_listener, &mut child);
    GatewayProcess(child)
}

/// Reads one response header inside a whole-header deadline and finite byte budget.
fn response_headers(address: SocketAddr, request: &[u8]) -> String {
    let mut downstream = TcpStream::connect(address).expect("gateway should accept traffic");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let now = Instant::now();
        assert!(now < deadline, "gateway response header exceeded 5s deadline");
        downstream
            .set_read_timeout(Some(deadline.saturating_duration_since(now)))
            .expect("downstream timeout should be configurable");
        let read = downstream
            .read(&mut buffer)
            .expect("gateway response headers should be readable");
        assert!(read > 0, "gateway closed before response headers completed");
        response.extend_from_slice(&buffer[..read]);
        assert!(
            response.len() <= MAX_RESPONSE_HEADER_BYTES,
            "gateway response header exceeded {MAX_RESPONSE_HEADER_BYTES} bytes"
        );
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&response).into_owned();
        }
    }
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

fn websocket_upgrade_request(path: &str) -> Vec<u8> {
    format!(
        "GET {path} HTTP/1.1\r\nHost: app.example:8080\r\nConnection: keep-alive, UpGrAdE\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
    )
    .into_bytes()
}

fn assert_ready(address: SocketAddr) {
    let response = response_headers(
        address,
        b"GET /readyz HTTP/1.1\r\nHost: gateway.local\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(
        http1_status_code(&response),
        Some(200),
        "protocol-transition rejection must not poison readiness: {response:?}"
    );
}

/// Observes the origin for a fixed post-response window so delayed connection attempts cannot pass.
fn assert_origin_untouched(origin: &TcpListener) {
    origin
        .set_nonblocking(true)
        .expect("fixture listener should become nonblocking");
    let deadline = Instant::now() + ORIGIN_CONTACT_OBSERVATION_WINDOW;
    loop {
        match origin.accept() {
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Ok(_) => panic!("uncharacterized protocol transition must not contact an origin"),
            Err(error) => panic!("unexpected origin accept failure: {error}"),
        }
    }
}

#[test]
fn status_parser_rejects_numeric_prefix_and_protocol_case_lookalikes() {
    assert_eq!(http1_status_code("HTTP/1.1 501 Not Implemented\r\n"), Some(501));
    assert_eq!(http1_status_code("HTTP/1.1 5010 Not Implemented\r\n"), None);
    assert_eq!(http1_status_code("http/1.1 501 Not Implemented\r\n"), None);
}

#[test]
fn generic_binary_rejects_websocket_upgrade_before_origin_contact() {
    let origin = TcpListener::bind("127.0.0.1:0").expect("origin fixture should bind");
    let origin_address = origin.local_addr().expect("origin address should exist");
    let (listener_reservation, metrics_reservation) = reserve_gateway_listeners();
    let listener = listener_reservation
        .local_addr()
        .expect("traffic reservation should expose an address");
    let metrics_listener = metrics_reservation
        .local_addr()
        .expect("metrics reservation should expose an address");
    let config = write_generic_config(listener, metrics_listener, origin_address);
    drop(listener_reservation);
    drop(metrics_reservation);
    let _process = start_gateway(
        env!("CARGO_BIN_EXE_cwl-pingora-gateway"),
        &config,
        listener,
        metrics_listener,
    );

    let response = response_headers(listener, &websocket_upgrade_request("/socket"));
    assert_eq!(
        http1_status_code(&response),
        Some(501),
        "generic v1 must fail closed instead of inheriting uncharacterized Upgrade behavior: {response:?}"
    );
    assert_origin_untouched(&origin);
    assert_ready(listener);
}

#[test]
fn pg_erd_binary_rejects_websocket_upgrade_before_route_origin_contact() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");
    let (listener_reservation, metrics_reservation) = reserve_gateway_listeners();
    let listener = listener_reservation
        .local_addr()
        .expect("traffic reservation should expose an address");
    let metrics_listener = metrics_reservation
        .local_addr()
        .expect("metrics reservation should expose an address");
    let config = write_migration_config(
        listener,
        metrics_listener,
        backend_address,
        frontend_address,
    );
    drop(listener_reservation);
    drop(metrics_reservation);
    let _process = start_gateway(
        env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"),
        &config,
        listener,
        metrics_listener,
    );

    let response = response_headers(listener, &websocket_upgrade_request("/api/socket"));
    assert_eq!(
        http1_status_code(&response),
        Some(501),
        "pg-erd candidate must reject Upgrade before route selection/contact: {response:?}"
    );
    assert_origin_untouched(&backend);
    assert_origin_untouched(&frontend);
    assert_ready(listener);
}
