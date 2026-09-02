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

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn reserve_loopback() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .expect("loopback port should be reservable")
        .local_addr()
        .expect("reservation should expose an address")
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

fn response_headers(address: SocketAddr, request: &[u8]) -> String {
    let mut downstream = TcpStream::connect(address).expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");

    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = downstream
            .read(&mut buffer)
            .expect("gateway response headers should be readable");
        assert!(read > 0, "gateway closed before response headers completed");
        response.extend_from_slice(&buffer[..read]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&response).into_owned();
        }
    }
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
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "protocol-transition rejection must not poison readiness: {response:?}"
    );
}

fn assert_origin_untouched(origin: &TcpListener) {
    origin
        .set_nonblocking(true)
        .expect("fixture listener should become nonblocking");
    match origin.accept() {
        Err(error) if error.kind() == ErrorKind::WouldBlock => {}
        Ok(_) => panic!("uncharacterized protocol transition must not contact an origin"),
        Err(error) => panic!("unexpected origin accept failure: {error}"),
    }
}

#[test]
fn generic_binary_rejects_websocket_upgrade_before_origin_contact() {
    let origin = TcpListener::bind("127.0.0.1:0").expect("origin fixture should bind");
    let origin_address = origin.local_addr().expect("origin address should exist");
    let listener = reserve_loopback();
    let metrics_listener = reserve_loopback();
    let config = write_generic_config(listener, metrics_listener, origin_address);
    let _process = start_gateway(
        env!("CARGO_BIN_EXE_cwl-pingora-gateway"),
        &config,
        listener,
        metrics_listener,
    );

    let response = response_headers(listener, &websocket_upgrade_request("/socket"));
    assert!(
        response.starts_with("HTTP/1.1 501"),
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
    let listener = reserve_loopback();
    let metrics_listener = reserve_loopback();
    let config = write_migration_config(
        listener,
        metrics_listener,
        backend_address,
        frontend_address,
    );
    let _process = start_gateway(
        env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"),
        &config,
        listener,
        metrics_listener,
    );

    let response = response_headers(listener, &websocket_upgrade_request("/api/socket"));
    assert!(
        response.starts_with("HTTP/1.1 501"),
        "pg-erd candidate must reject Upgrade before route selection/contact: {response:?}"
    );
    assert_origin_untouched(&backend);
    assert_origin_untouched(&frontend);
    assert_ready(listener);
}
