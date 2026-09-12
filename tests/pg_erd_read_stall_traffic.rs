//! Real-listener upstream read-stall acceptance for the dedicated pg-erd migration binary.
//!
//! This contract distinguishes a connected-but-silent characterized backend from connection
//! refusal. It proves Pingora's configured per-read upstream timeout reaches the compiled migration
//! process, fails closed, leaves process health observable, records the transport failure, and does
//! not poison an independent characterized route.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

/// Owns the compiled gateway child so every assertion path tears the process down.
struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    /// Terminates and reaps the child even when the traffic contract panics before normal teardown.
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Selects traffic and metrics loopback authorities while both ephemeral reservations remain held.
fn reserve_distinct_loopbacks() -> (SocketAddr, SocketAddr) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be reservable");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    let traffic_address = traffic
        .local_addr()
        .expect("traffic reservation should expose an address");
    let metrics_address = metrics
        .local_addr()
        .expect("metrics reservation should expose an address");
    assert_ne!(
        traffic_address, metrics_address,
        "traffic and metrics reservations must remain distinct while both sockets are held"
    );
    (traffic_address, metrics_address)
}

/// Writes the bounded pg-erd fixture with a 100 ms backend read budget and independent frontend.
fn write_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 100\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
    )
    .expect("migration config should be written");
    file
}

/// Waits for one listener without allowing an early child exit to look like startup success.
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

/// Starts the compiled migration binary and requires both traffic and metrics listeners to bind.
fn start_gateway(
    config: &NamedTempFile,
    gateway_address: SocketAddr,
    metrics_address: SocketAddr,
) -> GatewayProcess {
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

/// Sends one connection-closing HTTP/1.1 request and captures the complete downstream response.
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

/// Issues a fixture GET with the characterized downstream authority and explicit connection close.
fn get(address: SocketAddr, path: &str) -> String {
    raw_request(
        address,
        format!("GET {path} HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

/// Reads only through the HTTP header terminator so the silent fixture never emits response bytes.
fn read_request_headers(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable");
        assert!(
            read > 0,
            "gateway closed origin request before headers completed"
        );
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

/// Proves connected upstream inactivity fails closed without poisoning readiness or another route.
#[test]
fn compiled_pg_erd_silent_backend_hits_read_timeout_and_preserves_independent_routing() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let (backend_connected_tx, backend_connected_rx) = mpsc::channel();
    let (release_backend_tx, release_backend_rx) = mpsc::channel();
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/read-stall HTTP/1.1\r\n"));
        backend_connected_tx
            .send(())
            .expect("test should observe the connected silent backend");

        // Keep the accepted connection open and send no response bytes until the gateway has
        // already produced its downstream failure. This prevents fixture closure from masquerading
        // as the configured 100 ms per-read timeout.
        release_backend_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("gateway should time out the silent backend before fixture release");
    });

    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend
        .local_addr()
        .expect("frontend address should exist");
    let frontend_origin = thread::spawn(move || {
        let (mut stream, _) = frontend
            .accept()
            .expect("fallback request should reach the independent frontend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /after-read-timeout HTTP/1.1\r\n"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nrecovered",
            )
            .expect("frontend recovery response should be writable");
    });

    let (gateway_address, metrics_address) = reserve_distinct_loopbacks();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );
    let _process = start_gateway(&config, gateway_address, metrics_address);

    let started = Instant::now();
    let failed = get(gateway_address, "/api/read-stall");
    let failure_elapsed = started.elapsed();
    backend_connected_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the failure case must have connected to the characterized backend");
    assert!(
        failed.starts_with("HTTP/1.1 502"),
        "a characterized upstream read timeout must fail as Bad Gateway: {failed:?}"
    );
    assert!(
        failure_elapsed < Duration::from_secs(1),
        "a silent connected upstream must fail inside a conservative one-second envelope around the configured 100 ms per-read timeout; elapsed={failure_elapsed:?}"
    );
    release_backend_tx
        .send(())
        .expect("silent backend fixture should be released after timeout evidence is captured");

    let readiness = get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "one upstream read timeout must not poison process readiness: {readiness:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(
        metrics
            .lines()
            .any(|line| line == "cwl_pingora_gateway_request_errors_total 1"),
        "the upstream read timeout must expose exactly one request error through low-cardinality telemetry: {metrics:?}"
    );

    let recovered = get(gateway_address, "/after-read-timeout");
    assert!(
        recovered.starts_with("HTTP/1.1 200"),
        "an independent characterized route must remain usable after a backend read timeout: {recovered:?}"
    );
    assert!(recovered.ends_with("\r\n\r\nrecovered"));

    frontend_origin
        .join()
        .expect("frontend recovery fixture should complete");
    backend_origin
        .join()
        .expect("silent backend fixture should complete");
}
