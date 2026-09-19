//! Real-listener upstream-failure acceptance for the dedicated pg-erd migration binary.
//!
//! This contract proves one transport failure class through the compiled migration process without
//! importing product-domain behavior: a refused characterized backend connection fails closed,
//! process health remains available, error telemetry is emitted, and an independent fallback route
//! can still complete through the frontend authority.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use std::ffi::{c_int, c_void};
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use tempfile::NamedTempFile;

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct LoopbackReservation(TcpListener);

impl LoopbackReservation {
    fn bind() -> Self {
        Self(TcpListener::bind("127.0.0.1:0").expect("loopback port should be reservable"))
    }

    fn address(&self) -> SocketAddr {
        self.0
            .local_addr()
            .expect("reservation should expose an address")
    }
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct LinuxSockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn socket(domain: c_int, socket_type: c_int, protocol: c_int) -> c_int;
    fn bind(socket_fd: c_int, address: *const c_void, address_len: u32) -> c_int;
}

/// Holds an IPv4/TCP port bound without putting the socket into LISTEN state.
///
/// On Linux this keeps another process from claiming the characterized endpoint while causing
/// connection attempts to receive `ECONNREFUSED`.
#[cfg(target_os = "linux")]
struct RefusedTcpReservation {
    _socket: OwnedFd,
}

#[cfg(target_os = "linux")]
impl RefusedTcpReservation {
    fn bind(address: SocketAddr) -> Self {
        const AF_INET: c_int = 2;
        const SOCK_STREAM: c_int = 1;

        let SocketAddr::V4(address) = address else {
            panic!("refusal fixture requires an IPv4 loopback address");
        };

        // SAFETY: `socket` is called with Linux AF_INET/SOCK_STREAM constants and returns either a
        // fresh owned descriptor or -1. The descriptor is immediately wrapped in `OwnedFd`.
        let raw_fd = unsafe { socket(AF_INET, SOCK_STREAM, 0) };
        assert!(
            raw_fd >= 0,
            "refusal fixture socket creation failed: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: `raw_fd` was just returned successfully by `socket` and has no other Rust owner.
        let socket_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        let raw_address = LinuxSockAddrIn {
            sin_family: AF_INET as u16,
            sin_port: address.port().to_be(),
            sin_addr: u32::from_ne_bytes(address.ip().octets()),
            sin_zero: [0; 8],
        };

        // SAFETY: `raw_address` is a live C-compatible IPv4 sockaddr for the duration of the call;
        // the descriptor remains owned by `socket_fd`. A failed bind is a setup failure, never a
        // fallback to an unreserved free port.
        let bind_result = unsafe {
            bind(
                socket_fd.as_raw_fd(),
                (&raw_address as *const LinuxSockAddrIn).cast::<c_void>(),
                std::mem::size_of::<LinuxSockAddrIn>() as u32,
            )
        };
        assert_eq!(
            bind_result,
            0,
            "refusal fixture could not exclusively bind {address}: {}",
            std::io::Error::last_os_error()
        );

        Self { _socket: socket_fd }
    }
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
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 200\n      total_connection_ms: 400\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
    )
    .expect("migration config should be written");
    file
}

