//! Real-listener acceptance for RFC 9110 Max-Forwards on the pg-erd migration gateway.
//!
//! The fixture keeps intermediary control separate from product routing: zero-hop and malformed
//! TRACE/OPTIONS requests must terminate at the gateway, while a positive hop budget is decremented
//! before the characterized backend sees it. Ordinary methods retain Max-Forwards as application
//! metadata because RFC 9110 assigns the field's intermediary semantics only to TRACE and OPTIONS.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::runtime_policy::V1_TERMINATION_BUDGET_SECONDS;
use tempfile::NamedTempFile;

struct GatewayProcess(Child);

impl GatewayProcess {
    #[cfg(unix)]
    fn assert_graceful_shutdown(&mut self) {
        let pid = self.0.id().to_string();
        let signal = Command::new("kill")
            .args(["-TERM", pid.as_str()])
            .status()
            .expect("SIGTERM command should execute");
        assert!(signal.success(), "SIGTERM should reach gateway child");

        let deadline = Instant::now() + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
        loop {
            match self
                .0
                .try_wait()
                .expect("gateway process state should remain readable")
            {
                Some(status) => {
                    assert!(
                        status.success(),
                        "gateway must exit successfully after graceful SIGTERM: {status}"
                    );
                    return;
                }
                None => {
                    assert!(
                        Instant::now() < deadline,
                        "gateway did not complete graceful shutdown before the external termination budget"
                    );
                    thread::sleep(Duration::from_millis(25));
                }
            }
        }
    }
}

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(Some(_))) {
            return;
        }

        #[cfg(unix)]
        {
            let pid = self.0.id().to_string();
            if Command::new("kill")
                .args(["-TERM", pid.as_str()])
                .status()
                .is_ok()
            {
                let deadline = Instant::now() + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
                loop {
                    match self.0.try_wait() {
                        Ok(Some(_)) => return,
                        Ok(None) if Instant::now() < deadline => {
                            thread::sleep(Duration::from_millis(25));
                        }
                        Ok(None) | Err(_) => break,
                    }
                }
            }
        }

        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn reserve_loopback() -> TcpListener {
    TcpListener::bind("127.0.0.1:0").expect("loopback port should be reservable")
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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 64\nmax_in_flight_requests: 4\nupstream_keepalive_pool_size: 1\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 2000\n      write_ms: 2000\n      idle_ms: 5000"
    )
    .expect("migration config should be written");
    file
}

fn try_raw_request(address: SocketAddr, request: &[u8]) -> std::io::Result<String> {
    let mut downstream = TcpStream::connect_timeout(&address, Duration::from_millis(200))?;
    downstream.set_read_timeout(Some(Duration::from_secs(5)))?;
    downstream.set_write_timeout(Some(Duration::from_secs(5)))?;
    downstream.write_all(request)?;
    let mut response = String::new();
    downstream.read_to_string(&mut response)?;
    Ok(response)
}

fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    try_raw_request(address, request).expect("gateway request should complete")
}

fn wait_until_http_ready(address: SocketAddr, path: &str, process: &mut Child, service: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before {service} became HTTP-ready: {status}");
        }
        if let Ok(response) = try_raw_request(address, request.as_bytes()) {
            if response.starts_with("HTTP/1.1 200") {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "{service} did not become HTTP-ready within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn spawn_gateway(
    config: &NamedTempFile,
    traffic_reservation: TcpListener,
    metrics_reservation: TcpListener,
) -> GatewayProcess {
    // Reservations stay owned until the last moment before child bind, preventing a local process
    // from stealing either configured port between discovery and process activation.
    drop(traffic_reservation);
    drop(metrics_reservation);
    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    GatewayProcess(child)
}

fn read_request_headers(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("origin timeout should be configurable");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    while bytes.len() <= 64 * 1024 {
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable");
        assert!(read > 0, "gateway closed before request headers completed");
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8(bytes).expect("fixture request headers should be UTF-8");
        }
    }
    panic!("origin request headers exceeded fixture bound");
}

fn reply_ok(stream: &mut TcpStream) {
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        .expect("origin response should be writable");
}

