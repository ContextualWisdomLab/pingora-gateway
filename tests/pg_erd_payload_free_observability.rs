//! Real-listener payload-free logging acceptance for the dedicated pg-erd migration binary.
//!
//! The shared observability bounded context promises that request paths, query strings, headers,
//! cookies, credentials, customer payloads, and product identifiers never enter its access-log
//! vocabulary. This contract proves that boundary through the compiled migration process while
//! sensitive request material is actually present on the proxied request path.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const MAX_ORIGIN_REQUEST_HEADER_BYTES: usize = 64 * 1024;

struct GatewayProcess {
    child: Option<Child>,
    stderr: NamedTempFile,
}

impl GatewayProcess {
    fn wait_until_stderr_contains(&mut self, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let captured = fs::read_to_string(self.stderr.path())
                .expect("gateway stderr capture should remain readable");
            if captured.contains(needle) {
                return;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .expect("gateway child should still be owned")
                .try_wait()
                .expect("gateway process state should be readable")
            {
                panic!(
                    "gateway exited before expected log {needle:?}: {status}; stderr={captured:?}"
                );
            }
            assert!(
                Instant::now() < deadline,
                "gateway did not emit expected log {needle:?} within 10s; stderr={captured:?}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn capture_stderr(mut self) -> String {
        let mut child = self
            .child
            .take()
            .expect("gateway child should still be owned");
        child
            .kill()
            .expect("gateway should be terminable after traffic");
        child
            .wait()
            .expect("gateway should terminate after traffic capture");
        fs::read_to_string(self.stderr.path()).expect("gateway log output should be UTF-8")
    }
}

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Reserves both process listeners at once so sequential bind-and-drop cannot
/// reuse the first ephemeral port and manufacture an Admin Config collision.
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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
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
        assert!(
            Instant::now() < deadline,
            "gateway did not start within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn start_gateway(
    config: &NamedTempFile,
    gateway_address: SocketAddr,
    metrics_address: SocketAddr,
) -> GatewayProcess {
    let stderr = NamedTempFile::new().expect("gateway stderr capture should be writable");
    let stderr_writer = stderr
        .reopen()
        .expect("gateway stderr capture should be reopenable for the child");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "cwl_pingora_gateway::observability=info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_writer))
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    wait_until_listening(gateway_address, &mut child);
    wait_until_listening(metrics_address, &mut child);
    GatewayProcess {
        child: Some(child),
        stderr,
    }
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

/// Reads one origin-side request header block under a finite timeout and byte
/// budget so a broken forwarding path fails deterministically instead of hanging CI.
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

/// Selects exact HTTP field names case-insensitively while rejecting lookalike
/// fields such as `X-Forwarded-Host` that contain `Host` only as a suffix.
fn header_values<'a>(request: &'a str, name: &str) -> Vec<&'a str> {
    request
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

/// Requires a complete Prometheus sample line so a value such as `10` cannot
/// satisfy an oracle that expects the exact counter value `1`.
fn contains_exact_metric_sample(metrics: &str, sample: &str) -> bool {
    metrics
        .lines()
        .any(|line| line.trim_end_matches('\r') == sample)
}

#[test]
fn exact_header_matching_rejects_forwarded_host_lookalikes() {
    let request = "GET / HTTP/1.1\r\nX-Forwarded-Host: tenant-secret.example:8080\r\nhOsT: expected.example\r\n\r\n";
    assert_eq!(header_values(request, "Host"), vec!["expected.example"]);
}

#[test]
fn exact_metric_sample_rejects_numeric_prefix_lookalikes() {
    let metrics = "# TYPE cwl_pingora_gateway_requests_total counter\ncwl_pingora_gateway_requests_total 10\n";
    assert!(!contains_exact_metric_sample(
        metrics,
        "cwl_pingora_gateway_requests_total 1"
    ));
}

#[test]
fn compiled_pg_erd_shared_access_log_excludes_request_sensitive_material() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        assert!(
            request
                .to_ascii_lowercase()
                .starts_with("get /api/log-contract?customer=query-secret http/1.1\r\n")
        );
        assert_eq!(
            header_values(&request, "Host"),
            vec!["tenant-secret.example:8080"]
        );
        assert_eq!(
            header_values(&request, "Authorization"),
            vec!["Bearer authorization-secret"]
        );
        assert_eq!(
            header_values(&request, "Cookie"),
            vec!["session=cookie-secret"]
        );
        assert_eq!(
            header_values(&request, "X-Product-Context"),
            vec!["product-secret"]
        );
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("backend response should be writable");
    });

    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend
        .local_addr()
        .expect("frontend address should exist");

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
    let mut process = start_gateway(&config, gateway_address, metrics_address);

    let response = raw_request(
        gateway_address,
        b"GET /api/log-contract?customer=query-secret HTTP/1.1\r\nHost: tenant-secret.example:8080\r\nAuthorization: Bearer authorization-secret\r\nCookie: session=cookie-secret\r\nX-Product-Context: product-secret\r\nConnection: close\r\n\r\n",
    );
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "sensitive-material fixture request should proxy successfully: {response:?}"
    );
    backend_origin
        .join()
        .expect("backend sensitive-material fixture should complete");

    let metrics = raw_request(
        metrics_address,
        b"GET /metrics HTTP/1.1\r\nHost: metrics\r\nConnection: close\r\n\r\n",
    );
    assert!(
        contains_exact_metric_sample(&metrics, "cwl_pingora_gateway_requests_total 1"),
        "metrics scrape should prove exactly one proxied request reached shared completion recording: {metrics:?}"
    );
    process
        .wait_until_stderr_contains("gateway_request status=200 outcome=ok request_body_bytes=0");

    let stderr = process.capture_stderr();
    let request_logs: Vec<_> = stderr
        .lines()
        .filter(|line| line.contains("gateway_request"))
        .collect();
    assert_eq!(
        request_logs.len(),
        1,
        "the shared observability target should emit one completion record: {stderr:?}"
    );
    let access_log = request_logs[0];
    assert!(
        access_log.contains("gateway_request status=200 outcome=ok request_body_bytes=0"),
        "shared access logging should contain only bounded transport facts: {access_log:?}"
    );

    for forbidden in [
        "/api/log-contract",
        "query-secret",
        "tenant-secret.example",
        "authorization-secret",
        "cookie-secret",
        "product-secret",
    ] {
        assert!(
            !stderr.contains(forbidden),
            "shared observability target leaked request-sensitive material {forbidden:?}: {stderr:?}"
        );
    }
}