fn try_http_get(address: SocketAddr, path: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(100))?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: probe.invalid\r\nConnection: close\r\n\r\n"
    )?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn wait_until_http_ready(address: SocketAddr, path: &str, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before accepting traffic: {status}");
        }

        if try_http_get(address, path).is_ok_and(|response| response.starts_with("HTTP/1.1 200")) {
            return;
        }

        assert!(
            Instant::now() < deadline,
            "gateway did not expose {path} within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn start_gateway(
    config: &NamedTempFile,
    gateway_reservation: LoopbackReservation,
    metrics_reservation: LoopbackReservation,
) -> GatewayProcess {
    let gateway_address = gateway_reservation.address();
    let metrics_address = metrics_reservation.address();

    // The configured addresses stay exclusively reserved through configuration construction and
    // command preparation. Release them only at the child-bind handoff.
    drop(gateway_reservation);
    drop(metrics_reservation);

    let mut child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
        .args(["--config", config.path().to_str().expect("UTF-8 temp path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled pg-erd migration binary should start");
    wait_until_http_ready(gateway_address, "/readyz", &mut child);
    wait_until_http_ready(metrics_address, "/metrics", &mut child);
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
    const MAX_HEADER_BYTES: usize = 64 * 1024;
    let deadline = Instant::now() + Duration::from_secs(5);
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .expect("origin read timeout should be configurable");

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        assert!(
            Instant::now() < deadline,
            "origin request headers did not complete within 5s"
        );
        let read = match stream.read(&mut buffer) {
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(error) => panic!("origin request should be readable: {error}"),
        };
        assert!(
            read > 0,
            "gateway closed origin request before headers completed"
        );
        bytes.extend_from_slice(&buffer[..read]);
        assert!(
            bytes.len() <= MAX_HEADER_BYTES,
            "origin request headers exceeded 64 KiB fixture bound"
        );
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn compiled_pg_erd_refused_backend_fails_bounded_and_preserves_independent_routing() {
    let backend_reservation = LoopbackReservation::bind();
    let backend_address = backend_reservation.address();

    let frontend = TcpListener::bind("127.0.0.1:0").expect("frontend fixture should bind");
    let frontend_address = frontend
        .local_addr()
        .expect("frontend address should exist");
    let frontend_origin = thread::spawn(move || {
        let (mut stream, _) = frontend
            .accept()
            .expect("fallback request should reach the independent frontend authority");
        let request = read_request_headers(&mut stream);
        assert!(request.starts_with("GET /after-backend-failure HTTP/1.1\r\n"));
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nrecovered",
            )
            .expect("frontend recovery response should be writable");
    });

    let gateway_reservation = LoopbackReservation::bind();
    let gateway_address = gateway_reservation.address();
    let metrics_reservation = LoopbackReservation::bind();
    let metrics_address = metrics_reservation.address();
    let config = write_config(
        gateway_address,
        metrics_address,
        backend_address,
        frontend_address,
    );

    // Transfer the configured backend from a listening reservation to a bound, non-listening TCP
    // socket before the gateway starts. If another process wins the tiny handoff window, setup fails
    // here rather than producing false refusal evidence.
    drop(backend_reservation);
    let _refused_backend = RefusedTcpReservation::bind(backend_address);
    let direct_refusal = TcpStream::connect_timeout(&backend_address, Duration::from_millis(100))
        .expect_err("bound non-listening backend must reject direct TCP connection attempts");
    assert_eq!(
        direct_refusal.kind(),
        std::io::ErrorKind::ConnectionRefused,
        "fixture must prove the configured endpoint is deterministically refusing TCP connections"
    );

    let _process = start_gateway(&config, gateway_reservation, metrics_reservation);

    let started = Instant::now();
    let failed = get(gateway_address, "/api/unavailable");
    let failure_elapsed = started.elapsed();
    assert!(
        failed.starts_with("HTTP/1.1 502"),
        "a refused characterized backend must fail as Bad Gateway: {failed:?}"
    );
    assert!(
        failure_elapsed < Duration::from_secs(1),
        "loopback refusal must stay within a conservative one-second envelope around the configured 200/400 ms connection budgets; elapsed={failure_elapsed:?}"
    );

    let readiness = get(gateway_address, "/readyz");
    assert!(
        readiness.starts_with("HTTP/1.1 200"),
        "one upstream transport failure must not poison process readiness: {readiness:?}"
    );

    let metrics = get(metrics_address, "/metrics");
    assert!(
        metrics
            .lines()
            .any(|line| line == "cwl_pingora_gateway_request_errors_total 1"),
        "the refused upstream must expose exactly one request error through low-cardinality telemetry: {metrics:?}"
    );

    let recovered = get(gateway_address, "/after-backend-failure");
    assert!(
        recovered.starts_with("HTTP/1.1 200"),
        "an independent characterized route must remain usable after backend failure: {recovered:?}"
    );
    assert!(recovered.ends_with("\r\n\r\nrecovered"));

    frontend_origin
        .join()
        .expect("frontend recovery fixture should complete");
}
