//! Executable RED for the missing downstream HTTP/1 whole-request-header lifetime.
//!
//! Pingora's current downstream HTTP/1 read timeout is a per-read inactivity budget. These fixtures
//! keep an incomplete header making progress well inside that inactivity budget and require a
//! separate monotonic whole-header deadline. They exercise both a fresh connection and a sequential
//! keep-alive reuse. This gateway does not opt into Pingora HTTP/1 pipelining, so no pipelined-prefix
//! acceptance is claimed on this composition root.

#![cfg(unix)]

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const DESIRED_WHOLE_HEADER_BUDGET: Duration = Duration::from_millis(1_500);
const ENFORCEMENT_GRACE: Duration = Duration::from_millis(500);
const OUTER_EVIDENCE_BOUND: Duration = Duration::from_millis(2_500);
const DRIP_INTERVAL: Duration = Duration::from_millis(100);
const PROBE_TIMEOUT: Duration = Duration::from_millis(25);
const RESPONSE_BOUND: usize = 8 * 1024;
const ATTACKER_MARKER: &str = "h1-lifetime-secret-marker";

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Drains trace-level gateway stderr while the child is alive so the OS pipe cannot block shutdown.
fn drain_gateway_stderr(process: &mut Child) -> thread::JoinHandle<String> {
    let mut stderr = process
        .stderr
        .take()
        .expect("gateway stderr should remain captured");
    thread::spawn(move || {
        let mut logs = String::new();
        stderr
            .read_to_string(&mut logs)
            .expect("gateway logs should be readable");
        logs
    })
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

fn raw_request(address: SocketAddr, request: &[u8]) -> Vec<u8> {
    let mut stream = TcpStream::connect(address).expect("gateway should accept downstream traffic");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    stream
        .write_all(request)
        .expect("downstream request should be writable");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("gateway response should be readable");
    response
}

fn assert_ready(address: SocketAddr) {
    let response = raw_request(
        address,
        b"GET /readyz HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n",
    );
    assert!(
        response.starts_with(b"HTTP/1.1 200 "),
        "gateway readiness should remain healthy: {:?}",
        String::from_utf8_lossy(&response)
    );
}

fn parse_http11_status(response: &[u8]) -> u16 {
    let line_end = response
        .windows(2)
        .position(|window| window == b"\r\n")
        .expect("response should contain a complete HTTP status line");
    let line = std::str::from_utf8(&response[..line_end]).expect("status line should be UTF-8");
    let mut parts = line.split(' ');
    assert_eq!(parts.next(), Some("HTTP/1.1"), "unexpected protocol token");
    let status = parts.next().expect("status code should be present");
    assert_eq!(status.len(), 3, "status code must be exactly three digits");
    assert!(
        status.bytes().all(|byte| byte.is_ascii_digit()),
        "status code must be decimal"
    );
    status.parse().expect("three decimal digits should parse")
}

fn read_one_response(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("response timeout should be configurable");
    let mut response = Vec::new();
    let header_end = loop {
        assert!(
            response.len() < RESPONSE_BOUND,
            "response header exceeded bound"
        );
        let mut byte = [0_u8; 1];
        let read = stream
            .read(&mut byte)
            .expect("response header should be readable");
        assert!(
            read > 0,
            "connection closed before response header completed"
        );
        response.push(byte[0]);
        if response.ends_with(b"\r\n\r\n") {
            break response.len();
        }
    };
    let headers =
        std::str::from_utf8(&response[..header_end]).expect("response header should be UTF-8");
    let content_length = headers
        .split("\r\n")
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("valid Content-Length"))
        })
        .expect("fixture response should declare Content-Length");
    let body_start = response.len();
    response.resize(body_start + content_length, 0);
    stream
        .read_exact(&mut response[body_start..])
        .expect("declared response body should be readable");
    response
}

fn probe_header_lifetime_enforcement(stream: &mut TcpStream) -> bool {
    stream
        .set_read_timeout(Some(PROBE_TIMEOUT))
        .expect("probe timeout should be configurable");
    let mut first = [0_u8; 256];
    match stream.read(&mut first) {
        Ok(0) => true,
        Ok(read) => {
            let mut response = first[..read].to_vec();
            let deadline = Instant::now() + Duration::from_millis(250);
            while !response.windows(2).any(|window| window == b"\r\n") {
                assert!(
                    response.len() < RESPONSE_BOUND,
                    "timeout response exceeded bound"
                );
                assert!(
                    Instant::now() < deadline,
                    "timeout response did not complete its status line"
                );
                let mut buffer = [0_u8; 256];
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => response.extend_from_slice(&buffer[..read]),
                    Err(error)
                        if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                    {
                        continue;
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::ConnectionReset
                                | ErrorKind::ConnectionAborted
                                | ErrorKind::BrokenPipe
                        ) =>
                    {
                        return true;
                    }
                    Err(error) => panic!("unexpected timeout-response read error: {error}"),
                }
            }
            let status = parse_http11_status(&response);
            assert!(
                (400..600).contains(&status),
                "incomplete-header enforcement must not manufacture success: {}",
                String::from_utf8_lossy(&response)
            );
            true
        }
        Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => false,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe
            ) =>
        {
            true
        }
        Err(error) => panic!("unexpected slow-header probe error: {error}"),
    }
}

