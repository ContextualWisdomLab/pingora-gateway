//! Real-wire supplier RED for the Pingora 0.9.0 fast-101 WebSocket teardown race.
//!
//! Production gateway versions intentionally continue to deny HTTP/1 Upgrade. This fixture uses a
//! test-only Pingora proxy with the supplier's standards-oriented upgrade policy so the released
//! supplier behavior can be characterized without adding a gateway WebSocket capability or copying
//! the upstream repair. The request-body callback is delayed to deterministically force the ordering
//! documented by cloudflare/pingora#946/#947: the upstream 101 is observed before the queued
//! end-of-request-body event.

use std::env;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use cwl_pingora_gateway::runtime_policy::build_server_conf;
use pingora::prelude::{
    http_proxy_service, sleep, HttpPeer, ProxyHttp, ResponseHeader, Server, Session,
};
use pingora::server::RunArgs;
use pingora::upstreams::peer::{HttpUpstreamRequestPolicy, ALPN};

const CHILD_MODE_ENV: &str = "CWL_WEBSOCKET_SUPPLIER_CHILD";
const CHILD_LISTENER_ENV: &str = "CWL_WEBSOCKET_SUPPLIER_LISTENER";
const CHILD_ORIGIN_ENV: &str = "CWL_WEBSOCKET_SUPPLIER_ORIGIN";
const FAST_101_BODY_DELAY: Duration = Duration::from_millis(200);
const POST_101_SETTLE: Duration = Duration::from_millis(300);
const IO_DEADLINE: Duration = Duration::from_secs(5);
const READINESS_ATTEMPT: Duration = Duration::from_millis(250);
const MAX_HEADER_BYTES: usize = 64 * 1024;
const WEBSOCKET_PAYLOAD: &[u8] = b"cwl-fast-101";
const CLIENT_MASK: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
const RFC_SAMPLE_KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
const RFC_SAMPLE_ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

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
        assert!(
            now < deadline,
            "{context} exceeded the absolute header deadline"
        );
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

fn header_value<'a>(header: &'a [u8], name: &str) -> Option<&'a str> {
    let text = std::str::from_utf8(header).ok()?;
    text.split("\r\n")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (field_name, value) = line.split_once(':')?;
            field_name
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
}

fn header_has_token(header: &[u8], name: &str, expected: &str) -> bool {
    header_value(header, name).is_some_and(|value| {
        value
            .split(',')
            .any(|token| token.trim().eq_ignore_ascii_case(expected))
    })
}

fn readiness_is_200(address: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) else {
        return false;
    };
    if stream.set_write_timeout(Some(READINESS_ATTEMPT)).is_err()
        || stream.set_read_timeout(Some(READINESS_ATTEMPT)).is_err()
        || stream
            .write_all(
                b"GET /readyz HTTP/1.1\r\nHost: supplier.test\r\nConnection: close\r\n\r\n",
            )
            .is_err()
    {
        return false;
    }

    let deadline = Instant::now() + READINESS_ATTEMPT;
    let mut header = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        let now = Instant::now();
        if now >= deadline
            || stream
                .set_read_timeout(Some(deadline.saturating_duration_since(now)))
                .is_err()
        {
            return false;
        }
        match stream.read(&mut buffer) {
            Ok(0) => return false,
            Ok(read) => {
                header.extend_from_slice(&buffer[..read]);
                if header.len() > MAX_HEADER_BYTES {
                    return false;
                }
                if header.windows(4).any(|window| window == b"\r\n\r\n") {
                    return status_code(&header) == Some(200);
                }
            }
            Err(error)
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                return false;
            }
            Err(_) => return false,
        }
    }
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
        if readiness_is_200(address) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "supplier-proxy child did not become HTTP-ready within 10 seconds"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn start_supplier_proxy(
    listener_reservation: TcpListener,
    origin: SocketAddr,
) -> (SocketAddr, ChildProcess) {
    let listener = listener_reservation
        .local_addr()
        .expect("supplier proxy listener address must exist");
    let executable = env::current_exe().expect("integration-test executable path must exist");
    let mut command = Command::new(executable);
    command
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
        .stderr(Stdio::null());

    // Hold the reservation through child-command construction so unrelated local activity cannot
    // claim the selected port. The unavoidable reservation-to-bind gap begins only at spawn.
    drop(listener_reservation);
    let child = command.spawn().expect("supplier-proxy child must start");
    let mut child = ChildProcess(child);
    wait_for_ready(listener, &mut child.0);
    (listener, child)
}

fn masked_client_text_frame(payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 126, "fixture uses only a short text frame");
    let mut frame = Vec::with_capacity(2 + CLIENT_MASK.len() + payload.len());
    frame.push(0x81);
    frame.push(0x80 | payload.len() as u8);
    frame.extend_from_slice(&CLIENT_MASK);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ CLIENT_MASK[index % CLIENT_MASK.len()]),
    );
    frame
}

fn read_exact_before(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    deadline: Instant,
    context: &str,
) {
    let mut offset = 0;
    while offset < buffer.len() {
        let now = Instant::now();
        assert!(
            now < deadline,
            "{context} exceeded the absolute frame deadline"
        );
        stream
            .set_read_timeout(Some(deadline.saturating_duration_since(now)))
            .expect("frame read timeout must be configurable");
        match stream.read(&mut buffer[offset..]) {
            Ok(0) => panic!("{context} closed before the frame completed"),
            Ok(read) => offset += read,
            Err(error)
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                panic!("{context} exceeded the absolute frame deadline: {error}")
            }
            Err(error) => panic!("{context} should remain readable: {error}"),
        }
    }
}

