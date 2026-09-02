//! Real-listener partial upstream response acceptance for the dedicated pg-erd migration binary.
//!
//! This contract distinguishes a response that fails after its status/header block has already
//! been received from failures that occur before downstream response commitment. It proves that a
//! truncated characterized origin response is not rewritten into an invented retry/failover,
//! leaves process health observable, records the transport failure, and does not poison an
//! independent characterized route.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownstreamTermination {
    Eof,
    ConnectionReset,
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
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 500\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
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

fn raw_request_until_terminal(
    address: SocketAddr,
    request: &[u8],
) -> (Vec<u8>, DownstreamTermination) {
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
        match downstream.read(&mut buffer) {
            Ok(0) => return (response, DownstreamTermination::Eof),
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == ErrorKind::ConnectionReset => {
                return (response, DownstreamTermination::ConnectionReset);
            }
            Err(error) => panic!("partial downstream response should terminate, not stall: {error}"),
        }
    }
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
fn compiled_pg_erd_truncated_response_stays_committed_and_preserves_independent_routing() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/partial-response HTTP/1.1\r\n"));

        // Commit a valid response header and only part of the declared payload, then close. Once
        // that status/header block has been forwarded, a later upstream framing failure cannot be
        // replaced with a second HTTP status or silently failed over to another origin.
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\npartial",
            )
            .expect("partial backend response should be writable");
    });

    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");
    let frontend_origin = thread::spawn(move || {
        let (mut stream, _) = frontend
            .accept()
            .expect("fallback request should reach the independent frontend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /after-partial-response HTTP/1.1\r\n"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nrecovered",
            )
            .expect("frontend recovery response should be writable");
    });

    let gateway_address = reserve_loopback();
    let metrics_address = reserve_loopback();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );
    let _process = start_gateway(&config, gateway_address, metrics_address);

    let (partial, termination) = raw_request_until_terminal(
        gateway_address,
        b"GET /api/partial-response HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n",
    );
    assert!(
        matches!(
            termination,
            DownstreamTermination::Eof | DownstreamTermination::ConnectionReset
        ),
        "a committed truncated response must terminate the downstream connection"
    );
    let header_end = partial
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .expect("committed partial response must contain a complete header block");
    let headers = String::from_utf8_lossy(&partial[..header_end]).to_ascii_lowercase();
    assert!(
        headers.starts_with("http/1.1 200"),
        "a post-header upstream failure cannot be rewritten as a new status: {headers:?}"
    );
    assert!(
        headers.contains("content-length: 20"),
        "the committed response must retain its declared framing for this fixture: {headers:?}"
    );
    let body = &partial[header_end..];
    assert_eq!(body, b"partial");
    assert!(
        body.len() < 20,
        "fixture must terminate before its declared response body completes"
    );

    let readiness = get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "one truncated upstream response must not poison process readiness: {readiness:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(
        metrics.contains("cwl_pingora_gateway_request_errors_total 1"),
        "the post-header upstream framing failure must remain visible through low-cardinality error telemetry: {metrics:?}"
    );

    let recovered = get(gateway_address, "/after-partial-response");
    assert!(
        recovered.starts_with("HTTP/1.1 200"),
        "an independent characterized route must remain usable after a truncated response: {recovered:?}"
    );
    assert!(recovered.ends_with("\r\n\r\nrecovered"));

    frontend_origin
        .join()
        .expect("frontend recovery fixture should complete");
    backend_origin
        .join()
        .expect("partial backend fixture should complete");
}
