//! Routed SIGTERM drain acceptance for the dedicated pg-erd migration binary.
//!
//! The fixture holds one characterized backend response open, sends SIGTERM only after the routed
//! request reaches that backend, then releases the response during the shared grace period. The
//! downstream request must complete and the migration process must exit inside the external
//! termination budget. This proves the bounded composition root consumes the shared runtime drain
//! policy without transferring generic-binary evidence.

#![cfg(unix)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::runtime_policy::{V1_GRACE_PERIOD_SECONDS, V1_TERMINATION_BUDGET_SECONDS};
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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000"
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
            panic!("migration process exited before accepting traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "migration process did not start within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_exit(process: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("migration process state should be readable")
        {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "migration process did not terminate before the external hard-kill budget"
        );
        thread::sleep(Duration::from_millis(25));
    }
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
fn sigterm_drains_routed_pg_erd_request_before_process_exit() {
    let backend_listener =
        TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind loopback");
    let backend_address = backend_listener
        .local_addr()
        .expect("backend fixture should expose its address");
    let frontend_listener =
        TcpListener::bind("127.0.0.1:0").expect("frontend authority should bind loopback");
    let frontend_address = frontend_listener
        .local_addr()
        .expect("frontend authority should expose its address");
    let gateway_address = reserve_loopback();
    let metrics_address = reserve_loopback();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );

    let (request_seen_tx, request_seen_rx) = mpsc::channel();
    let (release_response_tx, release_response_rx) = mpsc::channel();
    let backend = thread::spawn(move || {
        let (mut stream, _) = backend_listener
            .accept()
            .expect("routed request should reach the characterized backend");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/held HTTP/1.1\r\n"));
        request_seen_tx
            .send(())
            .expect("test should observe the routed in-flight request");
        release_response_rx
            .recv_timeout(Duration::from_secs(V1_GRACE_PERIOD_SECONDS))
            .expect("test should release the held response during the grace period");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\ndrained")
            .expect("held backend response should be writable");
    });

    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    let mut process = GatewayProcess(child);
    wait_until_listening(gateway_address, &mut process.0);

    let downstream = thread::spawn(move || {
        let mut stream = TcpStream::connect(gateway_address)
            .expect("migration gateway should accept downstream traffic");
        stream
            .set_read_timeout(Some(Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS)))
            .expect("downstream timeout should be configurable");
        stream
            .write_all(
                b"GET /api/held HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n",
            )
            .expect("downstream request should be writable");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("drained downstream response should be readable");
        response
    });

    request_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("routed request should reach backend before SIGTERM");

    let signal_sent_at = Instant::now();
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.0.id().to_string()])
        .status()
        .expect("system kill command should send SIGTERM");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");

    release_response_tx
        .send(())
        .expect("held backend response should be released");
    let response = downstream
        .join()
        .expect("downstream request thread should complete");
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "routed in-flight request should complete during graceful drain: {response:?}"
    );
    assert!(response.ends_with("\r\n\r\ndrained"));
    assert!(
        signal_sent_at.elapsed() < Duration::from_secs(V1_GRACE_PERIOD_SECONDS + 1),
        "routed response should complete during the configured grace period"
    );

    backend.join().expect("backend fixture should complete");
    let exit_status = wait_for_exit(&mut process.0);
    assert!(
        exit_status.success(),
        "SIGTERM graceful shutdown should exit successfully: {exit_status}"
    );

    drop(frontend_listener);
}
