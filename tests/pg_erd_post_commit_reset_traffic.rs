#![cfg(target_os = "linux")]

//! Real-listener post-commit upstream TCP reset acceptance for the dedicated pg-erd migration.
//!
//! This contract distinguishes an abortive upstream close after a valid response header and partial
//! body have already crossed the proxy from both the pre-header reset and orderly truncation cases.
//! Once downstream response commitment exists, the gateway must preserve that status/framing,
//! terminate the incomplete response, record the transport failure, and keep unrelated routing
//! healthy rather than inventing retry/failover or a second HTTP status.

use core::ffi::c_void;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const SOL_SOCKET: i32 = 1;
const SO_LINGER: i32 = 13;
const MAX_ORIGIN_REQUEST_HEADER_BYTES: usize = 64 * 1024;

#[repr(C)]
struct Linger {
    onoff: i32,
    linger_seconds: i32,
}

unsafe extern "C" {
    fn setsockopt(
        socket: i32,
        level: i32,
        option_name: i32,
        option_value: *const c_void,
        option_len: u32,
    ) -> i32;
}

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

/// Reserves traffic and metrics listeners simultaneously so sequential
/// bind-and-drop cannot reuse one ephemeral port and invalidate Admin Config.
fn reserve_gateway_addresses() -> (TcpListener, TcpListener, SocketAddr, SocketAddr) {
    let gateway = TcpListener::bind("127.0.0.1:0").expect("gateway port should be reservable");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    let gateway_address = gateway
        .local_addr()
        .expect("gateway reservation should expose an address");
    let metrics_address = metrics
        .local_addr()
        .expect("metrics reservation should expose an address");
    assert_ne!(
        gateway_address, metrics_address,
        "traffic and metrics reservations must remain distinct"
    );
    (gateway, metrics, gateway_address, metrics_address)
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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 5000\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
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

fn raw_request_until_committed_then_reset(
    address: SocketAddr,
    request: &[u8],
    reset_release: mpsc::Sender<()>,
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
    let mut reset_released = false;
    loop {
        match downstream.read(&mut buffer) {
            Ok(0) => return (response, DownstreamTermination::Eof),
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if !reset_released {
                    if let Some(header_end) = response
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .map(|position| position + 4)
                    {
                        if response.len() >= header_end + b"partial".len() {
                            assert_eq!(
                                &response[header_end..header_end + b"partial".len()],
                                b"partial",
                                "downstream must observe the committed body prefix before reset"
                            );
                            reset_release
                                .send(())
                                .expect("backend reset fixture should still await release");
                            reset_released = true;
                        }
                    }
                }
            }
            Err(error) if error.kind() == ErrorKind::ConnectionReset => {
                return (response, DownstreamTermination::ConnectionReset);
            }
            Err(error) => panic!("post-commit reset response should terminate, not stall: {error}"),
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

/// Keeps origin-side request reads finite so a forwarding defect fails before
/// the intended reset phase instead of hanging the acceptance suite.
fn read_request_headers(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("origin request timeout should be configurable");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable before the fixture deadline");
        assert!(
            read > 0,
            "gateway closed origin request before headers completed"
        );
        bytes.extend_from_slice(&buffer[..read]);
        assert!(
            bytes.len() <= MAX_ORIGIN_REQUEST_HEADER_BYTES,
            "gateway origin request headers exceeded the fixture bound"
        );
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

/// Selects one exact HTTP field name case-insensitively and trims field-value
/// OWS so lookalike fields cannot satisfy a framing assertion.
fn header_values<'a>(headers: &'a str, name: &str) -> Vec<&'a str> {
    headers
        .split("\r\n")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .filter_map(|(field_name, value)| {
            field_name
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
        .collect()
}

/// Requires a complete Prometheus sample line so numeric-prefix values cannot
/// manufacture the expected exact counter sample.
fn contains_exact_metric_sample(metrics: &str, sample: &str) -> bool {
    metrics
        .lines()
        .any(|line| line.trim_end_matches('\r') == sample)
}

fn reset_on_close(stream: &TcpStream) {
    let linger = Linger {
        onoff: 1,
        linger_seconds: 0,
    };
    // SAFETY: the file descriptor belongs to this live TcpStream, `linger` matches Linux's
    // `struct linger { int l_onoff; int l_linger; }`, and the pointer remains valid for the call.
    let result = unsafe {
        setsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_LINGER,
            (&linger as *const Linger).cast(),
            std::mem::size_of::<Linger>() as u32,
        )
    };
    assert_eq!(
        result,
        0,
        "Linux SO_LINGER(0) should be configurable for the reset fixture: {}",
        std::io::Error::last_os_error()
    );
}

#[test]
fn exact_response_header_matching_rejects_content_length_lookalikes() {
    let headers = "HTTP/1.1 200 OK\r\nX-Content-Length: 20\r\ncOnTeNt-LeNgTh: 7\r\n\r\n";
    assert_eq!(header_values(headers, "Content-Length"), vec!["7"]);
}

#[test]
fn exact_metric_sample_rejects_numeric_prefix_lookalikes() {
    let metrics = "# TYPE cwl_pingora_gateway_request_errors_total counter\ncwl_pingora_gateway_request_errors_total 10\n";
    assert!(!contains_exact_metric_sample(
        metrics,
        "cwl_pingora_gateway_request_errors_total 1"
    ));
}

#[test]
fn compiled_pg_erd_post_commit_reset_preserves_committed_status_and_independent_routing() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let (reset_release, reset_wait) = mpsc::channel();
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/post-commit-reset HTTP/1.1\r\n"));

        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\npartial",
            )
            .expect("committed backend response should be writable");

        // The origin abort is released only after the downstream has observed the committed header
        // and body prefix. This makes the transport phase causal instead of relying on a sleep that
        // can race scheduler or socket-buffer timing on loaded CI runners.
        reset_wait
            .recv_timeout(Duration::from_secs(10))
            .expect("downstream should observe the committed prefix before fixture timeout");
        reset_on_close(&stream);
        drop(stream);
    });

    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");
    let frontend_origin = thread::spawn(move || {
        let (mut stream, _) = frontend
            .accept()
            .expect("fallback request should reach the independent frontend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /after-post-commit-reset HTTP/1.1\r\n"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nrecovered",
            )
            .expect("frontend recovery response should be writable");
    });

    let (gateway_reservation, metrics_reservation, gateway_address, metrics_address) =
        reserve_gateway_addresses();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );
    drop(gateway_reservation);
    drop(metrics_reservation);
    let _process = start_gateway(&config, gateway_address, metrics_address);

    let (partial, termination) = raw_request_until_committed_then_reset(
        gateway_address,
        b"GET /api/post-commit-reset HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n",
        reset_release,
    );
    assert!(
        matches!(
            termination,
            DownstreamTermination::Eof | DownstreamTermination::ConnectionReset
        ),
        "a post-commit upstream reset must terminate the incomplete downstream response"
    );
    let header_end = partial
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .expect("post-commit reset response must contain the committed header block");
    let headers = String::from_utf8_lossy(&partial[..header_end]);
    assert!(
        headers.to_ascii_lowercase().starts_with("http/1.1 200"),
        "a reset after downstream commitment cannot be rewritten as a second status: {headers:?}"
    );
    assert_eq!(
        header_values(&headers, "Content-Length"),
        vec!["20"],
        "the committed response must retain exactly one declared framing field: {headers:?}"
    );
    let body = &partial[header_end..];
    assert_eq!(body, b"partial");
    assert!(
        body.len() < 20,
        "fixture must reset before its declared response body completes"
    );

    let readiness = get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "one post-commit upstream reset must not poison process readiness: {readiness:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(
        contains_exact_metric_sample(&metrics, "cwl_pingora_gateway_request_errors_total 1"),
        "the post-commit reset must remain visible as exactly one low-cardinality error sample: {metrics:?}"
    );

    let recovered = get(gateway_address, "/after-post-commit-reset");
    assert!(
        recovered.starts_with("HTTP/1.1 200"),
        "an independent characterized route must remain usable after a post-commit reset: {recovered:?}"
    );
    assert!(recovered.ends_with("\r\n\r\nrecovered"));

    frontend_origin
        .join()
        .expect("frontend recovery fixture should complete");
    backend_origin
        .join()
        .expect("post-commit reset backend fixture should complete");
}
