//! Real-listener regression coverage for the process-health privilege boundary.
//!
//! Only payload-free `GET`/`HEAD` requests to `/livez` and `/readyz` are privileged process probes.
//! Requests that reuse those paths with a body or an unsupported method must not inherit that
//! bypass contract.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
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

fn reserve_loopback() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback port should be reservable");
    let address = listener
        .local_addr()
        .expect("loopback reservation should expose an address");
    (listener, address)
}

fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let mut stream = TcpStream::connect(address).expect("gateway should accept loopback traffic");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("response timeout should be configurable");
    stream
        .write_all(request)
        .expect("request should be writable");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("response should be readable");
    response
}

fn probe_readyz(address: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) else {
        return false;
    };
    if stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .is_err()
        || stream
            .set_write_timeout(Some(Duration::from_millis(250)))
            .is_err()
    {
        return false;
    }
    if stream
        .write_all(b"GET /readyz HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }

    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok() && response.starts_with("HTTP/1.1 200")
}

fn wait_until_ready(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before readiness: {status}");
        }
        if probe_readyz(address) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not become ready within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn write_generic_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary generic config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 1\nupstream_keepalive_pool_size: 1\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 2000"
    )
    .expect("generic config should be written");
    file
}

fn write_pg_erd_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary migration config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 1\nupstream_keepalive_pool_size: 1\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 2000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 2000"
    )
    .expect("migration config should be written");
    file
}

fn spawn_gateway(binary: &str, config: &NamedTempFile, listener: SocketAddr) -> GatewayProcess {
    let mut child = Command::new(binary)
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled gateway binary should start");
    wait_until_ready(listener, &mut child);
    GatewayProcess(child)
}

