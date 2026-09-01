//! End-to-end characterization through the compiled production binary and a real local upstream.
//!
//! The fixture binds only loopback sockets and therefore does not require external network access.
//! It proves that validated configuration reaches Pingora's serving path rather than stopping at a
//! unit-test-only adapter boundary.

use std::io::{Read, Write};
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

fn reserve_distinct_loopback_addresses() -> (SocketAddr, SocketAddr) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    let addresses = (
        traffic
            .local_addr()
            .expect("traffic reservation has an address"),
        metrics
            .local_addr()
            .expect("metrics reservation has an address"),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

fn write_gateway_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000"
    )
    .expect("gateway config should be written");
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

fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let mut downstream =
        TcpStream::connect(address).expect("gateway should accept downstream traffic");
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
        format!("GET {path} HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
}

#[test]
fn compiled_gateway_enforces_health_limits_forwarding_proxy_and_telemetry_paths() {
    let upstream_listener =
        TcpListener::bind("127.0.0.1:0").expect("fixture upstream should bind loopback");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("fixture upstream should expose its address");
    let fixture_listener = upstream_listener
        .try_clone()
        .expect("fixture listener should be clonable so the upstream stays available for streaming-limit characterization");
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_gateway_config(gateway_address, metrics_address, upstream_address);

    let fixture = thread::spawn(move || {
        let (mut stream, _) = fixture_listener
            .accept()
            .expect("gateway should connect to fixture upstream");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream
                .read(&mut buffer)
                .expect("request should be readable");
            assert!(read > 0, "gateway closed upstream request prematurely");
            request.extend_from_slice(&buffer[..read]);
        }
        let request = String::from_utf8_lossy(&request);
        assert!(
            request.starts_with("GET /through-pingora HTTP/1.1\r\n"),
            "unexpected upstream request: {request:?}"
        );
        let lowered = request.to_ascii_lowercase();
        assert!(lowered.contains("forwarded: proto=http\r\n"));
        assert!(!lowered.contains("x-forwarded-for:"));
        assert!(!lowered.contains("x-real-ip:"));
        assert!(!lowered.contains("for=203.0.113.7"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\npingora-path",
            )
            .expect("fixture response should be writable");
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled gateway binary should start");

    wait_until_listening(gateway_address, &mut child);
    wait_until_listening(metrics_address, &mut child);
    let mut process = GatewayProcess(child);

    for health_path in ["/livez", "/readyz"] {
        let response = get(gateway_address, health_path);
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "health endpoint {health_path} failed: {response:?}"
        );
        assert!(response
            .to_ascii_lowercase()
            .contains("cache-control: no-store"));
    }

    let oversized = raw_request(
        gateway_address,
        b"POST /too-large HTTP/1.1\r\nHost: gateway.test\r\nContent-Length: 9\r\nConnection: close\r\n\r\n123456789",
    );
    assert!(
        oversized.starts_with("HTTP/1.1 413"),
        "oversized request should fail before upstream selection: {oversized:?}"
    );

    let response = raw_request(
        gateway_address,
        b"GET /through-pingora HTTP/1.1\r\nHost: gateway.test\r\nAuthorization: Bearer super-secret-token\r\nCookie: session=super-secret-cookie\r\nForwarded: for=203.0.113.7;proto=https\r\nX-Forwarded-For: 203.0.113.7\r\nX-Real-IP: 203.0.113.7\r\nConnection: close\r\n\r\n",
    );
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected downstream response: {response:?}"
    );
    assert!(response.ends_with("\r\n\r\npingora-path"));

    fixture.join().expect("upstream fixture should complete");

    let chunked_oversized = raw_request(
        gateway_address,
        b"POST /too-large-chunked HTTP/1.1\r\nHost: gateway.test\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n9\r\n123456789\r\n0\r\n\r\n",
    );
    assert!(
        chunked_oversized.starts_with("HTTP/1.1 413"),
        "streamed request without Content-Length must still enforce the configured body limit: {chunked_oversized:?}"
    );

    let recovered = get(gateway_address, "/readyz");
    assert!(
        recovered.starts_with("HTTP/1.1 200"),
        "gateway must remain ready after rejecting an oversized streamed body: {recovered:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(metrics.starts_with("HTTP/1.1 200"));
    assert!(metrics.contains("cwl_pingora_gateway_requests_total"));
    assert!(metrics.contains("cwl_pingora_gateway_request_errors_total"));
    assert!(metrics.contains("cwl_pingora_gateway_request_body_bytes_total"));
    assert!(!metrics.contains("super-secret-token"));
    assert!(!metrics.contains("super-secret-cookie"));

    let mut stderr = process
        .0
        .stderr
        .take()
        .expect("gateway stderr should remain captured");
    process
        .0
        .kill()
        .expect("gateway process should still be running");
    process
        .0
        .wait()
        .expect("terminated gateway process should be reapable");
    let mut logs = String::new();
    stderr
        .read_to_string(&mut logs)
        .expect("gateway logs should be readable");
    assert!(logs.contains("gateway_request"));
    assert!(!logs.contains("super-secret-token"));
    assert!(!logs.contains("super-secret-cookie"));
}
