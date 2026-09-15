#![cfg(target_os = "linux")]

//! Real-listener post-commit upstream TCP reset acceptance for the dedicated pg-erd migration.
//!
//! This contract distinguishes an abortive upstream close after a valid response header has crossed
//! the origin-side TCP connection from both the pre-header reset and orderly truncation cases. The
//! origin emits a partial body, waits for the peer TCP stack to acknowledge all emitted bytes, and
//! only then resets. Downstream body delivery is deliberately not a prerequisite for releasing the
//! abort because proxy buffering is not response-commit authority. The gateway must preserve the
//! first downstream status/framing, terminate the incomplete response, record the transport failure,
//! and keep unrelated routing healthy rather than inventing retry/failover or a second HTTP status.

use core::ffi::c_void;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const SOL_SOCKET: i32 = 1;
const SO_LINGER: i32 = 13;
const SIOCOUTQ: usize = 0x5411;
const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;

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
    fn ioctl(socket: i32, request: usize, ...) -> i32;
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

/// Writes only the admitted migration transport configuration needed to isolate
/// post-commit reset behavior from product policy or dynamic route authority.
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

/// Parses only an exact HTTP/1.1 three-digit status token so case changes or
/// numeric-prefix lookalikes cannot satisfy the response-status oracle.
fn exact_http_1_1_status_code(response: &str) -> Option<u16> {
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

/// Returns the remaining per-operation timeout without allowing one operation to outlive the caller.
fn bounded_timeout(deadline: Instant, cap: Duration) -> Option<Duration> {
    let remaining = deadline.checked_duration_since(Instant::now())?;
    if remaining.is_zero() {
        return None;
    }
    Some(remaining.min(cap))
}

/// Attempts one bounded application-level readiness exchange. A successful TCP
/// handshake alone is not sufficient to admit the failure fixture.
fn probe_readyz(address: SocketAddr, deadline: Instant) -> bool {
    let Some(connect_timeout) = bounded_timeout(deadline, Duration::from_millis(100)) else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&address, connect_timeout) else {
        return false;
    };
    let Some(write_timeout) = bounded_timeout(deadline, Duration::from_millis(250)) else {
        return false;
    };
    if stream.set_write_timeout(Some(write_timeout)).is_err() {
        return false;
    }
    if stream
        .write_all(b"GET /readyz HTTP/1.1\r\nHost: readiness.local\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }

    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let Some(read_timeout) = bounded_timeout(deadline, Duration::from_millis(250)) else {
            return false;
        };
        if stream.set_read_timeout(Some(read_timeout)).is_err() {
            return false;
        }
        match stream.read(&mut buffer) {
            Ok(0) => return false,
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                if response.len() > MAX_HTTP_HEADER_BYTES {
                    return false;
                }
                if let Some(header_end) = response
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|position| position + 4)
                {
                    let headers = String::from_utf8_lossy(&response[..header_end]);
                    return exact_http_1_1_status_code(&headers) == Some(200);
                }
            }
            Err(_) => return false,
        }
    }
}

/// Waits for complete `/readyz` HTTP/1.1 200 while failing immediately if the
/// child exits, preventing a bare listener from manufacturing startup evidence.
fn wait_until_http_ready(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before application readiness: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not become application-ready within 10s"
        );
        if probe_readyz(address, deadline) {
            return;
        }
        let Some(sleep_budget) = bounded_timeout(deadline, Duration::from_millis(25)) else {
            panic!("gateway did not become application-ready within 10s");
        };
        thread::sleep(sleep_budget);
    }
}

/// Waits only for metrics-listener presence. The later real `/metrics` request
/// remains the application-level identity oracle for that separate service.
fn wait_until_listening(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before metrics listener startup: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "metrics listener did not start within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

/// Starts the compiled pg-erd composition root after retaining both socket
/// reservations through config construction and until the child-bind handoff.
fn start_gateway(
    config: &NamedTempFile,
    gateway_reservation: TcpListener,
    metrics_reservation: TcpListener,
    gateway_address: SocketAddr,
    metrics_address: SocketAddr,
) -> GatewayProcess {
    drop(gateway_reservation);
    drop(metrics_reservation);
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    wait_until_http_ready(gateway_address, &mut child);
    wait_until_listening(metrics_address, &mut child);
    GatewayProcess(child)
}

/// Sends a small raw HTTP/1.1 request with a finite downstream read budget for
/// metrics, readiness re-checks, and independent-route recovery probes.
fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let mut downstream = TcpStream::connect_timeout(&address, Duration::from_secs(1))
        .expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    downstream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .expect("downstream write timeout should be configurable");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");
    let mut response = String::new();
    downstream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    response
}