#[test]
fn compiled_pg_erd_listener_applies_max_forwards_after_admission_and_before_origin_contact() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend
        .local_addr()
        .expect("frontend address should exist");
    let traffic_reservation = reserve_loopback();
    let gateway_address = traffic_reservation
        .local_addr()
        .expect("traffic reservation should expose an address");
    let metrics_reservation = reserve_loopback();
    let metrics_address = metrics_reservation
        .local_addr()
        .expect("metrics reservation should expose an address");
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );

    let backend_thread = thread::spawn(move || {
        let (mut decremented, _) = backend
            .accept()
            .expect("positive OPTIONS should reach backend");
        let request = read_request_headers(&mut decremented);
        assert!(
            request.starts_with("OPTIONS /api/max-forwards HTTP/1.1\r\n"),
            "a locally rejected request reached the backend before the positive control: {request:?}"
        );
        assert!(
            request.to_ascii_lowercase().contains("max-forwards: 1\r\n"),
            "gateway must decrement Max-Forwards before forwarding: {request:?}"
        );
        assert!(!request.to_ascii_lowercase().contains("max-forwards: 2\r\n"));
        reply_ok(&mut decremented);

        let (mut ordinary, _) = backend.accept().expect("ordinary GET should reach backend");
        let request = read_request_headers(&mut ordinary);
        assert!(
            request.starts_with("GET /api/plain HTTP/1.1\r\n"),
            "a locally rejected TRACE/OPTIONS request reached the backend: {request:?}"
        );
        assert!(
            request.to_ascii_lowercase().contains("max-forwards: 0\r\n"),
            "ordinary methods must retain Max-Forwards as non-control metadata: {request:?}"
        );
        reply_ok(&mut ordinary);
    });

    let mut process = spawn_gateway(&config, traffic_reservation, metrics_reservation);
    wait_until_http_ready(
        gateway_address,
        "/readyz",
        &mut process.0,
        "traffic listener",
    );
    wait_until_http_ready(
        metrics_address,
        "/metrics",
        &mut process.0,
        "metrics listener",
    );

    let zero = raw_request(
        gateway_address,
        b"OPTIONS /api/max-forwards HTTP/1.1\r\nHost: app.example\r\nMax-Forwards: 0\r\nConnection: close\r\n\r\n",
    );
    assert!(
        zero.starts_with("HTTP/1.1 501"),
        "zero Max-Forwards must terminate at the gateway without origin contact: {zero:?}"
    );
    let zero_lower = zero.to_ascii_lowercase();
    for field in [
        "x-content-type-options: nosniff",
        "x-frame-options: deny",
        "referrer-policy: no-referrer",
        "permissions-policy: geolocation=(), microphone=(), camera=()",
    ] {
        assert!(
            zero_lower.contains(field),
            "local 501 must retain characterized pg-erd response policy field {field:?}: {zero:?}"
        );
    }

    let oversized_malformed = raw_request(
        gateway_address,
        b"OPTIONS /api/max-forwards HTTP/1.1\r\nHost: app.example\r\nContent-Length: 65\r\nMax-Forwards: 1x\r\nConnection: close\r\n\r\n",
    );
    assert!(
        oversized_malformed.starts_with("HTTP/1.1 413"),
        "declared-body admission must precede malformed Max-Forwards classification: {oversized_malformed:?}"
    );

    let oversized_duplicate = raw_request(
        gateway_address,
        b"TRACE /api/max-forwards HTTP/1.1\r\nHost: app.example\r\nContent-Length: 65\r\nMax-Forwards: 2\r\nMax-Forwards: 1\r\nConnection: close\r\n\r\n",
    );
    assert!(
        oversized_duplicate.starts_with("HTTP/1.1 413"),
        "declared-body admission must precede duplicate Max-Forwards classification: {oversized_duplicate:?}"
    );

    let positive = raw_request(
        gateway_address,
        b"OPTIONS /api/max-forwards HTTP/1.1\r\nHost: app.example\r\nMax-Forwards: 2\r\nConnection: close\r\n\r\n",
    );
    assert!(
        positive.starts_with("HTTP/1.1 200"),
        "positive Max-Forwards should reach the characterized backend: {positive:?}"
    );

    let malformed = raw_request(
        gateway_address,
        b"TRACE /api/max-forwards HTTP/1.1\r\nHost: app.example\r\nMax-Forwards: 1x\r\nConnection: close\r\n\r\n",
    );
    assert!(
        malformed.starts_with("HTTP/1.1 400"),
        "malformed Max-Forwards must fail closed at the gateway: {malformed:?}"
    );

    let duplicate = raw_request(
        gateway_address,
        b"OPTIONS /api/max-forwards HTTP/1.1\r\nHost: app.example\r\nMax-Forwards: 2\r\nMax-Forwards: 1\r\nConnection: close\r\n\r\n",
    );
    assert!(
        duplicate.starts_with("HTTP/1.1 400"),
        "duplicate Max-Forwards must fail closed at the gateway: {duplicate:?}"
    );

    let ordinary = raw_request(
        gateway_address,
        b"GET /api/plain HTTP/1.1\r\nHost: app.example\r\nMax-Forwards: 0\r\nConnection: close\r\n\r\n",
    );
    assert!(ordinary.starts_with("HTTP/1.1 200"));

    backend_thread
        .join()
        .expect("bounded backend fixture should complete");

    #[cfg(unix)]
    process.assert_graceful_shutdown();
}
