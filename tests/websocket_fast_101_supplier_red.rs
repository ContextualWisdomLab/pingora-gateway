//! Real-wire supplier RED for the Pingora 0.9.0 fast-101 WebSocket teardown race.
//!
//! Production gateway versions intentionally continue to deny HTTP/1 Upgrade. This fixture uses a
//! test-only Pingora proxy with the supplier's standards-oriented upgrade policy so the released
//! supplier behavior can be characterized without adding a gateway WebSocket capability or copying
//! the upstream repair. The request-body callback is delayed to deterministically force the ordering
//! documented by cloudflare/pingora#946/#947: the upstream 101 is observed before the queued
//! end-of-request-body event.

use std::env;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use cwl_pingora_gateway::runtime_policy::build_server_conf;
use pingora::prelude::{http_proxy_service, sleep, HttpPeer, ProxyHttp, ResponseHeader, Server, Session};
use pingora::server::RunArgs;
use pingora::upstreams::peer::{HttpUpstreamRequestPolicy, ALPN};

const CHILD_MODE_ENV: &str = "CWL_WEBSOCKET_SUPPLIER_CHILD";
const CHILD_LISTENER_ENV: &str = "CWL_WEBSOCKET_SUPPLIER_LISTENER";
const CHILD_ORIGIN_ENV: &str = "CWL_WEBSOCKET_SUPPLIER_ORIGIN";
const FAST_101_BODY_DELAY: Duration = Duration::from_millis(200);
const POST_101_SETTLE: Duration = Duration::from_millis(300);
const IO_DEADLINE: Duration = Duration::from_secs(5);
const MAX_HEADER_BYTES: usize = 64 * 1024;
const TUNNEL_PAYLOAD: &[u8] = b"cwl-fast-101";

#[derive(Clone)]
struct SupplierUpgradeProxy {
    origin: SocketAddr,
}

#[async_trait]
impl ProxyHttp for SupplierUpgradeProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        if session.req_header().uri.path() != "/readyz" {
            return Ok(false);
        }

        let mut response = ResponseHeader::build(200, None)
            .expect("literal readiness response header must be valid");
        response
            .insert_header("Content-Length", "0")
            .expect("literal Content-Length must be valid");
        response
            .insert_header("Cache-Control", "no-store")
            .expect("literal Cache-Control must be valid");
        session
            .write_response_header(Box::new(response), true)
            .await?;
        Ok(true)
    }

    async fn request_body_filter(
        &self,
        session: &mut Session,
        _body: &mut Option<Bytes>,
        _end_of_stream: bool,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        if session
            .req_header()
            .headers
            .contains_key("x-cwl-delay-request-body")
        {
            // This is test-only ordering control, mirroring the upstream #947 reproducer. It is
            // deliberately not a production timeout or WebSocket policy.
            sleep(FAST_101_BODY_DELAY).await;
        }
        Ok(())
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let mut peer = HttpPeer::new(self.origin, false, String::new());
        peer.options.alpn = ALPN::H1;
        peer.options.http_upstream_request_policy = HttpUpstreamRequestPolicy::standard();
        Ok(Box::new(peer))
    }
}

struct ChildProcess(Child);

impl Drop for ChildProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn read_header(stream: &mut TcpStream, context: &str) -> Vec<u8> {
    let deadline = Instant::now() + IO_DEADLINE;
    let mut header = Vec::new();
    let mut buffer = [0_u8; 1024];

    loop {
        let now = Instant::now();
        assert!(now < deadline, "{context} exceeded the absolute header deadline");
        stream
            .set_read_timeout(Some(deadline.saturating_duration_since(now)))
            .expect("read timeout must be configurable");
        let read = stream
            .read(&mut buffer)
            .unwrap_or_else(|error| panic!("{context} should remain readable: {error}"));
        assert!(read > 0, "{context} closed before the header completed");
        header.extend_from_slice(&buffer[..read]);
        assert!(
            header.len() <= MAX_HEADER_BYTES,
            "{context} exceeded {MAX_HEADER_BYTES} bytes"
        );
        if header.windows(4).any(|window| window == b"\r\n\r\n") {
            return header;
        }
    }
}

fn status_code(header: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(header).ok()?;
    let status_line = text.split("\r\n").next()?;
    let mut fields = status_line.split_ascii_whitespace();
    if fields.next()? != "HTTP/1.1" {
        return None;
    }
    let status = fields.next()?;
    if status.len() != 3 || !status.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    status.parse().ok()
}

