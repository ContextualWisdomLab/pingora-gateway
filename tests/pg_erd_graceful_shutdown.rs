//! Routed SIGTERM drain acceptance for the dedicated pg-erd migration binary.
//!
//! The fixture holds one characterized backend response open, sends SIGTERM only after the routed
//! request reaches that backend, then releases the response during the shared grace period. The
//! downstream request must complete and the migration process must exit inside the signal-relative
//! external termination budget. This keeps bounded-root evidence independent of the generic binary.

#![cfg(unix)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::runtime_policy::{V1_GRACE_PERIOD_SECONDS, V1_TERMINATION_BUDGET_SECONDS};
use tempfile::NamedTempFile;

const MAX_FIXTURE_EVIDENCE_BYTES: usize = 64 * 1024;

/// Owns the migration child so every assertion path terminates and reaps the spawned process.
struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    /// Forces teardown when a drain assertion fails before the child reaches its normal SIGTERM exit.
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Selects traffic and metrics authorities while both ephemeral loopback reservations remain held.
fn reserve_distinct_loopback_addresses() -> (SocketAddr, SocketAddr) {
    // Hold both ephemeral reservations at once so the kernel cannot hand the just-released traffic
    // port back to the metrics reservation and manufacture an invalid listener-authority config.
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be reservable");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    let addresses = (
        traffic
            .local_addr()
            .expect("traffic reservation should expose an address"),
        metrics
            .local_addr()
            .expect("metrics reservation should expose an address"),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

/// Writes the bounded pg-erd fixture with read budgets longer than the shared graceful-drain window.
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

/// Waits for the traffic listener without allowing an early process exit to look like startup success.
fn wait_until_listening(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("migration process state should be readable")
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

/// Waits only until the SIGTERM-relative external deadline so downstream work cannot reset the budget.
fn wait_for_exit(process: &mut Child, deadline: Instant) -> std::process::ExitStatus {
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

/// Reads through the origin header terminator with finite socket and evidence bounds.
fn read_request_headers(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("origin request timeout should be configurable");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable before its fixture timeout");
        assert!(
            read > 0,
            "gateway closed origin request before headers completed"
        );
        bytes.extend_from_slice(&buffer[..read]);
        assert!(
            bytes.len() <= MAX_FIXTURE_EVIDENCE_BYTES,
            "origin request exceeded the 64 KiB evidence ceiling"
        );
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

/// Reads exactly far enough to prove the admitted response body completed without waiting for EOF.
fn read_response_through_body(stream: &mut TcpStream, expected_body: &[u8]) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(V1_GRACE_PERIOD_SECONDS + 1)))
        .expect("downstream response timeout should be configurable");

    let mut response = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("downstream response should arrive before its fixture timeout");
        assert!(
            read > 0,
            "migration gateway closed before completing the response"
        );
        response.extend_from_slice(&buffer[..read]);
        assert!(
            response.len() <= MAX_FIXTURE_EVIDENCE_BYTES,
            "downstream response exceeded the 64 KiB evidence ceiling"
        );

        let Some(header_end) = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
        else {
            continue;
        };

        if response.len() < header_end + expected_body.len() {
            continue;
        }

        assert_eq!(
            &response[header_end..header_end + expected_body.len()],
            expected_body,
            "migration gateway should forward the admitted response body exactly"
        );
        return response;
    }
}

/// Proves an admitted pg-erd request drains to completion and the process exits inside one SIGTERM budget.
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
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
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
            .recv_timeout(Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS))
            .expect("test controller should release the held response before its hard watchdog");
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
            .write_all(
                b"GET /api/held HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n",
            )
            .expect("downstream request should be writable");
        read_response_through_body(&mut stream, b"drained")
    });

    request_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("routed request should reach backend before SIGTERM");

    let signal_sent_at = Instant::now();
    let termination_deadline = signal_sent_at + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
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
        response.starts_with(b"HTTP/1.1 200 "),
        "routed in-flight request should complete during graceful drain: {:?}",
        String::from_utf8_lossy(&response)
    );
    assert!(
        signal_sent_at.elapsed() < Duration::from_secs(V1_GRACE_PERIOD_SECONDS + 1),
        "routed response should complete during the configured grace period"
    );

    backend.join().expect("backend fixture should complete");
    let exit_status = wait_for_exit(&mut process.0, termination_deadline);
    assert!(
        exit_status.success(),
        "SIGTERM graceful shutdown should exit successfully: {exit_status}"
    );

    drop(frontend_listener);
}
