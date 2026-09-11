#![cfg(target_os = "linux")]

//! Real-wire HTTP/2 upstream reset and response-commitment acceptance.
//!
//! The generic TLS/H2 gateway has no gateway-local retry/failover policy. This fixture therefore
//! keeps supplier-default transport semantics visible while proving two distinct phases: an
//! established fresh H1 origin that resets before response commitment must fail as an HTTP-level
//! stream-local response without poisoning the H2 connection, while an origin reset after response
//! commitment must not invent a second response or replay the request and must terminate only the
//! affected stream. A compliant sibling must remain usable on the same H2 connection in both cases.

use core::ffi::c_void;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use pingora::tls::ssl::{SslConnector, SslMethod, SslVerifyMode};
use tempfile::{tempdir, NamedTempFile};

const SOL_SOCKET: i32 = 1;
const SO_LINGER: i32 = 13;
const H2_DATA: u8 = 0x0;
const H2_HEADERS: u8 = 0x1;
const H2_RST_STREAM: u8 = 0x3;
const H2_SETTINGS: u8 = 0x4;
const H2_GOAWAY: u8 = 0x7;
const H2_ACK: u8 = 0x1;
const H2_END_STREAM: u8 = 0x1;
const H2_END_HEADERS: u8 = 0x4;
const H2_MAX_STREAM_ID: u32 = 0x7fff_ffff;
const MAX_ORIGIN_BYTES: usize = 64 * 1024;
const IO_BUDGET: Duration = Duration::from_secs(5);
const PARTIAL_BODY: &[u8] = b"partial";

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

struct Certificates {
    _directory: tempfile::TempDir,
    ca: PathBuf,
    certificate: PathBuf,
    key: PathBuf,
}

fn openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

fn certificates() -> Certificates {
    let directory = tempdir().expect("certificate workspace should be available");
    let ca_key = directory.path().join("ca.key");
    let ca = directory.path().join("ca.crt");
    let key = directory.path().join("server.key");
    let csr = directory.path().join("server.csr");
    let certificate = directory.path().join("server.crt");
    let extensions = directory.path().join("server.ext");

    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-out",
        ca.to_str().expect("UTF-8 CA path"),
        "-subj",
        "/CN=CWL Downstream H2 Origin Failure Test CA",
        "-days",
        "1",
        "-sha256",
    ]);
    openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        key.to_str().expect("UTF-8 server key path"),
        "-out",
        csr.to_str().expect("UTF-8 CSR path"),
        "-subj",
        "/CN=gateway.test",
        "-sha256",
    ]);
    fs::write(
        &extensions,
        "subjectAltName=DNS:gateway.test\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n",
    )
    .expect("server certificate extension file should be writable");
    openssl(&[
        "x509",
        "-req",
        "-in",
        csr.to_str().expect("UTF-8 CSR path"),
        "-CA",
        ca.to_str().expect("UTF-8 CA path"),
        "-CAkey",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-CAcreateserial",
        "-out",
        certificate.to_str().expect("UTF-8 certificate path"),
        "-days",
        "1",
        "-sha256",
        "-extfile",
        extensions.to_str().expect("UTF-8 extension path"),
    ]);

    Certificates {
        _directory: directory,
        ca,
        certificate,
        key,
    }
}

fn reserve_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    assert_ne!(
        traffic.local_addr().expect("traffic reservation address"),
        metrics.local_addr().expect("metrics reservation address")
    );
    (traffic, metrics)
}

fn config(
    listener: SocketAddr,
    metrics: SocketAddr,
    upstream: SocketAddr,
    certificates: &Certificates,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificates.certificate.display(),
        certificates.key.display(),
    )
    .expect("gateway config should be written");
    file
}

fn spawn_gateway(config: &NamedTempFile, traffic: TcpListener, metrics: TcpListener) -> Child {
    drop(traffic);
    drop(metrics);
    Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 config path"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("compiled gateway binary should start")
}