/// Reads the incomplete downstream response until EOF or propagated RST under
/// one absolute five-second budget. Partial progress cannot renew that budget.
fn raw_request_until_committed_then_reset(
    address: SocketAddr,
    request: &[u8],
) -> (Vec<u8>, DownstreamTermination) {
    let mut downstream = TcpStream::connect_timeout(&address, Duration::from_secs(1))
        .expect("gateway should accept traffic");
    downstream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .expect("downstream write timeout should be configurable");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");
    let deadline = Instant::now() + Duration::from_secs(5);
    read_downstream_until_termination(&mut downstream, deadline)
}

/// Reads downstream evidence without allowing successful partial reads to
/// extend the caller's absolute termination deadline.
fn read_downstream_until_termination(
    downstream: &mut TcpStream,
    deadline: Instant,
) -> (Vec<u8>, DownstreamTermination) {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read_timeout = bounded_timeout(deadline, Duration::from_secs(5))
            .expect("post-commit reset response exceeded its absolute fixture deadline");
        downstream
            .set_read_timeout(Some(read_timeout))
            .expect("downstream timeout should be configurable");
        match downstream.read(&mut buffer) {
            Ok(0) => return (response, DownstreamTermination::Eof),
            Ok(read) => {
                response.extend_from_slice(&buffer[..read]);
                assert!(
                    response.len() <= MAX_HTTP_HEADER_BYTES,
                    "post-commit fixture response exceeded its bounded evidence envelope"
                );
            }
            Err(error) if error.kind() == ErrorKind::ConnectionReset => {
                return (response, DownstreamTermination::ConnectionReset);
            }
            Err(error)
                if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
            {
                panic!("post-commit reset response exceeded its absolute fixture deadline: {error}");
            }
            Err(error) => panic!("post-commit reset response should terminate cleanly: {error}"),
        }
    }
}