fn read_client_text_frame(stream: &mut TcpStream) -> Vec<u8> {
    let deadline = Instant::now() + IO_DEADLINE;
    let mut prefix = [0_u8; 2];
    read_exact_before(
        stream,
        &mut prefix,
        deadline,
        "origin client-frame prefix",
    );
    assert_eq!(prefix[0], 0x81, "client fixture must send one FIN text frame");
    assert_ne!(
        prefix[1] & 0x80,
        0,
        "RFC 6455 client-to-server frames must be masked"
    );
    let payload_len = usize::from(prefix[1] & 0x7f);
    assert!(payload_len < 126, "fixture does not admit extended lengths");

    let mut mask = [0_u8; 4];
    read_exact_before(stream, &mut mask, deadline, "origin client-frame mask");
    let mut payload = vec![0_u8; payload_len];
    read_exact_before(
        stream,
        &mut payload,
        deadline,
        "origin client-frame payload",
    );
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[index % mask.len()];
    }
    payload
}

fn server_text_frame(payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 126, "fixture uses only a short text frame");
    let mut frame = Vec::with_capacity(2 + payload.len());
    frame.push(0x81);
    frame.push(payload.len() as u8);
    frame.extend_from_slice(payload);
    frame
}

fn read_server_text_frame(stream: &mut TcpStream) -> Vec<u8> {
    let deadline = Instant::now() + IO_DEADLINE;
    let mut prefix = [0_u8; 2];
    read_exact_before(
        stream,
        &mut prefix,
        deadline,
        "client server-frame prefix",
    );
    assert_eq!(prefix[0], 0x81, "origin fixture must echo one FIN text frame");
    assert_eq!(
        prefix[1] & 0x80,
        0,
        "RFC 6455 server-to-client frames must not be masked"
    );
    let payload_len = usize::from(prefix[1] & 0x7f);
    assert!(payload_len < 126, "fixture does not admit extended lengths");
    let mut payload = vec![0_u8; payload_len];
    read_exact_before(
        stream,
        &mut payload,
        deadline,
        "client server-frame payload",
    );
    payload
}

fn spawn_fast_101_echo_origin(listener: TcpListener) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("origin must accept proxy connection");
        let request = read_header(&mut stream, "origin upgrade request");
        assert!(
            header_has_token(&request, "Upgrade", "websocket"),
            "supplier proxy must forward Upgrade: websocket"
        );
        assert!(
            header_has_token(&request, "Connection", "upgrade"),
            "supplier proxy must forward a normalized Connection: upgrade token"
        );
        assert_eq!(
            header_value(&request, "Sec-WebSocket-Version"),
            Some("13"),
            "supplier proxy must preserve the WebSocket version"
        );
        assert_eq!(
            header_value(&request, "Sec-WebSocket-Key"),
            Some(RFC_SAMPLE_KEY),
            "supplier proxy must preserve the case-sensitive WebSocket key"
        );

        stream
            .write_all(
                b"HTTP/1.1 101 Switching Protocols\r\n\
Connection: Upgrade\r\n\
Upgrade: websocket\r\n\
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\
\r\n",
            )
            .expect("origin 101 response must be writable");
        stream.flush().expect("origin 101 response must flush");

        let payload = read_client_text_frame(&mut stream);
        assert_eq!(payload, WEBSOCKET_PAYLOAD);
        stream
            .write_all(&server_text_frame(&payload))
            .expect("origin WebSocket echo frame must be writable through the upgraded tunnel");
        stream
            .flush()
            .expect("origin WebSocket echo frame must flush");
    })
}

#[test]
fn released_pingora_keeps_fast_101_upgrade_tunnel_bidirectional() {
    let origin = TcpListener::bind("127.0.0.1:0").expect("origin fixture must bind");
    let origin_address = origin.local_addr().expect("origin address must exist");
    let listener_reservation =
        TcpListener::bind("127.0.0.1:0").expect("supplier proxy listener must be reservable");

    let (listener, _proxy) = start_supplier_proxy(listener_reservation, origin_address);
    let origin_thread = spawn_fast_101_echo_origin(origin);

    let mut client =
        TcpStream::connect(listener).expect("supplier proxy must accept client traffic");
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
    assert!(
        header_has_token(&response, "Connection", "upgrade"),
        "valid WebSocket 101 must preserve the Connection: upgrade token"
    );
    assert!(
        header_has_token(&response, "Upgrade", "websocket"),
        "valid WebSocket 101 must preserve Upgrade: websocket"
    );
    assert_eq!(
        header_value(&response, "Sec-WebSocket-Accept"),
        Some(RFC_SAMPLE_ACCEPT),
        "proxy must preserve the case-sensitive RFC 6455 accept value"
    );

    // Let the deliberately delayed end-of-request-body event arrive after the 101. Pingora 0.9.0
    // currently misclassifies that event as tunnel completion; a release-qualified supplier repair
    // must make this unchanged assertion GREEN.
    thread::sleep(POST_101_SETTLE);
    client
        .write_all(&masked_client_text_frame(WEBSOCKET_PAYLOAD))
        .expect("upgraded tunnel must remain writable after delayed request-body completion");
    client.flush().expect("WebSocket client frame must flush");

    let echoed = read_server_text_frame(&mut client);
    assert_eq!(echoed, WEBSOCKET_PAYLOAD);

    origin_thread
        .join()
        .expect("fast-101 WebSocket origin must complete without fixture failure");
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
