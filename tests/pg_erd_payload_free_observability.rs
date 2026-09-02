//! Real-listener payload-free logging acceptance for the dedicated pg-erd migration binary.
//!
//! The shared observability bounded context promises that request paths, query strings, headers,
//! cookies, credentials, customer payloads, and product identifiers never enter its access-log
//! vocabulary. This contract proves that boundary through the compiled migration process while
//! sensitive request material is actually present on the proxied request path.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

struct GatewayProcess(Option<Child>);

impl GatewayProcess {
    fn capture_stderr(mut self) -> String {
        let mut child = self.0.take().expect("gateway child should still be owned");
        child.kill().expect("gateway should be terminable after traffic");
        let output = child
            .wait_with_output()
            .expect("gateway output should be collectable after termination");
        String::from_utf8(output.stderr).expect("gateway log output should be UTF-8")
    }
}

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
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
        .env("RUST_LOG", "cwl_pingora_gateway::observability=info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    wait_until_listening(gateway_address, &mut child);
    wait_until_listening(metrics_address, &mut child);
    GatewayProcess(Some(child))
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
fn compiled_pg_erd_shared_access_log_excludes_request_sensitive_material() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        let lower = request.to_ascii_lowercase();
        assert!(lower.starts_with(
            "get /api/log-contract?customer=query-secret http/1.1\r\n"
        ));
        assert!(lower.contains("host: tenant-secret.example:8080\r\n"));
        assert!(lower.contains("authorization: bearer authorization-secret\r\n"));
        assert!(lower.contains("cookie: session=cookie-secret\r\n"));
        assert!(lower.contains("x-product-context: product-secret\r\n"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("backend response should be writable");
    });

    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");

    let gateway_address = reserve_loopback();
    let metrics_address = reserve_loopback();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );
    let process = start_gateway(&config, gateway_address, metrics_address);

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
        metrics.contains("cwl_pingora_gateway_requests_total 1"),
        "metrics scrape should prove the proxied request reached shared completion recording before log capture: {metrics:?}"
    );

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