/// Builds the fixed-host HTTP/1.1 request used by non-reset control probes.
fn get(address: SocketAddr, path: &str) -> String {
    raw_request(
        address,
        format!("GET {path} HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

/// Keeps origin-side request reads inside one absolute five-second budget so a
/// slow-drip forwarding defect cannot renew a per-read socket timeout forever.
fn read_request_headers(stream: &mut TcpStream) -> String {
    read_request_headers_until(stream, Instant::now() + Duration::from_secs(5))
}

/// Reads one origin request header block without allowing partial progress to
/// extend the caller's absolute deadline.
fn read_request_headers_until(stream: &mut TcpStream, deadline: Instant) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read_timeout = bounded_timeout(deadline, Duration::from_secs(5))
            .expect("origin request headers exceeded the absolute fixture deadline");
        stream
            .set_read_timeout(Some(read_timeout))
            .expect("origin request timeout should be configurable");
        let read = match stream.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => {
                panic!("origin request headers exceeded the absolute fixture deadline: {error}");
            }
            Err(error) => panic!("origin request should be readable before the fixture deadline: {error}"),
        };
        assert!(
            read > 0,
            "gateway closed origin request before headers completed"
        );
        bytes.extend_from_slice(&buffer[..read]);
        assert!(
            bytes.len() <= MAX_HTTP_HEADER_BYTES,
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

/// Requires exactly one complete Prometheus sample line so duplicate or
/// numeric-prefix values cannot manufacture the expected counter evidence.
fn contains_exact_metric_sample(metrics: &str, sample: &str) -> bool {
    let mut exact = metrics
        .lines()
        .filter(|line| line.trim_end_matches('\r') == sample);
    exact.next().is_some() && exact.next().is_none()
}

/// Waits until Linux reports no response bytes outstanding in the origin TCP
/// send queue. Linux implements SIOCOUTQ as `write_seq - snd_una`, so zero means
/// the peer TCP stack has acknowledged every byte emitted before the abort.
fn wait_until_peer_acknowledged_response(stream: &TcpStream) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let mut pending_bytes = 0_i32;
        // SAFETY: SIOCOUTQ expects a live socket fd and a writable `int *` result buffer.
        // `pending_bytes` remains valid for the duration of this synchronous ioctl call.
        let result = unsafe { ioctl(stream.as_raw_fd(), SIOCOUTQ, &mut pending_bytes as *mut i32) };
        assert_eq!(
            result,
            0,
            "Linux SIOCOUTQ should expose the origin send queue: {}",
            std::io::Error::last_os_error()
        );
        assert!(
            pending_bytes >= 0,
            "Linux SIOCOUTQ cannot report a negative send-queue size: {pending_bytes}"
        );
        if pending_bytes == 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "origin response bytes were not acknowledged before the fixture deadline: {pending_bytes} bytes remain"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

/// Configures Linux abortive-close semantics on the established origin socket
/// so this phase exercises a real TCP reset after response bytes were acknowledged.
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

/// Rejects lookalike response fields so only the actual `Content-Length`
/// authority can satisfy the committed-framing oracle.
#[test]
fn exact_response_header_matching_rejects_content_length_lookalikes() {
    let headers = "HTTP/1.1 200 OK\r\nX-Content-Length: 20\r\ncOnTeNt-LeNgTh: 7\r\n\r\n";
    assert_eq!(header_values(headers, "Content-Length"), vec!["7"]);
}

/// Rejects protocol-case and numeric-prefix lookalikes that could otherwise
/// manufacture committed or recovery status evidence.
#[test]
fn exact_status_code_rejects_case_and_numeric_prefix_lookalikes() {
    assert_eq!(exact_http_1_1_status_code("HTTP/1.1 200 OK\r\n"), Some(200));
    assert_eq!(exact_http_1_1_status_code("http/1.1 200 OK\r\n"), None);
    assert_eq!(exact_http_1_1_status_code("HTTP/1.1 2000 OK\r\n"), None);
}

/// Proves the pg-erd readiness oracle cannot be extended by a peer that continuously drips headers.
#[test]
fn readiness_probe_honors_absolute_deadline_under_slow_header_drip() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("slow readiness fixture should bind");
    let address = listener
        .local_addr()
        .expect("slow readiness fixture address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("probe should connect");
        let mut request = [0_u8; 1024];
        let _ = stream
            .read(&mut request)
            .expect("probe request should be readable");
        for byte in b"HTTP/1.1 200 OK\r\nCache-Control: no-store\r\n" {
            if stream.write_all(&[*byte]).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    let started = Instant::now();
    let deadline = started + Duration::from_millis(350);
    assert!(!probe_readyz(address, deadline));
    assert!(
        started.elapsed() < Duration::from_millis(700),
        "pg-erd readiness probe must not reset its total budget on each read"
    );
    server
        .join()
        .expect("slow readiness fixture should complete");
}

/// Proves origin header evidence is bounded by one absolute budget even when a
/// peer keeps making partial progress before each socket-level read timeout.
#[test]
fn origin_header_read_does_not_allow_slow_drip_to_renew_total_budget() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("slow origin fixture should bind");
    let address = listener.local_addr().expect("slow origin fixture address");
    let client = thread::spawn(move || {
        let mut stream = TcpStream::connect(address).expect("origin fixture should accept client");
        for byte in b"GET /api/x" {
            if stream.write_all(&[*byte]).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(700));
        }
    });
    let (mut stream, _) = listener.accept().expect("origin fixture should accept");
    let started = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = read_request_headers(&mut stream);
    }));
    assert!(result.is_err(), "incomplete slow-drip headers must fail closed");
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "origin header evidence must use one absolute five-second deadline"
    );
    client.join().expect("slow origin fixture should complete");
}