fn connect_h2(
    address: SocketAddr,
    certificates: &Certificates,
    process: &mut Child,
) -> pingora::tls::ssl::SslStream<TcpStream> {
    let mut builder =
        SslConnector::builder(SslMethod::tls_client()).expect("TLS client should build");
    builder
        .set_ca_file(&certificates.ca)
        .expect("local CA should load");
    builder.set_verify(SslVerifyMode::PEER);
    builder
        .set_alpn_protos(b"\x02h2")
        .expect("h2 ALPN wire list should be valid");
    let connector = builder.build();
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before TLS/H2 handshake: {status}");
        }
        if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
            stream
                .set_read_timeout(Some(IO_BUDGET))
                .expect("downstream read timeout should be set");
            stream
                .set_write_timeout(Some(IO_BUDGET))
                .expect("downstream write timeout should be set");
            if let Ok(tls) = connector.connect("gateway.test", stream) {
                return tls;
            }
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not accept verified TLS/H2 within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn write_frame(stream: &mut impl Write, frame_type: u8, flags: u8, stream_id: u32, payload: &[u8]) {
    assert!(payload.len() <= 0x00ff_ffff);
    let length = payload.len() as u32;
    let mut header = [0_u8; 9];
    header[0] = ((length >> 16) & 0xff) as u8;
    header[1] = ((length >> 8) & 0xff) as u8;
    header[2] = (length & 0xff) as u8;
    header[3] = frame_type;
    header[4] = flags;
    header[5..9].copy_from_slice(&(stream_id & H2_MAX_STREAM_ID).to_be_bytes());
    stream
        .write_all(&header)
        .expect("H2 frame header should write");
    stream
        .write_all(payload)
        .expect("H2 frame payload should write");
}

fn read_frame(stream: &mut impl Read) -> (u8, u8, u32, Vec<u8>) {
    let mut header = [0_u8; 9];
    stream
        .read_exact(&mut header)
        .expect("H2 frame header should be readable");
    let length = ((header[0] as usize) << 16) | ((header[1] as usize) << 8) | header[2] as usize;
    let stream_id =
        u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & H2_MAX_STREAM_ID;
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .expect("H2 frame payload should be readable");
    (header[3], header[4], stream_id, payload)
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn acknowledge_settings(stream: &mut impl ReadWrite) {
    for _ in 0..16 {
        let (frame_type, flags, stream_id, _) = read_frame(stream);
        assert_ne!(
            frame_type, H2_GOAWAY,
            "server must not close a fresh compliant H2 connection"
        );
        if frame_type == H2_SETTINGS && stream_id == 0 && flags & H2_ACK == 0 {
            write_frame(stream, H2_SETTINGS, H2_ACK, 0, &[]);
            stream
                .flush()
                .expect("SETTINGS acknowledgement should flush");
            return;
        }
    }
    panic!("server SETTINGS was not observed");
}

fn hpack_get(path: u8) -> Vec<u8> {
    assert!(path == 0x84 || path == 0x85);
    let mut headers = vec![0x82, 0x87, path, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    headers
}

fn accept_within(listener: &TcpListener, purpose: &str) -> TcpStream {
    listener
        .set_nonblocking(true)
        .expect("origin acceptance must remain bounded");
    let deadline = Instant::now() + IO_BUDGET;
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    Instant::now() < deadline,
                    "{purpose} did not reach origin within the upstream I/O budget"
                );
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("origin accept failed for {purpose}: {error}"),
        }
    }
}

fn read_headers(stream: &mut TcpStream) -> String {
    let deadline = Instant::now() + IO_BUDGET;
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "origin headers exceeded I/O budget");
        stream
            .set_read_timeout(Some(remaining))
            .expect("origin read deadline should be set");
        let read = stream
            .read(&mut buffer)
            .expect("origin headers should read");
        assert!(read > 0, "gateway closed before origin headers completed");
        raw.extend_from_slice(&buffer[..read]);
        assert!(raw.len() <= MAX_ORIGIN_BYTES);
        if raw.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&raw).into_owned();
        }
    }
}

fn reset_on_close(stream: &TcpStream) {
    let linger = Linger {
        onoff: 1,
        linger_seconds: 0,
    };
    // SAFETY: the descriptor belongs to this live TcpStream, `linger` matches Linux's
    // `struct linger`, and the pointer remains valid for the duration of `setsockopt`.
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
        "Linux SO_LINGER(0) should be configurable: {}",
        std::io::Error::last_os_error()
    );
}

