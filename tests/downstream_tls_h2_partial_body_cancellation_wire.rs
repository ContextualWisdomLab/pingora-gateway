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

const H2_FRAME_DATA: u8 = 0x0;
const H2_FRAME_HEADERS: u8 = 0x1;
const H2_FRAME_RST_STREAM: u8 = 0x3;
const H2_FRAME_SETTINGS: u8 = 0x4;
const H2_FRAME_GOAWAY: u8 = 0x7;
const H2_FLAG_ACK: u8 = 0x1;
const H2_FLAG_END_STREAM: u8 = 0x1;
const H2_FLAG_END_HEADERS: u8 = 0x4;
const H2_MAX_STREAM_ID: u32 = 0x7fff_ffff;
const H2_ERROR_CANCEL: u32 = 0x8;
const REQUEST_BODY_LIMIT_BYTES: usize = 8;
const ADMITTED_PREFIX: &[u8] = b"12345678";
const OVER_LIMIT_BYTE: &[u8] = b"9";
const MAX_ORIGIN_REQUEST_BYTES: usize = 64 * 1024;
const IO_BUDGET: Duration = Duration::from_secs(5);

/// Ensures a spawned gateway cannot survive a failed or completed fixture.
struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Retains temporary certificate material for the negotiated-H2 fixture lifetime.
struct LocalCertificates {
    _directory: tempfile::TempDir,
    ca_cert: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
}

/// Executes the explicitly installed OpenSSL CLI and fails closed on fixture errors.
fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

/// Issues a one-day local CA/server identity so the acceptance verifies TLS and ALPN.
fn issue_gateway_certificate() -> LocalCertificates {
    let directory = tempdir().expect("certificate workspace should be available");
    let ca_key = directory.path().join("ca.key");
    let ca_cert = directory.path().join("ca.crt");
    let server_key = directory.path().join("server.key");
    let server_csr = directory.path().join("server.csr");
    let server_cert = directory.path().join("server.crt");
    let server_ext = directory.path().join("server.ext");

    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-out",
        ca_cert.to_str().expect("UTF-8 CA certificate path"),
        "-subj",
        "/CN=CWL Downstream H2 Partial Body Test CA",
        "-days",
        "1",
        "-sha256",
    ]);
    run_openssl(&[
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        server_key.to_str().expect("UTF-8 server key path"),
        "-out",
        server_csr.to_str().expect("UTF-8 server CSR path"),
        "-subj",
        "/CN=gateway.test",
        "-sha256",
    ]);
    fs::write(
        &server_ext,
        "subjectAltName=DNS:gateway.test\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n",
    )
    .expect("server certificate extension file should be writable");
    run_openssl(&[
        "x509",
        "-req",
        "-in",
        server_csr.to_str().expect("UTF-8 server CSR path"),
        "-CA",
        ca_cert.to_str().expect("UTF-8 CA certificate path"),
        "-CAkey",
        ca_key.to_str().expect("UTF-8 CA key path"),
        "-CAcreateserial",
        "-out",
        server_cert.to_str().expect("UTF-8 server certificate path"),
        "-days",
        "1",
        "-sha256",
        "-extfile",
        server_ext.to_str().expect("UTF-8 extension path"),
    ]);

    LocalCertificates {
        _directory: directory,
        ca_cert,
        server_cert,
        server_key,
    }
}

/// Reserves traffic and metrics authorities simultaneously until gateway process launch.
fn reserve_distinct_loopback_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    assert_ne!(
        traffic.local_addr().expect("traffic reservation address"),
        metrics.local_addr().expect("metrics reservation address")
    );
    (traffic, metrics)
}

/// Writes the versioned generic TLS/H2 contract with an eight-byte request-body budget.
fn write_gateway_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
    certificates: &LocalCertificates,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: {REQUEST_BODY_LIMIT_BYTES}\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificates.server_cert.display(),
        certificates.server_key.display(),
    )
    .expect("gateway config should be written");
    file
}