/// Proves downstream termination evidence is bounded by one absolute budget
/// even when the peer keeps delivering partial response bytes.
#[test]
fn downstream_termination_read_does_not_allow_slow_drip_to_renew_total_budget() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("slow downstream fixture should bind");
    let address = listener
        .local_addr()
        .expect("slow downstream fixture address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("downstream probe should connect");
        let mut request = [0_u8; 1024];
        let _ = stream
            .read(&mut request)
            .expect("downstream probe request should be readable");
        for byte in b"0123456789" {
            if stream.write_all(&[*byte]).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(700));
        }
    });
    let started = Instant::now();
    let result = std::panic::catch_unwind(|| {
        let _ = raw_request_until_committed_then_reset(
            address,
            b"GET /slow HTTP/1.1\r\nHost: app.example\r\nConnection: close\r\n\r\n",
        );
    });
    assert!(result.is_err(), "slow-drip downstream termination must fail closed");
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "downstream termination evidence must use one absolute five-second deadline"
    );
    server
        .join()
        .expect("slow downstream fixture should complete");
}

/// Rejects numeric-prefix and duplicate metric samples so the oracle proves one
/// post-commit transport error rather than merely finding at least one matching line.
#[test]
fn exact_metric_sample_rejects_numeric_prefix_lookalikes() {
    let metrics = "# TYPE cwl_pingora_gateway_request_errors_total counter\ncwl_pingora_gateway_request_errors_total 10\n";
    assert!(!contains_exact_metric_sample(
        metrics,
        "cwl_pingora_gateway_request_errors_total 1"
    ));

    let duplicated = "# TYPE cwl_pingora_gateway_request_errors_total counter\ncwl_pingora_gateway_request_errors_total 1\ncwl_pingora_gateway_request_errors_total 1\n";
    assert!(
        !contains_exact_metric_sample(
            duplicated,
            "cwl_pingora_gateway_request_errors_total 1"
        ),
        "duplicate exact samples must not satisfy the exactly-one error contract"
    );
}

/// Proves an origin RST after the response bytes were acknowledged by the peer
/// preserves the first downstream status/framing and spares sibling routing.
#[test]
fn compiled_pg_erd_post_commit_reset_preserves_committed_status_and_independent_routing() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/post-commit-reset HTTP/1.1\r\n"));

        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\npartial")
            .expect("committed backend response should be writable");

        // Waiting on downstream delivery formed a circular oracle under slower instrumented runs:
        // origin waited for downstream header forwarding while the intermediary could wait for
        // origin completion. Instead, require Linux transport evidence that every emitted header
        // and partial-body byte reached and was acknowledged by the peer TCP stack before the RST.
        wait_until_peer_acknowledged_response(&stream);
        reset_on_close(&stream);
        drop(stream);
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
    let _process = start_gateway(
        &config,
        gateway_reservation,
        metrics_reservation,
        gateway_address,
        metrics_address,
    );

    let (partial, termination) = raw_request_until_committed_then_reset(
        gateway_address,
        b"GET /api/post-commit-reset HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n",
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
    assert_eq!(
        exact_http_1_1_status_code(&headers),
        Some(200),
        "a reset after downstream commitment cannot be rewritten as a second status: {headers:?}"
    );
    assert_eq!(
        header_values(&headers, "Content-Length"),
        vec!["20"],
        "the committed response must retain exactly one declared framing field: {headers:?}"
    );
    let body = &partial[header_end..];
    assert!(
        b"partial".starts_with(body),
        "any body bytes that cross before the abort must be an exact prefix of origin bytes: {body:?}"
    );
    assert!(
        body.len() < 20,
        "fixture must reset before its declared response body completes"
    );

    let readiness = get(gateway_address, "/readyz");
    assert_eq!(
        exact_http_1_1_status_code(&readiness),
        Some(200),
        "one post-commit upstream reset must not poison process readiness: {readiness:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(
        contains_exact_metric_sample(&metrics, "cwl_pingora_gateway_request_errors_total 1"),
        "the post-commit reset must remain visible as exactly one low-cardinality error sample: {metrics:?}"
    );

    let recovered = get(gateway_address, "/after-post-commit-reset");
    assert_eq!(
        exact_http_1_1_status_code(&recovered),
        Some(200),
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
