//! Real-boundary regression cases for HTTP intermediary and forwarding authority edge cases.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::forwarding_policy::{DownstreamScheme, ForwardingContext};
use cwl_pingora_gateway::runtime_policy::V1_TERMINATION_BUDGET_SECONDS;
use pingora::prelude::{ErrorType, RequestHeader};
use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;
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
        assert!(
            signal.success(),
            "SIGTERM should be delivered to gateway child"
        );

        let deadline = Instant::now() + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
        loop {
            match self
                .0
                .try_wait()
                .expect("gateway process state should remain readable during graceful shutdown")
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
                        "gateway did not complete graceful shutdown before the external termination budget"
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
            // Pingora handles SIGTERM through its graceful shutdown path. Give the process the
            // runtime's external termination budget so `Server::run()` can complete and LLVM
            // coverage/profile state can flush; only fall back to SIGKILL if that bounded drain
            // fails. The test explicitly validates a successful graceful exit; Drop is cleanup-only
            // so it remains safe during unwinding.
            let pid = self.0.id().to_string();
            if Command::new("kill")
                .args(["-TERM", pid.as_str()])
                .status()
                .is_ok()
            {
                let deadline = Instant::now() + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
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

fn request_with_host(host: &str) -> RequestHeader {
    let mut request =
        RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
    request
        .insert_header("Host", host)
        .expect("fixture Host field bytes must be valid");
    request
}

#[test]
fn incomplete_ipvfuture_and_empty_reg_name_fail_closed_through_forwarding_boundary() {
    let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));

    for authority in ["[v1]", ":80"] {
        let request = request_with_host(authority);
        let error = ForwardingContext::from_downstream_transport(
            Some(&client),
            &request,
            &request,
            DownstreamScheme::Http,
        )
        .expect_err("malformed Host authority must fail before becoming forwarding identity");
        assert_eq!(error.etype, ErrorType::HTTPStatus(400), "{authority}");
    }
}

#[test]
fn ipvfuture_suffix_character_classes_are_enforced_through_forwarding_boundary() {
    let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));
    let valid = request_with_host("[vF.a:b!c]:9443");
    let context = ForwardingContext::from_downstream_transport(
        Some(&client),
        &valid,
        &valid,
        DownstreamScheme::Https,
    )
    .expect("IPvFuture suffix may contain RFC 3986 sub-delims and colon");
    let mut emitted = valid.clone();
    context
        .apply(&mut emitted)
        .expect("validated transport authority must remain representable as forwarding metadata");
    assert_eq!(emitted.headers["x-forwarded-host"], "[vF.a:b!c]:9443");
    assert_eq!(emitted.headers["x-forwarded-port"], "9443");

    let invalid = request_with_host("[vF.a/b]:9443");
    let error = ForwardingContext::from_downstream_transport(
        Some(&client),
        &invalid,
        &invalid,
        DownstreamScheme::Https,
    )
    .expect_err(
        "IPvFuture suffix must reject characters outside unreserved, sub-delims, and colon",
    );
    assert_eq!(error.etype, ErrorType::HTTPStatus(400));
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
fn zero_max_forwards_is_a_local_final_recipient_and_never_contacts_origin() {
    let upstream = TcpListener::bind("127.0.0.1:0").expect("fixture upstream should bind");
    let upstream_address = upstream
        .local_addr()
        .expect("fixture upstream should expose its address");
    let (traffic_reservation, gateway_address) = reserve_loopback();
    let (metrics_reservation, metrics_address) = reserve_loopback();
    assert_ne!(gateway_address, metrics_address);
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
        b"OPTIONS /intermediary-control HTTP/1.1\r\nHost: gateway.test\r\nMax-Forwards: 0\r\nConnection: close\r\n\r\n",
    );
    assert!(
        response.starts_with("HTTP/1.1 501"),
        "zero-hop OPTIONS must terminate at this intermediary: {response:?}"
    );
    assert!(
        response
            .to_ascii_lowercase()
            .contains("cache-control: no-store\r\n"),
        "local final-recipient response must remain non-cacheable: {response:?}"
    );

    upstream
        .set_nonblocking(true)
        .expect("fixture upstream should support a nonblocking no-contact assertion");
    let error = upstream
        .accept()
        .expect_err("zero-hop intermediary control must not establish an origin connection");
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);

    #[cfg(unix)]
    process.assert_graceful_shutdown();
}
