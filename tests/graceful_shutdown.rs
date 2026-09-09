//! Unix process-level SIGTERM acceptance for the compiled gateway binary.
//!
//! The fixture holds one upstream response open, sends SIGTERM only after the request has reached
//! the upstream, then releases the response. The downstream request must complete during the
//! configured grace period and the process must terminate before the signal-relative external
//! hard-kill budget.

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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000"
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

fn wait_for_exit(process: &mut Child, deadline: Instant) -> std::process::ExitStatus {
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not terminate before the external hard-kill budget"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

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
        assert!(read > 0, "gateway closed before completing the response");
        response.extend_from_slice(&buffer[..read]);
        assert!(
            response.len() <= MAX_FIXTURE_EVIDENCE_BYTES,
            "fixture response exceeded the 64 KiB evidence ceiling"
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
            "gateway should forward the admitted in-flight response body exactly"
        );
        return response;
    }
}

#[test]
fn sigterm_drains_an_in_flight_request_before_process_exit() {
    let upstream_listener =
        TcpListener::bind("127.0.0.1:0").expect("fixture upstream should bind loopback");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("fixture upstream should expose its address");
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_gateway_config(gateway_address, metrics_address, upstream_address);

    let (request_seen_tx, request_seen_rx) = mpsc::channel();
    let (release_response_tx, release_response_rx) = mpsc::channel();
    let upstream = thread::spawn(move || {
        let (mut stream, _) = upstream_listener
            .accept()
            .expect("gateway should connect to fixture upstream");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("upstream request timeout should be configurable");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream
                .read(&mut buffer)
                .expect("upstream request should be readable");
            assert!(read > 0, "gateway closed upstream request prematurely");
            request.extend_from_slice(&buffer[..read]);
            assert!(
                request.len() <= MAX_FIXTURE_EVIDENCE_BYTES,
                "fixture request exceeded the 64 KiB evidence ceiling"
            );
        }
        request_seen_tx
            .send(())
            .expect("test should observe the in-flight request");
        release_response_rx
            .recv_timeout(Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS))
            .expect("test controller should release the held response before its hard watchdog");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\ndrained")
            .expect("held upstream response should be writable");
    });

    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled gateway binary should start");
    let mut process = GatewayProcess(child);
    wait_until_listening(gateway_address, &mut process.0);

    let downstream = thread::spawn(move || {
        let mut stream =
            TcpStream::connect(gateway_address).expect("gateway should accept downstream traffic");
        stream
            .write_all(b"GET /held HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
            .expect("downstream request should be writable");
        read_response_through_body(&mut stream, b"drained")
    });

    request_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("request should reach upstream before SIGTERM");

    let signal_sent_at = Instant::now();
    let termination_deadline = signal_sent_at + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.0.id().to_string()])
        .status()
        .expect("system kill command should send SIGTERM");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");

    release_response_tx
        .send(())
        .expect("held upstream response should be released");
    let response = downstream
        .join()
        .expect("downstream request thread should complete");
    assert!(
        response.starts_with(b"HTTP/1.1 200 "),
        "in-flight request should complete during graceful drain: {:?}",
        String::from_utf8_lossy(&response)
    );
    assert!(
        signal_sent_at.elapsed() < Duration::from_secs(V1_GRACE_PERIOD_SECONDS + 1),
        "in-flight response should complete during the configured grace period"
    );

    upstream.join().expect("upstream fixture should complete");
    let exit_status = wait_for_exit(&mut process.0, termination_deadline);
    assert!(
        exit_status.success(),
        "SIGTERM graceful shutdown should exit successfully: {exit_status}"
    );
}