/// Releases listener reservations only at process launch, then starts the compiled gateway.
fn spawn_gateway(
    config: &NamedTempFile,
    traffic_reservation: TcpListener,
    metrics_reservation: TcpListener,
) -> Child {
    drop(traffic_reservation);
    drop(metrics_reservation);
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

/// Connects only when the compiled listener accepts a CA-verified session negotiated as H2.
fn connect_h2(
    address: SocketAddr,
    certificates: &LocalCertificates,
    process: &mut Child,
) -> pingora::tls::ssl::SslStream<TcpStream> {
    let mut builder =
        SslConnector::builder(SslMethod::tls_client()).expect("TLS client should build");
    builder
        .set_ca_file(&certificates.ca_cert)
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
            panic!("gateway exited before downstream TLS/H2 handshake: {status}");
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
            "gateway did not accept a verified TLS/H2 connection within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

/// Encodes one raw HTTP/2 frame without relying on a client library's request-body behavior.
fn write_h2_frame(
    stream: &mut impl Write,
    frame_type: u8,
    flags: u8,
    stream_id: u32,
    payload: &[u8],
) {
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

/// Reads one complete HTTP/2 frame so stream-local response/cancellation remains observable.
fn read_h2_frame(stream: &mut impl Read) -> (u8, u8, u32, Vec<u8>) {
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

/// Completes the server SETTINGS side of the raw H2 handshake before acceptance traffic starts.
fn acknowledge_server_settings(stream: &mut impl ReadWrite) {
    for _ in 0..16 {
        let (frame_type, flags, stream_id, _payload) = read_h2_frame(stream);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "server must not close a fresh compliant H2 connection"
        );
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            write_h2_frame(stream, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            stream
                .flush()
                .expect("SETTINGS acknowledgement should flush");
            return;
        }
    }
    panic!("server SETTINGS was not observed");
}

/// Encodes a static-table GET or POST request with the fixture's verified authority.
fn hpack_base_request(method_index: u8, path_index: u8) -> Vec<u8> {
    assert!(method_index == 0x82 || method_index == 0x83);
    assert!(path_index == 0x84 || path_index == 0x85);
    let mut headers = vec![method_index, 0x87, path_index, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    headers
}

/// Accepts one origin connection within the same configured upstream I/O budget used by the gateway.
fn accept_origin_within(listener: &TcpListener, purpose: &str) -> TcpStream {
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

/// Decodes all complete or partially received HTTP/1 chunk payload bytes available in `raw`.
fn decode_chunked_prefix(raw: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::new();
    let mut cursor = 0_usize;
    loop {
        let Some(line_end_offset) = raw[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
        else {
            break;
        };
        let line_end = cursor + line_end_offset;
        let size_text = std::str::from_utf8(&raw[cursor..line_end])
            .expect("origin chunk size must be ASCII/UTF-8");
        let size_token = size_text.split(';').next().expect("chunk size token");
        let size = usize::from_str_radix(size_token.trim(), 16)
            .expect("origin request must use valid HTTP/1 chunk sizes");
        cursor = line_end + 2;
        if size == 0 {
            break;
        }
        let available = raw.len().saturating_sub(cursor);
        let take = available.min(size);
        decoded.extend_from_slice(&raw[cursor..cursor + take]);
        if take < size {
            break;
        }
        cursor += size;
        if raw.len() < cursor + 2 {
            break;
        }
        assert_eq!(
            &raw[cursor..cursor + 2],
            b"\r\n",
            "origin chunk payload must end with CRLF"
        );
        cursor += 2;
    }
    decoded
}

/// Reads the partial H1 request until the full admitted eight-byte prefix has reached the origin.
fn read_partial_origin_request(stream: &mut TcpStream) -> Vec<u8> {
    let deadline = Instant::now() + IO_BUDGET;
    let mut raw = Vec::new();
    let mut header_end = None;
    let mut buffer = [0_u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "partial upstream request did not expose the admitted prefix within the I/O budget"
        );
        stream
            .set_read_timeout(Some(remaining))
            .expect("origin read deadline should be set");
        let read = stream
            .read(&mut buffer)
            .expect("partial origin request should be readable");
        assert!(read > 0, "gateway closed before the admitted request prefix arrived");
        raw.extend_from_slice(&buffer[..read]);
        assert!(
            raw.len() <= MAX_ORIGIN_REQUEST_BYTES,
            "partial origin request exceeded the fixture bound"
        );
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
        assert!(
            headers.starts_with("post / http/1.1\r\n"),
            "partial-body stream must own the root POST request: {headers:?}"
        );
        assert!(
            headers.contains("\r\ntransfer-encoding: chunked\r\n"),
            "unknown-length H2 request must use explicit chunked H1 framing upstream: {headers:?}"
        );
        let decoded = decode_chunked_prefix(&raw[body_offset..]);
        if decoded.len() >= ADMITTED_PREFIX.len() {
            assert_eq!(
                &decoded[..ADMITTED_PREFIX.len()],
                ADMITTED_PREFIX,
                "origin must receive the exact within-budget request prefix before cancellation"
            );
            return raw;
        }
    }
}

/// Waits for the gateway to release the partial upstream request before client-side H2 cleanup.
fn wait_for_upstream_release(stream: &mut TcpStream, mut raw: Vec<u8>) {
    let header_end = raw
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
            "gateway did not release the partial H1 upstream request within the I/O budget"
        );
        stream
            .set_read_timeout(Some(remaining))
            .expect("origin release deadline should be set");
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                raw.extend_from_slice(&buffer[..read]);
                assert!(
                    raw.len() <= MAX_ORIGIN_REQUEST_BYTES,
                    "partial origin request exceeded the fixture bound before release"
                );
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

    let decoded = decode_chunked_prefix(&raw[header_end..]);
    assert_eq!(
        decoded, ADMITTED_PREFIX,
        "the over-limit byte must not be forwarded into the partial H1 request before cleanup"
    );
}

/// Reads one complete bounded H1 request header block despite arbitrary TCP segmentation.
fn read_origin_headers(stream: &mut TcpStream) -> String {
    let deadline = Instant::now() + IO_BUDGET;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "origin request headers exceeded the fixture deadline"
        );
        stream
            .set_read_timeout(Some(remaining))
            .expect("origin read deadline should be set");
        let read = stream
            .read(&mut buffer)
            .expect("origin request should be readable before fixture deadline");
        assert!(read > 0, "gateway closed before origin headers completed");
        request.extend_from_slice(&buffer[..read]);
        assert!(
            request.len() <= MAX_ORIGIN_REQUEST_BYTES,
            "origin request headers exceeded fixture bound"
        );
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&request).into_owned();
        }
    }
}