fn require_monotonic_header_deadline<F>(
    stream: &mut TcpStream,
    started_at: Instant,
    mut assert_no_application_progress: F,
) where
    F: FnMut(),
{
    loop {
        assert_no_application_progress();
        if probe_header_lifetime_enforcement(stream) {
            let elapsed = started_at.elapsed();
            assert!(
                elapsed <= DESIRED_WHOLE_HEADER_BUDGET + ENFORCEMENT_GRACE,
                "incomplete header was terminated too late: {elapsed:?}"
            );
            return;
        }

        let elapsed = started_at.elapsed();
        assert!(
            elapsed < OUTER_EVIDENCE_BOUND,
            "supplier kept an incomplete HTTP/1 header alive for {elapsed:?}, beyond the desired {:?} whole-header budget, while every partial write made progress inside the per-read inactivity window",
            DESIRED_WHOLE_HEADER_BUDGET
        );

        thread::sleep(DRIP_INTERVAL);
        match stream.write_all(b"a") {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::ConnectionReset
                        | ErrorKind::ConnectionAborted
                        | ErrorKind::BrokenPipe
                        | ErrorKind::NotConnected
                ) =>
            {
                let elapsed = started_at.elapsed();
                assert!(
                    elapsed <= DESIRED_WHOLE_HEADER_BUDGET + ENFORCEMENT_GRACE,
                    "incomplete header write failed only after the acceptance window: {elapsed:?}"
                );
                return;
            }
            Err(error) => panic!("unexpected slow-header write error: {error}"),
        }
    }
}

fn assert_no_upstream_connection(listener: &TcpListener) {
    match listener.accept() {
        Err(error) if error.kind() == ErrorKind::WouldBlock => {}
        Err(error) => panic!("unexpected upstream accept error: {error}"),
        Ok((_stream, peer)) => {
            panic!("incomplete header reached upstream unexpectedly from {peer}")
        }
    }
}

fn read_origin_request_header(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("origin read timeout should be configurable");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return request;
        }
        assert!(
            request.len() < 64 * 1024,
            "origin request header exceeded 64 KiB"
        );
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable");
        assert!(
            read > 0,
            "gateway closed origin request before header completion"
        );
        request.extend_from_slice(&buffer[..read]);
    }
}

fn terminate_gateway(process: &mut Child) {
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.id().to_string()])
        .status()
        .expect("system kill command should send SIGTERM");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");
    let exit_status = process
        .wait()
        .expect("gracefully terminated gateway should be reapable");
    assert!(
        exit_status.success(),
        "SIGTERM graceful shutdown should exit successfully: {exit_status}"
    );
}

#[test]
fn fresh_h1_slow_drip_is_terminated_by_whole_header_budget() {
    let upstream_listener =
        TcpListener::bind("127.0.0.1:0").expect("fixture upstream should bind loopback");
    upstream_listener
        .set_nonblocking(true)
        .expect("upstream observation should be nonblocking");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("fixture upstream should expose its address");
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_gateway_config(gateway_address, metrics_address, upstream_address);

    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "trace")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled gateway binary should start");
    let mut process = GatewayProcess(child);
    let log_drain = drain_gateway_stderr(&mut process.0);
    wait_until_listening(gateway_address, &mut process.0);
    wait_until_listening(metrics_address, &mut process.0);
    assert_ready(gateway_address);

    let mut slow = TcpStream::connect(gateway_address).expect("slow client should connect");
    slow.set_write_timeout(Some(Duration::from_secs(1)))
        .expect("slow-client write timeout should be configurable");
    let prefix = format!(
        "GET /slow-header HTTP/1.1\r\nHost: gateway.test\r\nX-Slow-Drip: {ATTACKER_MARKER}"
    );
    let started_at = Instant::now();
    slow.write_all(prefix.as_bytes())
        .expect("incomplete request prefix should be writable");

    require_monotonic_header_deadline(&mut slow, started_at, || {
        assert_no_upstream_connection(&upstream_listener);
    });
    assert_no_upstream_connection(&upstream_listener);
    assert_ready(gateway_address);

    terminate_gateway(&mut process.0);
    let logs = log_drain
        .join()
        .expect("gateway stderr drain should complete");
    assert!(
        !logs.contains(ATTACKER_MARKER),
        "attacker-controlled incomplete header content must not enter process logs"
    );
}

