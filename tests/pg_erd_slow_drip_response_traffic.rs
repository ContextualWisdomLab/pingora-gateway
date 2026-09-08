//! Real-listener RED→GREEN contract for a pg-erd upstream that continuously drips response-body bytes.
//!
//! Pingora's peer `read_timeout` is an inactivity timeout that resets after each successful read.
//! This fixture therefore keeps each origin write well inside `read_ms` while extending the response
//! beyond an explicit migration-owned response-body lifetime. The version-2 admin/runtime boundary
//! must terminate that body without retrying or failing over after the response has been committed.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const SOCKET_IO_TIMEOUT: Duration = Duration::from_secs(5);

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

/// Holds traffic and metrics ports simultaneously so sequential bind/drop cannot reuse one port.
fn reserve_distinct_loopback_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be reservable");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    assert_ne!(
        traffic.local_addr().expect("traffic reservation address"),
        metrics.local_addr().expect("metrics reservation address")
    );
    (traffic, metrics)
}

/// Writes the version-2 bounded response-lifetime contract while route authority remains compiled.
fn write_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nmax_upstream_response_body_ms: 300\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 500\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
    )
    .expect("migration config should be written");
    file
}

/// Waits for one real listener while failing immediately if process activation aborts.
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

/// Starts the compiled pg-erd gateway after its traffic/metrics reservations are released.
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

/// Applies finite origin I/O bounds before any fixture read or write can stall the hosted lane.
fn set_origin_deadlines(stream: &TcpStream) {
    stream
        .set_read_timeout(Some(SOCKET_IO_TIMEOUT))
        .expect("origin read timeout should be configurable");
    stream
        .set_write_timeout(Some(SOCKET_IO_TIMEOUT))
        .expect("origin write timeout should be configurable");
}

/// Sends a bounded raw request used for readiness, metrics, and independent-route recovery checks.
fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let mut downstream = TcpStream::connect(address).expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(SOCKET_IO_TIMEOUT))
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

/// Captures the committed response until EOF/RST and records the wall-clock termination boundary.
fn raw_request_until_terminal(
    address: SocketAddr,
    request: &[u8],
) -> (Vec<u8>, DownstreamTermination, Duration) {
    let started = Instant::now();
    let mut downstream = TcpStream::connect(address).expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("downstream timeout should be configurable");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");

    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        match downstream.read(&mut buffer) {
            Ok(0) => return (response, DownstreamTermination::Eof, started.elapsed()),
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == ErrorKind::ConnectionReset => {
                return (
                    response,
                    DownstreamTermination::ConnectionReset,
                    started.elapsed(),
                );
            }
            Err(error) => {
                panic!("slow-drip downstream response should terminate, not stall: {error}")
            }
        }
    }
}