/// Waits for a complete HTTP-level rejection while permitting a later stream-local reset strategy.
fn wait_for_rejection(stream: &mut impl ReadWrite, target_stream_id: u32) {
    let mut response_headers_seen = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, _payload) = read_h2_frame(stream);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "partial-body rejection must not poison the H2 connection"
        );
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            write_h2_frame(stream, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            stream
                .flush()
                .expect("late SETTINGS acknowledgement should flush");
            continue;
        }
        if frame_type == H2_FRAME_RST_STREAM && stream_id == target_stream_id {
            assert!(
                response_headers_seen,
                "server must not replace the HTTP-level body-limit rejection with a pre-response reset"
            );
            continue;
        }
        if stream_id == target_stream_id && frame_type == H2_FRAME_HEADERS {
            response_headers_seen = true;
        }
        if stream_id == target_stream_id && flags & H2_FLAG_END_STREAM != 0 {
            assert!(
                response_headers_seen,
                "partial-body rejection must include HTTP response headers"
            );
            return;
        }
    }
    panic!("partial-body stream did not receive a bounded complete HTTP-level rejection");
}

/// Proves an over-limit incomplete H2 request releases partial H1 state before client cleanup.
#[test]
fn partial_streamed_body_limit_releases_upstream_before_end_stream_and_preserves_sibling() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");
    let (prefix_seen_tx, prefix_seen_rx) = mpsc::channel();
    let (observe_release_tx, observe_release_rx) = mpsc::channel();
    let (released_tx, released_rx) = mpsc::channel();

    let origin_fixture = thread::spawn(move || {
        let mut partial = accept_origin_within(&upstream_listener, "partial streamed request");
        let raw = read_partial_origin_request(&mut partial);
        prefix_seen_tx
            .send(())
            .expect("partial-prefix observation channel should remain connected");
        observe_release_rx
            .recv_timeout(IO_BUDGET)
            .expect("client must request upstream-release observation within the I/O budget");
        wait_for_upstream_release(&mut partial, raw);
        released_tx
            .send(())
            .expect("upstream-release observation channel should remain connected");

        let mut sibling = accept_origin_within(&upstream_listener, "compliant sibling");
        let request = read_origin_headers(&mut sibling);
        assert!(
            request.starts_with("GET /index.html HTTP/1.1\r\n"),
            "only the compliant sibling may follow partial-request cleanup: {request:?}"
        );
        sibling
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nsibling-ok",
            )
            .expect("sibling origin response should be writable");
        sibling
            .flush()
            .expect("sibling origin response should flush");
    });

    let (listener_reservation, metrics_reservation) = reserve_distinct_loopback_listeners();
    let listener = listener_reservation
        .local_addr()
        .expect("traffic reservation address");
    let metrics_listener = metrics_reservation
        .local_addr()
        .expect("metrics reservation address");
    let config = write_gateway_config(listener, metrics_listener, upstream, &certificates);
    let mut process = GatewayProcess(spawn_gateway(
        &config,
        listener_reservation,
        metrics_reservation,
    ));
    let mut tls = connect_h2(listener, &certificates, &mut process.0);
    assert_eq!(
        tls.ssl().selected_alpn_protocol(),
        Some(b"h2".as_slice()),
        "partial-body cancellation evidence must run on negotiated h2"
    );

    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 connection preface should write");
    write_h2_frame(&mut tls, H2_FRAME_SETTINGS, 0, 0, &[]);
    tls.flush().expect("client SETTINGS should flush");
    acknowledge_server_settings(&mut tls);

    write_h2_frame(
        &mut tls,
        H2_FRAME_HEADERS,
        H2_FLAG_END_HEADERS,
        1,
        &hpack_base_request(0x83, 0x84),
    );
    write_h2_frame(&mut tls, H2_FRAME_DATA, 0, 1, ADMITTED_PREFIX);
    tls.flush()
        .expect("within-budget partial request prefix should flush without END_STREAM");
    prefix_seen_rx
        .recv_timeout(IO_BUDGET)
        .expect("within-budget request prefix must reach the partial H1 upstream");

    write_h2_frame(&mut tls, H2_FRAME_DATA, 0, 1, OVER_LIMIT_BYTE);
    tls.flush()
        .expect("over-limit byte should flush while the request remains incomplete");
    wait_for_rejection(&mut tls, 1);

    observe_release_tx
        .send(())
        .expect("release-observation channel should remain connected");
    released_rx.recv_timeout(IO_BUDGET).expect(
        "gateway must release the partial H1 upstream request before client-side H2 cancellation",
    );

    write_h2_frame(
        &mut tls,
        H2_FRAME_RST_STREAM,
        0,
        1,
        &H2_ERROR_CANCEL.to_be_bytes(),
    );
    tls.flush()
        .expect("client cleanup RST_STREAM should flush only after upstream release is proven");

    write_h2_frame(
        &mut tls,
        H2_FRAME_HEADERS,
        H2_FLAG_END_STREAM | H2_FLAG_END_HEADERS,
        3,
        &hpack_base_request(0x82, 0x85),
    );
    tls.flush().expect("compliant sibling should flush");

    let mut sibling_body = Vec::new();
    let mut sibling_headers_seen = false;
    let mut sibling_ended = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "H2 connection must remain usable after partial-body cleanup"
        );
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 3,
            "compliant sibling stream must not be reset"
        );
        if stream_id == 3 && frame_type == H2_FRAME_HEADERS {
            sibling_headers_seen = true;
        }
        if stream_id == 3 && frame_type == H2_FRAME_DATA {
            sibling_body.extend_from_slice(&payload);
        }
        if stream_id == 3 && flags & H2_FLAG_END_STREAM != 0 {
            sibling_ended = true;
            break;
        }
    }

    assert!(
        sibling_headers_seen,
        "compliant sibling must receive response headers"
    );
    assert!(
        sibling_ended,
        "compliant sibling must complete on the same H2 connection"
    );
    assert_eq!(sibling_body, b"sibling-ok");
    origin_fixture
        .join()
        .expect("partial-body origin fixture should complete");
}
