//! Real-listener runtime-isolation acceptance for the dedicated pg-erd migration binary.
//!
//! These tests exercise the compiled Pingora process over loopback traffic. They keep product
//! authentication and business behavior outside the gateway while proving streamed body rejection,
//! process-health availability under saturation, admission backpressure, and capacity recovery.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
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

fn write_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
    max_in_flight_requests: usize,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: {max_in_flight_requests}\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000"
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

fn start_gateway(config: &NamedTempFile, gateway_address: SocketAddr, metrics_address: SocketAddr) -> GatewayProcess {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    wait_until_listening(gateway_address, &mut child);
    wait_until_listening(metrics_address, &mut child);
    GatewayProcess(child)
}

fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let mut downstream = TcpStream::connect(address).expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");
    let mut response = String::new();
    downstream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    response
}

fn get(address: SocketAddr, path: &str) -> String {
    raw_request(
        address,
        format!("GET {path} HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

fn read_request_headers(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("origin request should be readable");
        assert!(read > 0, "gateway closed origin request before headers completed");
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

#[test]
fn compiled_pg_erd_rejects_streamed_body_overflow_and_keeps_readiness_available() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");
    let gateway_address = reserve_loopback();
    let metrics_address = reserve_loopback();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
        8,
    );
    let _process = start_gateway(&config, gateway_address, metrics_address);

    let oversized = raw_request(
        gateway_address,
        b"POST /api/streamed HTTP/1.1\r\nHost: app.example:8080\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n4\r\nping\r\n5\r\n12345\r\n0\r\n\r\n",
    );
    assert!(
        oversized.starts_with("HTTP/1.1 413"),
        "streamed body above max_request_body_bytes must fail closed: {oversized:?}"
    );

    let readiness = get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "streamed-body rejection must not poison process readiness: {readiness:?}"
    );

    drop(backend);
    drop(frontend);
}

#[test]
fn compiled_pg_erd_in_flight_saturation_rejects_recovers_and_preserves_control_plane() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");
    let gateway_address = reserve_loopback();
    let metrics_address = reserve_loopback();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
        1,
    );

    let (request_seen_tx, request_seen_rx) = mpsc::channel();
    let (release_response_tx, release_response_rx) = mpsc::channel();
    let origin = thread::spawn(move || {
        let (mut held, _) = backend
            .accept()
            .expect("first admitted request should reach backend");
        let held_request = read_request_headers(&mut held);
        assert!(held_request.starts_with("GET /api/held HTTP/1.1\r\n"));
        request_seen_tx
            .send(())
            .expect("test should observe the admitted request");
        release_response_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("test should release the held response");
        held.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nheld",
        )
        .expect("held response should be writable");

        let (mut recovered, _) = backend
            .accept()
            .expect("capacity recovery should admit another backend request");
        let recovered_request = read_request_headers(&mut recovered);
        assert!(recovered_request.starts_with("GET /api/recovered HTTP/1.1\r\n"));
        recovered
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nrecovered",
            )
            .expect("recovery response should be writable");
    });

    let _process = start_gateway(&config, gateway_address, metrics_address);
    let held = thread::spawn(move || get(gateway_address, "/api/held"));
    request_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first request should hold the sole admission lease");

    let rejected = get(gateway_address, "/api/over-capacity");
    assert!(
        rejected.starts_with("HTTP/1.1 503"),
        "request above max_in_flight_requests must fail fast: {rejected:?}"
    );

    let readiness = get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "process health must remain available while application capacity is saturated: {readiness:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(
        metrics.contains("cwl_pingora_gateway_backpressure_rejections_total 1"),
        "saturation must be visible through low-cardinality gateway telemetry: {metrics:?}"
    );

    release_response_tx
        .send(())
        .expect("held request should be released");
    let held_response = held.join().expect("held request should complete");
    assert!(held_response.starts_with("HTTP/1.1 200"));
    assert!(held_response.ends_with("\r\n\r\nheld"));

    let recovered = get(gateway_address, "/api/recovered");
    assert!(
        recovered.starts_with("HTTP/1.1 200"),
        "admission capacity must recover after request completion: {recovered:?}"
    );
    assert!(recovered.ends_with("\r\n\r\nrecovered"));

    origin.join().expect("backend fixture should complete");
    drop(frontend);
}
