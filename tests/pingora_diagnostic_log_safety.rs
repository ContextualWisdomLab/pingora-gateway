//! Compiled-process regression for dependency diagnostic logging at broad operator verbosity.
//!
//! A production operator may request broad diagnostics through `RUST_LOG`. The gateway's
//! payload-minimization invariant must still prevent request URI/header secrets from entering
//! process stderr through Pingora dependency diagnostics. The origin must receive the sentinels so
//! the test cannot pass by rejecting or stripping the request before proxy delivery.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

struct GatewayProcess {
    child: Option<Child>,
    stderr: NamedTempFile,
}

impl GatewayProcess {
    fn wait_until_stderr_contains(&mut self, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let captured = fs::read_to_string(self.stderr.path())
                .expect("gateway stderr capture should remain readable");
            if captured.contains(needle) {
                return;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .expect("gateway child should still be owned")
                .try_wait()
                .expect("gateway process state should be readable")
            {
                panic!("gateway exited before expected log {needle:?}: {status}; stderr={captured:?}");
            }
            assert!(
                Instant::now() < deadline,
                "gateway did not emit expected log {needle:?} within 10s; stderr={captured:?}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn capture_stderr(mut self) -> String {
        let mut child = self
            .child
            .take()
            .expect("gateway child should still be owned");
        child.kill().expect("gateway should be terminable after traffic");
        child
            .wait()
            .expect("gateway should terminate after traffic capture");
        fs::read_to_string(self.stderr.path()).expect("gateway log output should be UTF-8")
    }
}

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
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
    upstream: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: origin\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
    )
    .expect("generic config should be written");
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
fn broad_runtime_diagnostics_do_not_log_request_secrets() {
    let origin = TcpListener::bind("127.0.0.1:0").expect("origin fixture should bind");
    let origin_address = origin.local_addr().expect("origin address should exist");
    let origin_thread = thread::spawn(move || {
        let (mut stream, _) = origin.accept().expect("proxied request should reach origin");
        let request = read_request_headers(&mut stream);
        let lower = request.to_ascii_lowercase();
        assert!(lower.starts_with("get /diagnostic-secret?token=query-secret http/1.1\r\n"));
        assert!(lower.contains("host: host-secret.example\r\n"));
        assert!(lower.contains("authorization: bearer authorization-secret\r\n"));
        assert!(lower.contains("cookie: session=cookie-secret\r\n"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("origin response should be writable");
    });

    let listener = reserve_loopback();
    let metrics_listener = reserve_loopback();
    let config = write_config(listener, metrics_listener, origin_address);
    let stderr = NamedTempFile::new().expect("gateway stderr capture should be writable");
    let stderr_writer = stderr
        .reopen()
        .expect("gateway stderr capture should be reopenable for child");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .env("RUST_LOG", "trace")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_writer))
        .spawn()
        .expect("compiled gateway binary should start");
    wait_until_listening(listener, &mut child);
    wait_until_listening(metrics_listener, &mut child);
    let mut process = GatewayProcess {
        child: Some(child),
        stderr,
    };

    let mut downstream = TcpStream::connect(listener).expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    downstream
        .write_all(
            b"GET /diagnostic-secret?token=query-secret HTTP/1.1\r\nHost: host-secret.example\r\nAuthorization: Bearer authorization-secret\r\nCookie: session=cookie-secret\r\nConnection: close\r\n\r\n",
        )
        .expect("request should be writable");
    let mut response = String::new();
    downstream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    assert!(response.starts_with("HTTP/1.1 200"), "request should proxy: {response:?}");
    origin_thread
        .join()
        .expect("origin diagnostic fixture should complete");

    process.wait_until_stderr_contains("gateway_request status=200 outcome=ok request_body_bytes=0");
    let captured = process.capture_stderr();
    for forbidden in [
        "/diagnostic-secret",
        "query-secret",
        "host-secret.example",
        "authorization-secret",
        "cookie-secret",
    ] {
        assert!(
            !captured.contains(forbidden),
            "runtime diagnostics leaked request-sensitive material {forbidden:?}: {captured:?}"
        );
    }
}
