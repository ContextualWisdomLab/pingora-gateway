//! Unix process-level SIGTERM acceptance for the compiled gateway binary.
//!
//! The fixture holds one upstream response open, sends SIGTERM only after the request has reached
//! the upstream, then releases the response. The downstream request must complete before Pingora's
//! configured grace period expires and the process must terminate without a forced kill.

#![cfg(unix)]

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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000"
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

fn wait_for_exit(process: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not terminate within the bounded shutdown window"
        );
        thread::sleep(Duration::from_millis(25));
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
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream
                .read(&mut buffer)
                .expect("upstream request should be readable");
            assert!(read > 0, "gateway closed upstream request prematurely");
            request.extend_from_slice(&buffer[..read]);
        }
        request_seen_tx
            .send(())
            .expect("test should observe the in-flight request");
        release_response_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("test should release the held response");
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
            .set_read_timeout(Some(Duration::from_secs(8)))
            .expect("downstream timeout should be configurable");
        stream
            .write_all(b"GET /held HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
            .expect("downstream request should be writable");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("drained downstream response should be readable");
        response
    });

    request_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("request should reach upstream before SIGTERM");

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
        response.starts_with("HTTP/1.1 200"),
        "in-flight request should complete during graceful drain: {response:?}"
    );
    assert!(response.ends_with("\r\n\r\ndrained"));

    upstream.join().expect("upstream fixture should complete");
    let exit_status = wait_for_exit(&mut process.0);
    assert!(
        exit_status.success(),
        "SIGTERM graceful shutdown should exit successfully: {exit_status}"
    );
}