fn wait_for_ready(address: SocketAddr, child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child
            .try_wait()
            .expect("supplier-proxy child state must be readable")
        {
            panic!("supplier-proxy child exited before readiness: {status}");
        }

        if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            stream
                .set_write_timeout(Some(Duration::from_millis(250)))
                .expect("readiness write timeout must be configurable");
            if stream
                .write_all(
                    b"GET /readyz HTTP/1.1\r\nHost: supplier.test\r\nConnection: close\r\n\r\n",
                )
                .is_ok()
            {
                let header = read_header(&mut stream, "supplier-proxy readiness response");
                if status_code(&header) == Some(200) {
                    return;
                }
            }
        }

        assert!(
            Instant::now() < deadline,
            "supplier-proxy child did not become HTTP-ready within 10 seconds"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn start_supplier_proxy(listener: SocketAddr, origin: SocketAddr) -> ChildProcess {
    let executable = env::current_exe().expect("integration-test executable path must exist");
    let mut child = Command::new(executable)
        .args([
            "--ignored",
            "--exact",
            "supplier_proxy_child",
            "--nocapture",
        ])
        .env(CHILD_MODE_ENV, "1")
        .env(CHILD_LISTENER_ENV, listener.to_string())
        .env(CHILD_ORIGIN_ENV, origin.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("supplier-proxy child must start");
    wait_for_ready(listener, &mut child);
    ChildProcess(child)
}

fn spawn_fast_101_echo_origin(listener: TcpListener) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("origin must accept proxy connection");
        let request = read_header(&mut stream, "origin upgrade request");
        let request_text = String::from_utf8_lossy(&request).to_ascii_lowercase();
        assert!(
            request_text.contains("upgrade: websocket\r\n"),
            "supplier proxy must forward the WebSocket Upgrade field: {request_text:?}"
        );
        assert!(
            request_text.contains("connection: upgrade\r\n"),
            "supplier proxy must forward a normalized Connection: upgrade field: {request_text:?}"
        );

        stream
            .write_all(
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n",
            )
            .expect("origin 101 response must be writable");
        stream.flush().expect("origin 101 response must flush");

        let mut received = vec![0_u8; TUNNEL_PAYLOAD.len()];
        stream
            .set_read_timeout(Some(IO_DEADLINE))
            .expect("origin tunnel timeout must be configurable");
        stream
            .read_exact(&mut received)
            .expect("upgraded supplier tunnel must carry client bytes after 101");
        assert_eq!(received, TUNNEL_PAYLOAD);
        stream
            .write_all(&received)
            .expect("origin echo must be writable through the upgraded tunnel");
        stream.flush().expect("origin echo must flush");
    })
}

#[test]
fn released_pingora_keeps_fast_101_upgrade_tunnel_bidirectional() {
    let origin = TcpListener::bind("127.0.0.1:0").expect("origin fixture must bind");
    let origin_address = origin.local_addr().expect("origin address must exist");
    let listener_reservation = TcpListener::bind("127.0.0.1:0")
        .expect("supplier proxy listener must be reservable");
    let listener = listener_reservation
        .local_addr()
        .expect("supplier proxy listener address must exist");

    let origin_thread = spawn_fast_101_echo_origin(origin);
    drop(listener_reservation);
    let _proxy = start_supplier_proxy(listener, origin_address);

    let mut client = TcpStream::connect(listener).expect("supplier proxy must accept client traffic");
    client
        .set_write_timeout(Some(IO_DEADLINE))
        .expect("client write timeout must be configurable");
    client
        .write_all(
            b"GET /socket HTTP/1.1\r\n\
Host: supplier.test\r\n\
Connection: Upgrade\r\n\
Upgrade: websocket\r\n\
Sec-WebSocket-Version: 13\r\n\
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
X-CWL-Delay-Request-Body: 200\r\n\
\r\n",
        )
        .expect("WebSocket opening handshake must be writable");
    client.flush().expect("WebSocket opening handshake must flush");

    let response = read_header(&mut client, "downstream upgrade response");
    assert_eq!(
        status_code(&response),
        Some(101),
        "supplier proxy must first establish the HTTP/1 upgrade: {}",
        String::from_utf8_lossy(&response)
    );

    // Let the deliberately delayed end-of-request-body event arrive after the 101. Pingora 0.9.0
    // currently misclassifies that event as tunnel completion; a release-qualified supplier repair
    // must make this unchanged assertion GREEN.
    thread::sleep(POST_101_SETTLE);
    client
        .write_all(TUNNEL_PAYLOAD)
        .expect("upgraded tunnel must remain writable after delayed request-body completion");
    client.flush().expect("tunnel payload must flush");

    let mut echoed = vec![0_u8; TUNNEL_PAYLOAD.len()];
    client
        .set_read_timeout(Some(IO_DEADLINE))
        .expect("client tunnel timeout must be configurable");
    client
        .read_exact(&mut echoed)
        .expect("upgraded tunnel must remain readable after delayed request-body completion");
    assert_eq!(echoed, TUNNEL_PAYLOAD);

    origin_thread
        .join()
        .expect("fast-101 echo origin must complete without fixture failure");
}

#[test]
#[ignore = "test-support process launched only by the supplier RED parent test"]
fn supplier_proxy_child() {
    if env::var_os(CHILD_MODE_ENV).is_none() {
        return;
    }

    let listener: SocketAddr = env::var(CHILD_LISTENER_ENV)
        .expect("child listener environment must exist")
        .parse()
        .expect("child listener must parse");
    let origin: SocketAddr = env::var(CHILD_ORIGIN_ENV)
        .expect("child origin environment must exist")
        .parse()
        .expect("child origin must parse");

    let proxy = SupplierUpgradeProxy { origin };
    let mut server = Server::new_with_opt_and_conf(None, build_server_conf(4));
    server.bootstrap();
    let mut service = http_proxy_service(&server.configuration, proxy);
    service.add_tcp(&listener.to_string());
    server.add_service(service);
    server.run(RunArgs::default());
}