/// Sends one characterized close-delimited GET through a gateway or metrics listener.
fn get(address: SocketAddr, path: &str) -> String {
    raw_request(
        address,
        format!("GET {path} HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

/// Reads exactly one finite HTTP/1 request header block from an origin connection.
fn read_request_headers(stream: &mut TcpStream) -> String {
    set_origin_deadlines(stream);
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable inside the fixture deadline");
        assert!(
            read > 0,
            "gateway closed origin request before headers completed"
        );
        bytes.extend_from_slice(&buffer[..read]);
        assert!(
            bytes.len() <= MAX_REQUEST_HEADER_BYTES,
            "origin request headers exceeded the 64 KiB fixture bound"
        );
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

/// Parses only an exact case-sensitive HTTP/1.1 three-digit status token.
fn http11_status(response: &str) -> Option<u16> {
    let mut tokens = response.lines().next()?.split_ascii_whitespace();
    if tokens.next()? != "HTTP/1.1" {
        return None;
    }
    let code = tokens.next()?;
    if code.len() != 3 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    code.parse().ok()
}

/// Returns semantically exact values for one HTTP field name, rejecting lookalike field names.
fn header_values(headers: &str, field_name: &str) -> Vec<String> {
    headers
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case(field_name))
        .map(|(_, value)| value.trim().to_string())
        .collect()
}

/// Requires one exact unlabelled Prometheus sample so numeric-prefix values cannot create GREEN.
fn has_exact_metric_sample(metrics: &str, sample: &str) -> bool {
    metrics.lines().any(|line| line.trim() == sample)
}

/// Proves progress inside `read_ms` cannot evade the versioned whole-body lifetime boundary.
#[test]
fn compiled_pg_erd_terminates_continuous_response_drip_without_poisoning_other_routes() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/slow-drip HTTP/1.1\r\n"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\n")
            .expect("backend response header should be writable");

        for _ in 0..20 {
            match stream.write_all(b"x") {
                Ok(()) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::BrokenPipe
                            | ErrorKind::ConnectionReset
                            | ErrorKind::NotConnected
                    ) =>
                {
                    break;
                }
                Err(error) => panic!("unexpected slow-drip origin write failure: {error}"),
            }
            thread::sleep(Duration::from_millis(60));
        }
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
        assert!(request.starts_with("GET /after-slow-drip HTTP/1.1\r\n"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nrecovered",
            )
            .expect("frontend recovery response should be writable");
    });

    let (traffic_reservation, metrics_reservation) = reserve_distinct_loopback_listeners();
    let gateway_address = traffic_reservation
        .local_addr()
        .expect("traffic reservation address");
    let metrics_address = metrics_reservation
        .local_addr()
        .expect("metrics reservation address");
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );
    drop(traffic_reservation);
    drop(metrics_reservation);
    let _process = start_gateway(&config, gateway_address, metrics_address);

    let (partial, termination, elapsed) = raw_request_until_terminal(
        gateway_address,
        b"GET /api/slow-drip HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n",
    );
    assert!(
        matches!(
            termination,
            DownstreamTermination::Eof | DownstreamTermination::ConnectionReset
        ),
        "an over-budget response body must terminate the downstream connection"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "the 300ms response-body budget must stop a continuously progressing response instead of allowing the full 1.2s drip: {elapsed:?}"
    );

    let header_end = partial
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .expect("slow-drip response must commit a complete header block before termination");
    let headers = String::from_utf8_lossy(&partial[..header_end]);
    assert_eq!(
        http11_status(&headers),
        Some(200),
        "a post-commit lifetime failure cannot be rewritten as a second status: {headers:?}"
    );
    assert_eq!(
        header_values(&headers, "Content-Length"),
        vec!["20"],
        "the committed framing must remain the characterized single Content-Length field"
    );
    let body = &partial[header_end..];
    assert!(
        body.len() < 20,
        "the configured response-body budget must terminate before the declared body completes"
    );

    let readiness = get(gateway_address, "/readyz");
    assert_eq!(
        http11_status(&readiness),
        Some(200),
        "one slow-drip origin must not poison process readiness: {readiness:?}"
    );
    let metrics = get(metrics_address, "/metrics");
    assert!(
        has_exact_metric_sample(&metrics, "cwl_pingora_gateway_request_errors_total 1"),
        "response-lifetime enforcement must remain visible through low-cardinality error telemetry: {metrics:?}"
    );
    let recovered = get(gateway_address, "/after-slow-drip");
    assert_eq!(http11_status(&recovered), Some(200));
    assert!(recovered.ends_with("\r\n\r\nrecovered"));

    frontend_origin
        .join()
        .expect("frontend recovery fixture should complete");
    backend_origin
        .join()
        .expect("slow-drip backend fixture should complete");
}

/// Prevents status, header-name, and metric-value lookalikes from satisfying the traffic oracle.
#[test]
fn slow_drip_evidence_parsers_reject_lookalikes() {
    assert_eq!(http11_status("HTTP/1.1 200 OK\r\n\r\n"), Some(200));
    assert_eq!(http11_status("HTTP/1.1 2000 Weird\r\n\r\n"), None);
    assert_eq!(http11_status("http/1.1 200 OK\r\n\r\n"), None);

    let headers = "HTTP/1.1 200 OK\r\nX-Content-Length: 20\r\ncontent-length: 20\r\n\r\n";
    assert_eq!(header_values(headers, "Content-Length"), vec!["20"]);

    assert!(has_exact_metric_sample(
        "cwl_pingora_gateway_request_errors_total 1\n",
        "cwl_pingora_gateway_request_errors_total 1"
    ));
    assert!(!has_exact_metric_sample(
        "cwl_pingora_gateway_request_errors_total 10\n",
        "cwl_pingora_gateway_request_errors_total 1"
    ));
}