fn start_h2(
    origin: SocketAddr,
    certificates: &Certificates,
) -> (GatewayProcess, pingora::tls::ssl::SslStream<TcpStream>) {
    let (traffic, metrics) = reserve_listeners();
    let listener = traffic.local_addr().expect("traffic address");
    let metrics_address = metrics.local_addr().expect("metrics address");
    let config = config(listener, metrics_address, origin, certificates);
    let mut process = GatewayProcess(spawn_gateway(&config, traffic, metrics));
    let mut tls = connect_h2(listener, certificates, &mut process.0);
    assert_eq!(tls.ssl().selected_alpn_protocol(), Some(b"h2".as_slice()));
    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 preface should write");
    write_frame(&mut tls, H2_SETTINGS, 0, 0, &[]);
    tls.flush().expect("client SETTINGS should flush");
    acknowledge_settings(&mut tls);
    (process, tls)
}

fn write_get(stream: &mut impl Write, stream_id: u32, path: u8) {
    write_frame(
        stream,
        H2_HEADERS,
        H2_END_STREAM | H2_END_HEADERS,
        stream_id,
        &hpack_get(path),
    );
    stream.flush().expect("request HEADERS should flush");
}

fn wait_for_http_failure(stream: &mut impl ReadWrite, target: u32) {
    let mut headers_seen = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, _) = read_frame(stream);
        assert_ne!(
            frame_type, H2_GOAWAY,
            "origin failure must remain stream-local"
        );
        if frame_type == H2_SETTINGS && stream_id == 0 && flags & H2_ACK == 0 {
            write_frame(stream, H2_SETTINGS, H2_ACK, 0, &[]);
            stream
                .flush()
                .expect("late SETTINGS acknowledgement should flush");
            continue;
        }
        assert!(
            frame_type != H2_RST_STREAM || stream_id != target,
            "pre-commit origin failure must complete an HTTP response before any reset"
        );
        if stream_id == target && frame_type == H2_HEADERS {
            headers_seen = true;
        }
        if stream_id == target && flags & H2_END_STREAM != 0 {
            assert!(
                headers_seen,
                "pre-commit failure must include response headers"
            );
            return;
        }
    }
    panic!("pre-commit origin failure did not complete an HTTP-level response");
}

fn wait_for_committed_prefix(stream: &mut impl Read, target: u32) -> Vec<u8> {
    let mut headers_seen = false;
    let mut body = Vec::new();
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_frame(stream);
        assert_ne!(
            frame_type, H2_GOAWAY,
            "committed response must not close H2"
        );
        assert!(
            frame_type != H2_RST_STREAM || stream_id != target,
            "stream must not reset before committed body prefix is observed"
        );
        if stream_id == target && frame_type == H2_HEADERS {
            assert!(!headers_seen, "response commitment must occur only once");
            headers_seen = true;
        }
        if stream_id == target && frame_type == H2_DATA {
            assert!(headers_seen, "response DATA must follow response HEADERS");
            body.extend_from_slice(&payload);
        }
        assert!(
            stream_id != target || flags & H2_END_STREAM == 0,
            "incomplete committed response must not terminate before the injected origin reset"
        );
        if body.len() >= PARTIAL_BODY.len() {
            assert_eq!(&body[..PARTIAL_BODY.len()], PARTIAL_BODY);
            return body;
        }
    }
    panic!("committed response prefix was not observed");
}

fn wait_for_post_commit_failure(stream: &mut impl Read, target: u32, mut body: Vec<u8>) -> Vec<u8> {
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_frame(stream);
        assert_ne!(
            frame_type, H2_GOAWAY,
            "post-commit origin failure must remain stream-local"
        );
        assert!(
            stream_id != target || frame_type != H2_HEADERS,
            "post-commit failure must not invent a second response"
        );
        if stream_id == target && frame_type == H2_DATA {
            body.extend_from_slice(&payload);
        }
        if stream_id == target && frame_type == H2_RST_STREAM {
            return body;
        }
        assert!(
            stream_id != target || flags & H2_END_STREAM == 0,
            "truncated committed response must surface as a stream error, not clean END_STREAM"
        );
    }
    panic!("post-commit origin failure did not terminate the affected stream");
}

