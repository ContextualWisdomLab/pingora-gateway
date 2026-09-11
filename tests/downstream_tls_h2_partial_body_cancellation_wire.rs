//! Real-wire HTTP/2 partial request-body cancellation acceptance.
//!
//! A streamed request reaches the H1 origin while still within the configured body budget, then
//! crosses that budget without END_STREAM. The gateway must complete an HTTP-level rejection and
//! release the partial upstream request before the client closes its request side. A compliant
//! sibling must remain usable on the same H2 connection. The test does not require a particular
//! post-response RST_STREAM strategy because RFC 9113 permits, but does not mandate, that mechanism.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use pingora::tls::ssl::{SslConnector, SslMethod, SslVerifyMode};
use tempfile::{tempdir, NamedTempFile};

const H2_DATA: u8 = 0x0;
const H2_HEADERS: u8 = 0x1;
const H2_RST_STREAM: u8 = 0x3;
const H2_SETTINGS: u8 = 0x4;
const H2_GOAWAY: u8 = 0x7;
const H2_ACK: u8 = 0x1;
const H2_END_STREAM: u8 = 0x1;
const H2_END_HEADERS: u8 = 0x4;
const H2_MAX_STREAM_ID: u32 = 0x7fff_ffff;
const H2_CANCEL: u32 = 0x8;
const BODY_LIMIT: usize = 8;
const ADMITTED_PREFIX: &[u8] = b"12345678";
const OVER_LIMIT_BYTE: &[u8] = b"9";
const MAX_ORIGIN_BYTES: usize = 64 * 1024;
const IO_BUDGET: Duration = Duration::from_secs(5);

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
        "/CN=CWL Downstream H2 Partial Body Test CA",
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
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics}\nmax_request_body_bytes: {BODY_LIMIT}\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
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

fn hpack_request(method: u8, path: u8) -> Vec<u8> {
    assert!(method == 0x82 || method == 0x83);
    assert!(path == 0x84 || path == 0x85);
    let mut headers = vec![method, 0x87, path, 0x01, 0x0c];
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

fn decoded_chunk_prefix(raw: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::new();
    let mut cursor = 0_usize;
    while cursor < raw.len() {
        let Some(offset) = raw[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
        else {
            break;
        };
        let line_end = cursor + offset;
        let text = std::str::from_utf8(&raw[cursor..line_end])
            .expect("origin chunk size must be ASCII/UTF-8");
        let token = text.split(';').next().expect("chunk size token");
        let size = usize::from_str_radix(token.trim(), 16)
            .expect("origin request must use valid HTTP/1 chunk sizes");
        cursor = line_end + 2;
        if size == 0 {
            break;
        }
        let take = raw.len().saturating_sub(cursor).min(size);
        decoded.extend_from_slice(&raw[cursor..cursor + take]);
        if take < size {
            break;
        }
        cursor += size;
        if raw.len() < cursor + 2 {
            break;
        }
        assert_eq!(&raw[cursor..cursor + 2], b"\r\n");
        cursor += 2;
    }
    decoded
}

fn read_admitted_prefix(stream: &mut TcpStream) -> Vec<u8> {
    let deadline = Instant::now() + IO_BUDGET;
    let mut raw = Vec::new();
    let mut header_end = None;
    let mut buffer = [0_u8; 1024];

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "admitted request prefix did not reach origin within the I/O budget"
        );
        stream
            .set_read_timeout(Some(remaining))
            .expect("origin read deadline should be set");
        let read = stream
            .read(&mut buffer)
            .expect("partial origin request should be readable");
        assert!(
            read > 0,
            "gateway closed before the admitted request prefix arrived"
        );
        raw.extend_from_slice(&buffer[..read]);
        assert!(raw.len() <= MAX_ORIGIN_BYTES);

        if header_end.is_none() {
            header_end = raw
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|offset| offset + 4);
        }
        let Some(body_offset) = header_end else {
            continue;
        };
        let headers = String::from_utf8_lossy(&raw[..body_offset]).to_ascii_lowercase();
        assert!(headers.starts_with("post / http/1.1\r\n"));
        assert!(headers.contains("\r\ntransfer-encoding: chunked\r\n"));
        let decoded = decoded_chunk_prefix(&raw[body_offset..]);
        if decoded.len() >= ADMITTED_PREFIX.len() {
            assert_eq!(&decoded[..ADMITTED_PREFIX.len()], ADMITTED_PREFIX);
            return raw;
        }
    }
}

fn wait_for_release(stream: &mut TcpStream, mut raw: Vec<u8>) {
    let body_offset = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|offset| offset + 4)
        .expect("partial origin headers must already be complete");
    let deadline = Instant::now() + IO_BUDGET;
    let mut buffer = [0_u8; 1024];

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "gateway did not release partial H1 state within the I/O budget"
        );
        stream
            .set_read_timeout(Some(remaining))
            .expect("origin release deadline should be set");
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                raw.extend_from_slice(&buffer[..read]);
                assert!(raw.len() <= MAX_ORIGIN_BYTES);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe
                ) =>
            {
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("partial upstream release observation failed: {error}"),
        }
    }

    assert_eq!(decoded_chunk_prefix(&raw[body_offset..]), ADMITTED_PREFIX);
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

