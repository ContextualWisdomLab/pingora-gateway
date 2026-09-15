//! Real-listener regression for the HTTP-to-HTTP gateway TRACE content boundary.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

        let deadline = Instant::now() + Duration::from_secs(5);
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
                        "gateway did not complete graceful shutdown within five seconds"
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
                let deadline = Instant::now() + Duration::from_secs(5);
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

fn reserve_loopback() -> (TcpListener, SocketAddr) {
    let reservation = TcpListener::bind("127.0.0.1:0").expect("loopback port should be reservable");
    let address = reservation
        .local_addr()
        .expect("loopback reservation should expose its address");
    (reservation, address)
}

fn write_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 4\nupstream_keepalive_pool_size: 2\nupstreams:\n  - name: fixture\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 500\n      total_connection_ms: 1000\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 2000"
    )
    .expect("gateway config should be written");
    file
}

fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let mut stream = TcpStream::connect(address).expect("gateway should accept loopback traffic");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream read timeout should be configurable");
    stream
        .write_all(request)
        .expect("downstream request should be writable");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    response
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
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            let response = raw_request(
                address,
                b"GET /readyz HTTP/1.1\r\nHost: gateway.test\r\nConnection: close\r\n\r\n",
            );
            if response.starts_with("HTTP/1.1 200") {
                return;
            }
        }
        assert!(Instant::now() < deadline, "gateway did not become ready");
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn trace_content_is_rejected_locally_before_origin_contact() {
    let upstream = TcpListener::bind("127.0.0.1:0").expect("fixture upstream should bind");
    let upstream_address = upstream
        .local_addr()
        .expect("fixture upstream should expose its address");
    upstream
        .set_nonblocking(true)
        .expect("fixture upstream should support bounded contact observation");
    let origin = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match upstream.accept() {
                Ok((mut stream, _peer)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .expect("origin read timeout should be configurable");
                    let mut request = [0_u8; 4096];
                    let _ = stream.read(&mut request);
                    stream
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                        .expect("diagnostic origin response should be writable");
                    return true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return false;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("origin contact observation failed: {error}"),
            }
        }
    });

    let (traffic_reservation, gateway_address) = reserve_loopback();
    let (metrics_reservation, metrics_address) = reserve_loopback();
    let config = write_config(gateway_address, metrics_address, upstream_address);

    drop(traffic_reservation);
    drop(metrics_reservation);
    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled gateway binary should start");
    let mut process = GatewayProcess(child);
    wait_until_ready(gateway_address, &mut process.0);

    let response = raw_request(
        gateway_address,
        b"TRACE /diagnostic HTTP/1.1\r\nHost: gateway.test\r\nContent-Length: 1\r\nConnection: close\r\n\r\nx",
    );
    assert!(
        response.starts_with("HTTP/1.1 400"),
        "an HTTP-to-HTTP gateway must not emit TRACE content on its inbound client hop: {response:?}"
    );
    assert!(
        !origin.join().expect("origin observer should terminate"),
        "TRACE content must fail closed before any origin connection"
    );

    #[cfg(unix)]
    process.assert_graceful_shutdown();
}
