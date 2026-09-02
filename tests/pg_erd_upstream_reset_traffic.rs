#![cfg(target_os = "linux")]

//! Real-listener upstream TCP reset acceptance for the dedicated pg-erd migration binary.
//!
//! This contract distinguishes an established origin connection that is reset before any response
//! header arrives from both connection refusal and orderly/partial response termination. It proves
//! the routed failure is bounded and observable without inventing retry/failover, poisoning process
//! readiness, or affecting the independent characterized route.

use core::ffi::c_void;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const SOL_SOCKET: i32 = 1;
const SO_LINGER: i32 = 13;

#[repr(C)]
struct Linger {
    onoff: i32,
    linger_seconds: i32,
}

unsafe extern "C" {
    fn setsockopt(
        socket: i32,
        level: i32,
        option_name: i32,
        option_value: *const c_void,
        option_len: u32,
    ) -> i32;
}

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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 5000\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
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
            panic!("gateway exited before accepting traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(Instant::now() < deadline, "gateway did not start within 10s");
        thread::sleep(Duration::from_millis(25));
    }
}

fn start_gateway(
    config: &NamedTempFile,
    gateway_address: SocketAddr,
    metrics_address: SocketAddr,
) -> GatewayProcess {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    wait_until_listening(gateway_address, &mut child);
    wait_until_listening(metrics_address, &mut child);
    GatewayProcess(child)
}

fn raw_request(address: SocketAddr, request: &[u8]) -> String {
    let mut downstream = TcpStream::connect(address).expect("gateway should accept traffic");
    downstream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("downstream timeout should be configurable");
    downstream
        .write_all(request)
        .expect("downstream request should be writable");
    let mut response = String::new();
    downstream
        .read_to_string(&mut response)
        .expect("gateway response should be readable");
    response
}

fn get(address: SocketAddr, path: &str) -> String {
    raw_request(
        address,
        format!("GET {path} HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )
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

fn reset_on_close(stream: &TcpStream) {
    let linger = Linger {
        onoff: 1,
        linger_seconds: 0,
    };
    // SAFETY: the file descriptor belongs to this live TcpStream, `linger` matches Linux's
    // `struct linger { int l_onoff; int l_linger; }`, and the pointer remains valid for the call.
    let result = unsafe {
        setsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_LINGER,
            (&linger as *const Linger).cast(),
            std::mem::size_of::<Linger>() as u32,
        )
    };
    assert_eq!(
        result,
        0,
        "Linux SO_LINGER(0) should be configurable for the reset fixture: {}",
        std::io::Error::last_os_error()
    );
}

#[test]
fn compiled_pg_erd_pre_header_reset_returns_502_and_preserves_independent_routing() {
    let backend = TcpListener::bind("127.0.0.1:0").expect("backend fixture should bind");
    let backend_address = backend.local_addr().expect("backend address should exist");
    let backend_origin = thread::spawn(move || {
        let (mut stream, _) = backend
            .accept()
            .expect("routed request should reach the characterized backend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /api/reset HTTP/1.1\r\n"));

        // An abortive close after the complete request has arrived yields a real TCP RST rather
        // than the orderly FIN used by the partial-response fixture.
        reset_on_close(&stream);
        drop(stream);
    });

    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend.local_addr().expect("frontend address should exist");
    let frontend_origin = thread::spawn(move || {
        let (mut stream, _) = frontend
            .accept()
            .expect("fallback request should reach the independent frontend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /after-reset HTTP/1.1\r\n"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nrecovered",
            )
            .expect("frontend recovery response should be writable");
    });

    let gateway_address = reserve_loopback();
    let metrics_address = reserve_loopback();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );
    let _process = start_gateway(&config, gateway_address, metrics_address);

    let started = Instant::now();
    let reset_response = get(gateway_address, "/api/reset");
    assert!(
        reset_response.starts_with("HTTP/1.1 502"),
        "a pre-header upstream reset must fail as gateway transport failure without failover: {reset_response:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "an explicit reset must not wait for the configured read inactivity budget"
    );

    let readiness = get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "one upstream reset must not poison process readiness: {readiness:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(
        metrics.contains("cwl_pingora_gateway_request_errors_total 1"),
        "the upstream reset must remain visible through low-cardinality error telemetry: {metrics:?}"
    );

    let recovered = get(gateway_address, "/after-reset");
    assert!(
        recovered.starts_with("HTTP/1.1 200"),
        "an independent characterized route must remain usable after an upstream reset: {recovered:?}"
    );
    assert!(recovered.ends_with("\r\n\r\nrecovered"));

    frontend_origin
        .join()
        .expect("frontend recovery fixture should complete");
    backend_origin
        .join()
        .expect("reset backend fixture should complete");
}
