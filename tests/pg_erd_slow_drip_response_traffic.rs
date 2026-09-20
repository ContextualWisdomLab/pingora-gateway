//! Real-listener RED→GREEN contract for a pg-erd upstream that continuously drips response-body bytes.
//!
//! Pingora's peer `read_timeout` is an inactivity timeout that resets after each successful read.
//! This fixture therefore keeps each origin write well inside `read_ms` while extending the response
//! beyond an explicit migration-owned response-body lifetime. Fixture readiness and I/O use absolute
//! deadlines so partial progress cannot renew the evidence window.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const MAX_HEADER_BYTES: usize = 64 * 1024;
const FIXTURE_IO_TIMEOUT: Duration = Duration::from_secs(5);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);
const PRE_RESPONSE_HEADER_DELAY: Duration = Duration::from_millis(150);
const RESPONSE_BODY_LIFETIME: Duration = Duration::from_millis(300);

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

fn reserve_distinct_loopback_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be reservable");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    assert_ne!(
        traffic.local_addr().expect("traffic reservation address"),
        metrics.local_addr().expect("metrics reservation address")
    );
    (traffic, metrics)
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
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nmax_upstream_response_body_ms: 300\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 500\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
    )
    .expect("migration config should be written");
    file
}

fn remaining(deadline: Instant, context: &str) -> Duration {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .unwrap_or_else(|| panic!("absolute fixture deadline expired while {context}"))
}

fn set_read_timeout_to_remaining(stream: &TcpStream, deadline: Instant, context: &str) {
    stream
        .set_read_timeout(Some(remaining(deadline, context)))
        .expect("read timeout should be configurable");
}

fn set_write_timeout_to_remaining(stream: &TcpStream, deadline: Instant, context: &str) {
    stream
        .set_write_timeout(Some(remaining(deadline, context)))
        .expect("write timeout should be configurable");
}

fn read_header_block(
    stream: &mut TcpStream,
    deadline: Instant,
    context: &str,
) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        set_read_timeout_to_remaining(stream, deadline, context);
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "connection closed before response headers completed",
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "header block exceeded 64 KiB fixture bound",
            ));
        }
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(bytes);
        }
    }
}

fn probe_http_status(address: SocketAddr, path: &str, deadline: Instant) -> Option<u16> {
    let connect_timeout =
        remaining(deadline, "connecting readiness probe").min(Duration::from_millis(100));
    let mut stream = TcpStream::connect_timeout(&address, connect_timeout).ok()?;
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: readiness.invalid\r\nConnection: close\r\n\r\n");
    set_write_timeout_to_remaining(&stream, deadline, "writing readiness probe");
    stream.write_all(request.as_bytes()).ok()?;
    let headers = read_header_block(&mut stream, deadline, "reading readiness response").ok()?;
    http11_status(&String::from_utf8_lossy(&headers))
}

fn wait_until_http_status(address: SocketAddr, path: &str, process: &mut Child, expected: u16) {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before HTTP readiness: {status}");
        }

        if Instant::now() >= deadline {
            panic!("gateway did not return HTTP {expected} for {path} within 10s");
        }

        if probe_http_status(address, path, deadline) == Some(expected) {
            return;
        }

        let sleep_for =
            remaining(deadline, "waiting to retry readiness").min(Duration::from_millis(25));
        thread::sleep(sleep_for);
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

    wait_until_http_status(gateway_address, "/readyz", &mut child, 200);
    wait_until_http_status(metrics_address, "/metrics", &mut child, 200);
    GatewayProcess(child)
}

fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let deadline = Instant::now() + FIXTURE_IO_TIMEOUT;
    let mut downstream =
        TcpStream::connect_timeout(&address, remaining(deadline, "connecting bounded request"))
            .expect("gateway should accept traffic");
    set_write_timeout_to_remaining(&downstream, deadline, "writing bounded request");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");

    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        set_read_timeout_to_remaining(&downstream, deadline, "reading bounded response");
        match downstream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => response.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == ErrorKind::ConnectionReset => break,
            Err(error) => {
                panic!("gateway response should complete inside absolute deadline: {error}")
            }
        }
    }
    String::from_utf8_lossy(&response).into_owned()
}

fn raw_request_until_terminal(
    address: SocketAddr,
    request: &[u8],
) -> (Vec<u8>, DownstreamTermination, Duration) {
    let started = Instant::now();
    let deadline = started + TERMINATION_TIMEOUT;
    let mut downstream = TcpStream::connect_timeout(
        &address,
        remaining(deadline, "connecting slow-drip downstream"),
    )
    .expect("gateway should accept traffic");
    set_write_timeout_to_remaining(&downstream, deadline, "writing slow-drip request");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");

    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        set_read_timeout_to_remaining(&downstream, deadline, "reading slow-drip termination");
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
            Err(error) => panic!(
                "slow-drip downstream response must terminate inside one absolute 2s deadline: {error}"
            ),
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
    let deadline = Instant::now() + FIXTURE_IO_TIMEOUT;
    let bytes = read_header_block(stream, deadline, "reading origin request headers")
        .expect("origin request headers should complete inside one absolute 5s deadline");
    String::from_utf8_lossy(&bytes).into_owned()
}

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

fn header_values(headers: &str, field_name: &str) -> Vec<String> {
    headers
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case(field_name))
        .map(|(_, value)| value.trim().to_string())
        .collect()
}

fn has_exact_metric_sample(metrics: &str, sample: &str) -> bool {
    metrics.lines().any(|line| line.trim() == sample)
}

#[test]
fn compiled_pg_erd_terminates_continuous_response_drip_without_poisoning_other_routes() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        stream
            .set_write_timeout(Some(FIXTURE_IO_TIMEOUT))
            .expect("origin write timeout should be configurable");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/slow-drip HTTP/1.1\r\n"));

        thread::sleep(PRE_RESPONSE_HEADER_DELAY);
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
        stream
            .set_write_timeout(Some(FIXTURE_IO_TIMEOUT))
            .expect("origin write timeout should be configurable");
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
    assert!(matches!(
        termination,
        DownstreamTermination::Eof | DownstreamTermination::ConnectionReset
    ));
    assert!(
        elapsed < Duration::from_secs(1),
        "response-body lifetime must stop the continuous drip instead of allowing completion: {elapsed:?}"
    );
    assert!(
        elapsed >= PRE_RESPONSE_HEADER_DELAY + RESPONSE_BODY_LIFETIME,
        "lifetime must start at the final response header rather than request start: {elapsed:?}"
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
        "post-commit lifetime failure cannot be rewritten as a second status: {headers:?}"
    );
    assert_eq!(header_values(&headers, "Content-Length"), vec!["20"]);
    let body = &partial[header_end..];
    assert!(
        !body.is_empty(),
        "body progress must cross the callback boundary"
    );
    assert!(
        body.len() < 20,
        "configured lifetime must terminate before the declared body completes"
    );

    let readiness = get(gateway_address, "/readyz");
    assert_eq!(http11_status(&readiness), Some(200));
    let metrics = get(metrics_address, "/metrics");
    assert!(
        has_exact_metric_sample(&metrics, "cwl_pingora_gateway_request_errors_total 1"),
        "lifetime enforcement must remain visible through bounded error telemetry: {metrics:?}"
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