#[test]
fn reused_keepalive_slow_drip_is_terminated_by_whole_header_budget() {
    let upstream_listener =
        TcpListener::bind("127.0.0.1:0").expect("fixture upstream should bind loopback");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("fixture upstream should expose its address");
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_gateway_config(gateway_address, metrics_address, upstream_address);

    let (release_origin_tx, release_origin_rx) = mpsc::channel();
    let (unexpected_origin_tx, unexpected_origin_rx) = mpsc::channel();
    let origin = thread::spawn(move || {
        let (mut stream, _) = upstream_listener
            .accept()
            .expect("first complete request should reach the fixture origin");
        let request = read_origin_request_header(&mut stream);
        assert!(
            request.starts_with(b"GET /first HTTP/1.1\r\n"),
            "unexpected first origin request: {:?}",
            String::from_utf8_lossy(&request)
        );
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nfirst")
            .expect("first origin response should be writable");
        stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .expect("origin post-response probe timeout should be configurable");
        upstream_listener
            .set_nonblocking(true)
            .expect("second origin-connection observation should be nonblocking");
        let mut first_connection_open = true;
        loop {
            match release_origin_rx.try_recv() {
                Ok(()) | Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {}
            }

            if first_connection_open {
                let mut byte = [0_u8; 1];
                match stream.read(&mut byte) {
                    Ok(0) => first_connection_open = false,
                    Ok(_) => {
                        let _ = unexpected_origin_tx.send(());
                        break;
                    }
                    Err(error)
                        if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                        ) =>
                    {
                        first_connection_open = false;
                    }
                    Err(error) => panic!("unexpected kept-alive origin read error: {error}"),
                }
            }

            match upstream_listener.accept() {
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => panic!("unexpected second origin accept error: {error}"),
                Ok((_second, _peer)) => {
                    let _ = unexpected_origin_tx.send(());
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "trace")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled gateway binary should start");
    let mut process = GatewayProcess(child);
    let log_drain = drain_gateway_stderr(&mut process.0);
    wait_until_listening(gateway_address, &mut process.0);
    wait_until_listening(metrics_address, &mut process.0);
    assert_ready(gateway_address);

    let mut downstream =
        TcpStream::connect(gateway_address).expect("keep-alive client should connect");
    downstream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .expect("keep-alive-client write timeout should be configurable");
    downstream
        .write_all(b"GET /first HTTP/1.1\r\nHost: gateway.test\r\n\r\n")
        .expect("first complete keep-alive request should be writable");

    let first_response = read_one_response(&mut downstream);
    assert_eq!(
        parse_http11_status(&first_response),
        200,
        "first request must complete before the reused-connection timeout assertion"
    );
    assert!(
        first_response.ends_with(b"\r\n\r\nfirst"),
        "first response body should be exact: {:?}",
        String::from_utf8_lossy(&first_response)
    );

    let prefix = format!(
        "GET /slow-header HTTP/1.1\r\nHost: gateway.test\r\nX-Slow-Drip: {ATTACKER_MARKER}"
    );
    let second_header_started_at = Instant::now();
    downstream
        .write_all(prefix.as_bytes())
        .expect("incomplete second request prefix should be writable");

    require_monotonic_header_deadline(&mut downstream, second_header_started_at, || {
        match unexpected_origin_rx.try_recv() {
            Err(mpsc::TryRecvError::Empty) => {}
            Ok(()) => panic!("incomplete second request reached an origin connection"),
            Err(mpsc::TryRecvError::Disconnected) => {
                panic!("origin monitor disconnected before the timeout assertion completed")
            }
        }
    });
    match unexpected_origin_rx.try_recv() {
        Err(mpsc::TryRecvError::Empty) => {}
        Ok(()) => panic!("incomplete second request reached an origin connection"),
        Err(mpsc::TryRecvError::Disconnected) => {
            panic!("origin monitor disconnected before the timeout assertion completed")
        }
    }
    assert_ready(gateway_address);

    let _ = release_origin_tx.send(());
    origin.join().expect("origin fixture should complete");

    terminate_gateway(&mut process.0);
    let logs = log_drain
        .join()
        .expect("gateway stderr drain should complete");
    assert!(
        !logs.contains(ATTACKER_MARKER),
        "reused attacker-controlled header content must not enter process logs"
    );
}
