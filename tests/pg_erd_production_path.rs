//! End-to-end contract for the dedicated pg-erd migration binary.
//!
//! The fixture uses only loopback sockets. It proves that the bounded Admin Config reaches the
//! compiled Pingora listener and preserves the characterized route/header/forwarding boundary
//! without making product authentication or arbitrary destinations part of the gateway.

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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000"
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

fn read_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("origin request should be readable");
        assert!(read > 0, "gateway closed origin request before headers completed");
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8(bytes).expect("fixture request headers should be UTF-8");
        }
    }
}

fn serve_origin(listener: TcpListener, expected_paths: &'static [&'static str], body: &'static str) {
    for expected_path in expected_paths {
        let (mut stream, _) = listener.accept().expect("gateway should reach origin");
        let request = read_request(&mut stream);
        assert!(
            request.starts_with(&format!("GET {expected_path} HTTP/1.1\r\n")),
            "unexpected routed request: {request:?}"
        );
        let lowered = request.to_ascii_lowercase();
        assert!(!lowered.contains("forwarded:"));
        assert!(lowered.contains("x-forwarded-for: 127.0.0.1\r\n"));
        assert!(lowered.contains("x-real-ip: 127.0.0.1\r\n"));
        assert!(lowered.contains("x-forwarded-host: app.example:8080\r\n"));
        assert!(lowered.contains("x-forwarded-proto: http\r\n"));
        assert!(!lowered.contains("x-forwarded-server:"));
        assert!(!lowered.contains("attacker.example"));
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nX-Frame-Options: SAMEORIGIN\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("origin response should be writable");
    }
}

fn assert_characterized_response_headers(response: &str) {
    let lowered = response.to_ascii_lowercase();
    for expected in [
        "x-content-type-options: nosniff\r\n",
        "x-frame-options: deny\r\n",
        "referrer-policy: no-referrer\r\n",
        "permissions-policy: geolocation=(), microphone=(), camera=()\r\n",
    ] {
        assert!(lowered.contains(expected), "missing characterized field {expected:?}: {response:?}");
    }
    assert!(!lowered.contains("x-frame-options: sameorigin"));
}

#[test]
fn compiled_pg_erd_listener_preserves_health_route_header_and_forwarding_boundaries() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");
    let gateway_address = reserve_loopback();
    let metrics_address = reserve_loopback();
    assert_ne!(gateway_address, metrics_address);

    let backend_thread = thread::spawn(move || serve_origin(backend, &["/healthz", "/apiary"], "backend"));
    let frontend_thread = thread::spawn(move || serve_origin(frontend, &["/projects/42"], "frontend"));
    let config = write_config(gateway_address, metrics_address, backend_address, frontend_address);

    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    wait_until_listening(gateway_address, &mut child);
    wait_until_listening(metrics_address, &mut child);
    let _process = GatewayProcess(child);

    for health_path in ["/livez", "/readyz"] {
        let response = get(gateway_address, health_path);
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "process health endpoint {health_path} must not fall through to a product origin: {response:?}"
        );
        assert!(response.to_ascii_lowercase().contains("cache-control: no-store\r\n"));
    }

    for (path, expected_body) in [
        ("/healthz", "backend"),
        ("/apiary", "backend"),
        ("/projects/42", "frontend"),
    ] {
        let hostile = format!(
            "GET {path} HTTP/1.1\r\nHost: app.example:8080\r\nForwarded: for=203.0.113.7;proto=https\r\nX-Forwarded-For: 203.0.113.7\r\nX-Forwarded-Host: attacker.example\r\nX-Forwarded-Proto: https\r\nX-Forwarded-Server: attacker-proxy\r\nX-Real-IP: 203.0.113.7\r\nConnection: close\r\n\r\n"
        );
        let response = raw_request(gateway_address, hostile.as_bytes());
        assert!(response.starts_with("HTTP/1.1 200"), "routed request failed: {response:?}");
        assert!(response.ends_with(expected_body));
        assert_characterized_response_headers(&response);
    }

    let oversize = raw_request(
        gateway_address,
        b"POST /api HTTP/1.1\r\nHost: app.example:8080\r\nContent-Length: 9\r\nConnection: close\r\n\r\n123456789",
    );
    assert!(
        oversize.starts_with("HTTP/1.1 413"),
        "declared body limit must fail before origin delivery: {oversize:?}"
    );

    backend_thread.join().expect("backend fixture should complete");
    frontend_thread.join().expect("frontend fixture should complete");
}