fn assert_probe_boundary(binary: &str, config: &NamedTempFile, listener: SocketAddr) {
    let _process = spawn_gateway(binary, config, listener);

    for path in ["/livez", "/readyz"] {
        let valid_get = raw_request(
            listener,
            format!("GET {path} HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        );
        assert!(
            valid_get.starts_with("HTTP/1.1 200"),
            "payload-free GET process probe must remain privileged: {valid_get:?}"
        );

        let valid_head = raw_request(
            listener,
            format!("HEAD {path} HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        );
        assert!(
            valid_head.starts_with("HTTP/1.1 200"),
            "payload-free HEAD process probe must follow GET semantics: {valid_head:?}"
        );

        let body_bearing = raw_request(
            listener,
            format!(
                "GET {path} HTTP/1.1\r\nHost: gateway.test\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx"
            )
            .as_bytes(),
        );
        assert!(
            body_bearing.starts_with("HTTP/1.1 413"),
            "body-bearing health-path traffic must not receive the payload-free probe bypass: {body_bearing:?}"
        );

        let unsupported_method = raw_request(
            listener,
            format!("DELETE {path} HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        );
        assert!(
            unsupported_method.starts_with("HTTP/1.1 405"),
            "unsupported methods must not receive the process-probe contract: {unsupported_method:?}"
        );
        assert!(
            unsupported_method
                .to_ascii_lowercase()
                .contains("allow: get, head\r\n"),
            "method rejection must advertise both mandatory retrieval methods"
        );
    }
}

fn assert_invalid_health_shapes_obey_saturation(
    binary: &str,
    config: &NamedTempFile,
    listener: SocketAddr,
    upstream_listener: TcpListener,
    held_path: &'static str,
) {
    let (request_seen_tx, request_seen_rx) = mpsc::channel();
    let (release_response_tx, release_response_rx) = mpsc::channel();
    let fixture = thread::spawn(move || {
        let (mut upstream, _) = upstream_listener
            .accept()
            .expect("held application request should connect upstream");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = upstream
                .read(&mut buffer)
                .expect("held upstream request should be readable");
            assert!(read > 0, "held request must complete its headers");
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        assert!(
            String::from_utf8_lossy(&request).starts_with(&format!("GET {held_path} HTTP/1.1\r\n")),
            "unexpected held application request: {:?}",
            String::from_utf8_lossy(&request)
        );
        request_seen_tx
            .send(())
            .expect("test should observe the held application request");
        release_response_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("test should release held capacity");
        upstream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nheld")
            .expect("held response should be writable");
    });

    let _process = spawn_gateway(binary, config, listener);
    let held = thread::spawn(move || {
        raw_request(
            listener,
            format!("GET {held_path} HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
    });
    request_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("application request should hold the single admission lease");

    let body_bearing = raw_request(
        listener,
        b"GET /readyz HTTP/1.1\r\nHost: gateway.test\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx",
    );
    assert!(
        body_bearing.starts_with("HTTP/1.1 503"),
        "body-bearing health-path traffic must use application admission under saturation: {body_bearing:?}"
    );

    let unsupported_method = raw_request(
        listener,
        b"DELETE /livez HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n",
    );
    assert!(
        unsupported_method.starts_with("HTTP/1.1 503"),
        "unsupported health-path traffic must use application admission under saturation: {unsupported_method:?}"
    );

    for method in ["GET", "HEAD"] {
        let valid_probe = raw_request(
            listener,
            format!("{method} /readyz HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        );
        assert!(
            valid_probe.starts_with("HTTP/1.1 200"),
            "payload-free {method} process probe must remain observable under saturation: {valid_probe:?}"
        );
    }

    release_response_tx
        .send(())
        .expect("held application response should be released");
    let held_response = held
        .join()
        .expect("held downstream request should complete");
    assert!(held_response.starts_with("HTTP/1.1 200"));
    fixture
        .join()
        .expect("held-capacity fixture should complete");
}

#[test]
fn generic_listener_limits_process_health_bypass_to_payload_free_retrieval() {
    let (traffic_reservation, traffic) = reserve_loopback();
    let (metrics_reservation, metrics) = reserve_loopback();
    let (upstream_reservation, upstream) = reserve_loopback();
    let config = write_generic_config(traffic, metrics, upstream);

    drop(traffic_reservation);
    drop(metrics_reservation);
    assert_probe_boundary(env!("CARGO_BIN_EXE_cwl-pingora-gateway"), &config, traffic);
    drop(upstream_reservation);
}

#[test]
fn pg_erd_listener_limits_process_health_bypass_to_payload_free_retrieval() {
    let (traffic_reservation, traffic) = reserve_loopback();
    let (metrics_reservation, metrics) = reserve_loopback();
    let (backend_reservation, backend) = reserve_loopback();
    let (frontend_reservation, frontend) = reserve_loopback();
    let config = write_pg_erd_config(traffic, metrics, backend, frontend);

    drop(traffic_reservation);
    drop(metrics_reservation);
    assert_probe_boundary(
        env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"),
        &config,
        traffic,
    );
    drop(backend_reservation);
    drop(frontend_reservation);
}

#[test]
fn generic_invalid_health_shapes_do_not_bypass_saturated_admission() {
    let (traffic_reservation, traffic) = reserve_loopback();
    let (metrics_reservation, metrics) = reserve_loopback();
    let (upstream_listener, upstream) = reserve_loopback();
    let config = write_generic_config(traffic, metrics, upstream);

    drop(traffic_reservation);
    drop(metrics_reservation);
    assert_invalid_health_shapes_obey_saturation(
        env!("CARGO_BIN_EXE_cwl-pingora-gateway"),
        &config,
        traffic,
        upstream_listener,
        "/held-capacity",
    );
}

#[test]
fn pg_erd_invalid_health_shapes_do_not_bypass_saturated_admission() {
    let (traffic_reservation, traffic) = reserve_loopback();
    let (metrics_reservation, metrics) = reserve_loopback();
    let (backend_listener, backend) = reserve_loopback();
    let (frontend_reservation, frontend) = reserve_loopback();
    let config = write_pg_erd_config(traffic, metrics, backend, frontend);

    drop(traffic_reservation);
    drop(metrics_reservation);
    assert_invalid_health_shapes_obey_saturation(
        env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"),
        &config,
        traffic,
        backend_listener,
        "/api/held-capacity",
    );
    drop(frontend_reservation);
}
