//! End-to-end characterization through the compiled production binary and a real local upstream.
//!
//! The fixture binds only loopback sockets and therefore does not require external network access.
//! It proves that validated configuration reaches Pingora's serving path rather than stopping at a
//! unit-test-only adapter boundary.

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

fn reserve_loopback_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback port should be available");
    listener
        .local_addr()
        .expect("bound loopback listener has an address")
}

fn write_gateway_config(listener: SocketAddr, upstream: SocketAddr) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000"
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
        assert!(Instant::now() < deadline, "gateway did not start within 10s");
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn compiled_gateway_proxies_http_through_pingora() {
    let upstream_listener =
        TcpListener::bind("127.0.0.1:0").expect("fixture upstream should bind loopback");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("fixture upstream should expose its address");
    let gateway_address = reserve_loopback_address();
    let config = write_gateway_config(gateway_address, upstream_address);

    let fixture = thread::spawn(move || {
        let (mut stream, _) = upstream_listener
            .accept()
            .expect("gateway should connect to fixture upstream");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).expect("request should be readable");
            assert!(read > 0, "gateway closed upstream request prematurely");
            request.extend_from_slice(&buffer[..read]);
        }
        let request = String::from_utf8_lossy(&request);
        assert!(
            request.starts_with("GET /through-pingora HTTP/1.1\r\n"),
            "unexpected upstream request: {request:?}"
        );
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\npingora-path",
            )
            .expect("fixture response should be writable");
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled gateway binary should start");

    wait_until_listening(gateway_address, &mut child);
    let mut process = GatewayProcess(child);

    let mut downstream =
        TcpStream::connect(gateway_address).expect("gateway should accept downstream traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    downstream
        .write_all(b"GET /through-pingora HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n")
        .expect("downstream request should be writable");

    let mut response = String::new();
    downstream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");

    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected downstream response: {response:?}"
    );
    assert!(response.ends_with("\r\n\r\npingora-path"));

    fixture.join().expect("upstream fixture should complete");
    process.0.kill().expect("gateway process should still be running");
}