fn read_sibling(stream: &mut impl ReadWrite, target: u32) -> Vec<u8> {
    let mut headers_seen = false;
    let mut body = Vec::new();
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_frame(stream);
        assert_ne!(
            frame_type, H2_GOAWAY,
            "origin failure must not poison the shared H2 connection"
        );
        if frame_type == H2_SETTINGS && stream_id == 0 && flags & H2_ACK == 0 {
            write_frame(stream, H2_SETTINGS, H2_ACK, 0, &[]);
            stream
                .flush()
                .expect("late SETTINGS acknowledgement should flush");
            continue;
        }
        assert!(
            frame_type != H2_RST_STREAM || stream_id != target,
            "compliant sibling must not reset"
        );
        if stream_id == target && frame_type == H2_HEADERS {
            headers_seen = true;
        }
        if stream_id == target && frame_type == H2_DATA {
            body.extend_from_slice(&payload);
        }
        if stream_id == target && flags & H2_END_STREAM != 0 {
            assert!(headers_seen, "sibling response must include headers");
            return body;
        }
    }
    panic!("compliant sibling did not complete on the same H2 connection");
}

fn write_sibling_response(stream: &mut TcpStream) {
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nsibling-ok")
        .expect("sibling response should write");
    stream.flush().expect("sibling response should flush");
}

#[test]
fn pre_commit_origin_reset_returns_http_failure_and_preserves_h2_sibling() {
    let certificates = certificates();
    let origin_listener = TcpListener::bind("127.0.0.1:0").expect("origin should bind");
    let origin_address = origin_listener.local_addr().expect("origin address");

    let origin = thread::spawn(move || {
        let mut failed = accept_within(&origin_listener, "pre-commit reset request");
        let request = read_headers(&mut failed);
        assert!(request.starts_with("GET / HTTP/1.1\r\n"));
        reset_on_close(&failed);
        drop(failed);

        // The production adapter does not opt into fail_to_connect retry/failover. If a fresh
        // established reset is replayed before the sibling is sent, this accept consumes that
        // replay and the exact request-target assertion below fails closed.
        let mut sibling = accept_within(&origin_listener, "post-failure sibling");
        let request = read_headers(&mut sibling);
        assert!(request.starts_with("GET /index.html HTTP/1.1\r\n"));
        write_sibling_response(&mut sibling);
    });

    let (_process, mut tls) = start_h2(origin_address, &certificates);
    write_get(&mut tls, 1, 0x84);
    wait_for_http_failure(&mut tls, 1);

    write_get(&mut tls, 3, 0x85);
    assert_eq!(read_sibling(&mut tls, 3), b"sibling-ok");
    origin
        .join()
        .expect("pre-commit origin fixture should complete");
}

#[test]
fn post_commit_origin_reset_preserves_first_response_and_h2_sibling() {
    let certificates = certificates();
    let origin_listener = TcpListener::bind("127.0.0.1:0").expect("origin should bind");
    let origin_address = origin_listener.local_addr().expect("origin address");
    let (reset_tx, reset_rx) = mpsc::channel();

    let origin = thread::spawn(move || {
        let mut failed = accept_within(&origin_listener, "post-commit reset request");
        let request = read_headers(&mut failed);
        assert!(request.starts_with("GET / HTTP/1.1\r\n"));
        failed
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\npartial")
            .expect("committed partial response should write");
        failed
            .flush()
            .expect("committed partial response should flush");
        reset_rx
            .recv_timeout(IO_BUDGET)
            .expect("downstream should observe commitment before reset");
        reset_on_close(&failed);
        drop(failed);

        // A retry after downstream commitment would be accepted here before the sibling request
        // and fail the target assertion, preventing replay from being mistaken for recovery.
        let mut sibling = accept_within(&origin_listener, "post-commit sibling");
        let request = read_headers(&mut sibling);
        assert!(request.starts_with("GET /index.html HTTP/1.1\r\n"));
        write_sibling_response(&mut sibling);
    });

    let (_process, mut tls) = start_h2(origin_address, &certificates);
    write_get(&mut tls, 1, 0x84);
    let partial = wait_for_committed_prefix(&mut tls, 1);
    reset_tx
        .send(())
        .expect("origin reset release channel should remain open");
    let body = wait_for_post_commit_failure(&mut tls, 1, partial);
    assert_eq!(body, PARTIAL_BODY);

    write_get(&mut tls, 3, 0x85);
    assert_eq!(read_sibling(&mut tls, 3), b"sibling-ok");
    origin
        .join()
        .expect("post-commit origin fixture should complete");
}