fn wait_for_rejection(stream: &mut impl ReadWrite, target: u32) {
    let mut headers_seen = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, _) = read_frame(stream);
        assert_ne!(frame_type, H2_GOAWAY, "rejection must remain stream-local");
        if frame_type == H2_SETTINGS && stream_id == 0 && flags & H2_ACK == 0 {
            write_frame(stream, H2_SETTINGS, H2_ACK, 0, &[]);
            stream
                .flush()
                .expect("late SETTINGS acknowledgement should flush");
            continue;
        }
        assert!(
            frame_type != H2_RST_STREAM || stream_id != target,
            "server must complete the HTTP rejection before any reset"
        );
        if stream_id == target && frame_type == H2_HEADERS {
            headers_seen = true;
        }
        if stream_id == target && flags & H2_END_STREAM != 0 {
            assert!(headers_seen, "rejection must include response headers");
            return;
        }
    }
    panic!("partial-body request did not receive a complete HTTP-level rejection");
}

#[test]
fn partial_body_limit_releases_upstream_before_end_stream_and_preserves_sibling() {
    let certificates = certificates();
    let origin_listener = TcpListener::bind("127.0.0.1:0").expect("origin should bind");
    let upstream = origin_listener.local_addr().expect("origin address");
    let (prefix_tx, prefix_rx) = mpsc::channel();
    let (observe_tx, observe_rx) = mpsc::channel();
    let (released_tx, released_rx) = mpsc::channel();

    let origin = thread::spawn(move || {
        let mut partial = accept_within(&origin_listener, "partial streamed request");
        let raw = read_admitted_prefix(&mut partial);
        prefix_tx
            .send(())
            .expect("prefix channel should remain open");
        observe_rx
            .recv_timeout(IO_BUDGET)
            .expect("release observation must start within I/O budget");
        wait_for_release(&mut partial, raw);
        released_tx
            .send(())
            .expect("release channel should remain open");

        let mut sibling = accept_within(&origin_listener, "compliant sibling");
        let request = read_headers(&mut sibling);
        assert!(request.starts_with("GET /index.html HTTP/1.1\r\n"));
        sibling
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nsibling-ok",
            )
            .expect("sibling response should write");
        sibling.flush().expect("sibling response should flush");
    });

    let (traffic, metrics) = reserve_listeners();
    let listener = traffic.local_addr().expect("traffic address");
    let metrics_address = metrics.local_addr().expect("metrics address");
    let config = config(listener, metrics_address, upstream, &certificates);
    let mut process = GatewayProcess(spawn_gateway(&config, traffic, metrics));
    let mut tls = connect_h2(listener, &certificates, &mut process.0);
    assert_eq!(tls.ssl().selected_alpn_protocol(), Some(b"h2".as_slice()));

    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 preface should write");
    write_frame(&mut tls, H2_SETTINGS, 0, 0, &[]);
    tls.flush().expect("client SETTINGS should flush");
    acknowledge_settings(&mut tls);

    write_frame(
        &mut tls,
        H2_HEADERS,
        H2_END_HEADERS,
        1,
        &hpack_request(0x83, 0x84),
    );
    write_frame(&mut tls, H2_DATA, 0, 1, ADMITTED_PREFIX);
    tls.flush().expect("admitted prefix should flush");
    prefix_rx
        .recv_timeout(IO_BUDGET)
        .expect("admitted prefix must reach partial H1 state");

    write_frame(&mut tls, H2_DATA, 0, 1, OVER_LIMIT_BYTE);
    tls.flush().expect("over-limit byte should flush");
    wait_for_rejection(&mut tls, 1);

    observe_tx
        .send(())
        .expect("release observation channel should remain open");
    released_rx
        .recv_timeout(IO_BUDGET)
        .expect("partial H1 state must release before client-side H2 cleanup");

    write_frame(&mut tls, H2_RST_STREAM, 0, 1, &H2_CANCEL.to_be_bytes());
    tls.flush().expect("client cleanup reset should flush");

    write_frame(
        &mut tls,
        H2_HEADERS,
        H2_END_STREAM | H2_END_HEADERS,
        3,
        &hpack_request(0x82, 0x85),
    );
    tls.flush().expect("sibling request should flush");

    let mut sibling_body = Vec::new();
    let mut sibling_headers = false;
    let mut sibling_ended = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_frame(&mut tls);
        assert_ne!(
            frame_type, H2_GOAWAY,
            "partial-body cleanup must preserve the H2 connection"
        );
        assert!(
            frame_type != H2_RST_STREAM || stream_id != 3,
            "compliant sibling must not reset"
        );
        if stream_id == 3 && frame_type == H2_HEADERS {
            sibling_headers = true;
        }
        if stream_id == 3 && frame_type == H2_DATA {
            sibling_body.extend_from_slice(&payload);
        }
        if stream_id == 3 && flags & H2_END_STREAM != 0 {
            sibling_ended = true;
            break;
        }
    }

    assert!(sibling_headers, "sibling must receive response headers");
    assert!(
        sibling_ended,
        "sibling must complete on the same connection"
    );
    assert_eq!(sibling_body, b"sibling-ok");
    origin.join().expect("origin fixture should complete");
}
