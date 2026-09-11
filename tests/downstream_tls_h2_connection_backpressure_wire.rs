//! Real-wire HTTP/2 connection-flow-control acceptance for the generic downstream TLS root.
//!
//! The client deliberately exhausts only the shared connection response window while keeping each
//! stream window large. Control frames and request dispatch must remain live, no response DATA may
//! cross the exhausted connection window, and restoring only connection credit must recover both
//! the original response and a compliant sibling. Bounded-memory acceptance remains separate.

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
const H2_FRAME_PING: u8 = 0x6;
const H2_FRAME_GOAWAY: u8 = 0x7;
const H2_FRAME_WINDOW_UPDATE: u8 = 0x8;
const H2_FLAG_ACK: u8 = 0x1;
const H2_FLAG_END_STREAM: u8 = 0x1;
const H2_FLAG_END_HEADERS: u8 = 0x4;
const H2_MAX_STREAM_ID: u32 = 0x7fff_ffff;
const SETTINGS_INITIAL_WINDOW_SIZE: u16 = 0x4;
const CLIENT_STREAM_WINDOW_BYTES: usize = 1024 * 1024;
const INITIAL_CONNECTION_WINDOW_BYTES: usize = 65_535;
const LARGE_BODY_BYTES: usize = 128 * 1024;
const MAX_ORIGIN_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const PING_BARRIER: &[u8; 8] = b"conn-bar";

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct LocalCertificates {
    _directory: tempfile::TempDir,
    ca_cert: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
}

fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

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
        "/CN=CWL Downstream H2 Connection Backpressure Test CA",
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

fn reserve_distinct_loopback_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    assert_ne!(
        traffic.local_addr().expect("traffic reservation address"),
        metrics.local_addr().expect("metrics reservation address")
    );
    (traffic, metrics)
}

fn write_gateway_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
    certificates: &LocalCertificates,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary config should be writable");
    writeln!(
        file,
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificates.server_cert.display(),
        certificates.server_key.display(),
    )
    .expect("gateway config should be written");
    file
}

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
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("downstream read timeout should be set");
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
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

fn write_window_update(stream: &mut impl Write, stream_id: u32, increment: usize) {
    assert!((1..=H2_MAX_STREAM_ID as usize).contains(&increment));
    write_h2_frame(
        stream,
        H2_FRAME_WINDOW_UPDATE,
        0,
        stream_id,
        &(increment as u32).to_be_bytes(),
    );
}

fn handle_control_frame(
    stream: &mut impl ReadWrite,
    frame_type: u8,
    flags: u8,
    stream_id: u32,
    payload: &[u8],
) -> bool {
    if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
        write_h2_frame(stream, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
        stream
            .flush()
            .expect("SETTINGS acknowledgement should flush");
        return true;
    }
    if frame_type == H2_FRAME_PING && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
        assert_eq!(
            payload.len(),
            8,
            "PING payload must be exactly eight octets"
        );
        write_h2_frame(stream, H2_FRAME_PING, H2_FLAG_ACK, 0, payload);
        stream.flush().expect("PING acknowledgement should flush");
        return true;
    }
    false
}

fn send_client_settings(stream: &mut impl Write) {
    let mut payload = Vec::with_capacity(6);
    payload.extend_from_slice(&SETTINGS_INITIAL_WINDOW_SIZE.to_be_bytes());
    payload.extend_from_slice(&(CLIENT_STREAM_WINDOW_BYTES as u32).to_be_bytes());
    write_h2_frame(stream, H2_FRAME_SETTINGS, 0, 0, &payload);
}

fn finish_settings_handshake(stream: &mut impl ReadWrite) {
    let mut server_settings_seen = false;
    let mut client_settings_acked = false;
    for _ in 0..32 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(stream);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "server must not close a fresh compliant H2 connection"
        );
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 {
            if flags & H2_FLAG_ACK != 0 {
                assert!(
                    payload.is_empty(),
                    "SETTINGS ACK must have an empty payload"
                );
                client_settings_acked = true;
            } else {
                server_settings_seen = true;
                write_h2_frame(stream, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
                stream
                    .flush()
                    .expect("server SETTINGS acknowledgement should flush");
            }
        } else {
            let _ = handle_control_frame(stream, frame_type, flags, stream_id, &payload);
        }
        if server_settings_seen && client_settings_acked {
            return;
        }
    }
    panic!("H2 SETTINGS handshake did not complete within the fixture frame bound");
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn hpack_get(path_index: u8) -> Vec<u8> {
    assert!(path_index == 0x84 || path_index == 0x85);
    let mut headers = vec![0x82, 0x87, path_index, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    headers
}

fn read_origin_headers(stream: &mut TcpStream) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
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
            request.len() <= MAX_ORIGIN_REQUEST_HEADER_BYTES,
            "origin request headers exceeded fixture bound"
        );
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&request).into_owned();
        }
    }
}

fn accept_before(listener: &TcpListener, deadline: Instant) -> TcpStream {
    listener
        .set_nonblocking(true)
        .expect("origin listener must support bounded acceptance");
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    Instant::now() < deadline,
                    "origin request did not arrive before fixture deadline"
                );
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("origin accept failed: {error}"),
        }
    }
}

#[test]
fn exhausted_connection_window_blocks_data_but_not_control_or_recovery() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");
    let (sibling_sent_tx, sibling_sent_rx) = mpsc::channel();

    let origin_fixture = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut large = accept_before(&upstream_listener, deadline);
        let large_request = read_origin_headers(&mut large);
        assert!(
            large_request.starts_with("GET / HTTP/1.1\r\n"),
            "stream 1 must own the large origin response: {large_request:?}"
        );
        large
            .set_write_timeout(Some(Duration::from_secs(10)))
            .expect("large origin write timeout should be set");
        let large_writer = thread::spawn(move || {
            write!(
                large,
                "HTTP/1.1 200 OK\r\nContent-Length: {LARGE_BODY_BYTES}\r\nConnection: close\r\n\r\n"
            )
            .expect("large response headers should be writable");
            large
                .write_all(&vec![b'x'; LARGE_BODY_BYTES])
                .expect("large origin body should be writable");
            large.flush().expect("large origin response should flush");
        });

        let mut sibling = accept_before(&upstream_listener, deadline);
        let sibling_request = read_origin_headers(&mut sibling);
        assert!(
            sibling_request.starts_with("GET /index.html HTTP/1.1\r\n"),
            "stream 3 must remain independently dispatchable: {sibling_request:?}"
        );
        sibling
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nsibling-ok",
            )
            .expect("sibling response should be writable");
        sibling.flush().expect("sibling response should flush");
        sibling_sent_tx
            .send(())
            .expect("sibling observation channel should remain connected");
        large_writer
            .join()
            .expect("large origin writer should complete after connection credit returns");
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
        "connection-backpressure evidence must run on negotiated h2"
    );

    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 connection preface should write");
    send_client_settings(&mut tls);
    tls.flush().expect("client SETTINGS should flush");
    finish_settings_handshake(&mut tls);

    write_h2_frame(
        &mut tls,
        H2_FRAME_HEADERS,
        H2_FLAG_END_STREAM | H2_FLAG_END_HEADERS,
        1,
        &hpack_get(0x84),
    );
    tls.flush().expect("stream 1 request should flush");

    let mut stream1_headers_seen = false;
    let mut stream1_body = Vec::with_capacity(LARGE_BODY_BYTES);
    for _ in 0..128 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(frame_type, H2_FRAME_GOAWAY, "connection must remain live");
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 1,
            "stream 1 must stall through connection flow control rather than reset"
        );
        if handle_control_frame(&mut tls, frame_type, flags, stream_id, &payload) {
            continue;
        }
        if stream_id == 1 && frame_type == H2_FRAME_HEADERS {
            stream1_headers_seen = true;
        }
        if stream_id == 1 && frame_type == H2_FRAME_DATA {
            assert!(
                stream1_headers_seen,
                "response DATA requires preceding headers"
            );
            assert!(
                flags & H2_FLAG_END_STREAM == 0,
                "large response must not finish inside the initial connection window"
            );
            stream1_body.extend_from_slice(&payload);
            assert!(
                stream1_body.len() <= INITIAL_CONNECTION_WINDOW_BYTES,
                "server exceeded the default connection window without connection credit"
            );
            if stream1_body.len() == INITIAL_CONNECTION_WINDOW_BYTES {
                break;
            }
        }
    }
    assert!(
        stream1_headers_seen,
        "stream 1 response headers must be observed"
    );
    assert_eq!(
        stream1_body.len(),
        INITIAL_CONNECTION_WINDOW_BYTES,
        "stream 1 must consume exactly the default connection window before stalling"
    );

    write_h2_frame(
        &mut tls,
        H2_FRAME_HEADERS,
        H2_FLAG_END_STREAM | H2_FLAG_END_HEADERS,
        3,
        &hpack_get(0x85),
    );
    tls.flush()
        .expect("sibling request should flush while connection DATA credit is exhausted");

    sibling_sent_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("sibling origin response must be produced while connection DATA is stalled");

    write_h2_frame(&mut tls, H2_FRAME_PING, 0, 0, PING_BARRIER);
    tls.flush()
        .expect("PING barrier should flush while connection DATA credit is exhausted");

    let mut ping_barrier_acked = false;
    let mut sibling_headers_seen = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "connection-level DATA backpressure must not poison the H2 control plane"
        );
        assert!(
            frame_type != H2_FRAME_RST_STREAM || (stream_id != 1 && stream_id != 3),
            "flow-controlled response streams must not be reset"
        );
        if frame_type == H2_FRAME_PING
            && stream_id == 0
            && flags & H2_FLAG_ACK != 0
            && payload.as_slice() == PING_BARRIER
        {
            ping_barrier_acked = true;
            break;
        }
        if handle_control_frame(&mut tls, frame_type, flags, stream_id, &payload) {
            continue;
        }
        if stream_id == 3 && frame_type == H2_FRAME_HEADERS {
            sibling_headers_seen = true;
        }
        if frame_type == H2_FRAME_DATA && (stream_id == 1 || stream_id == 3) {
            assert!(
                payload.is_empty(),
                "non-empty DATA must not cross an exhausted connection window before WINDOW_UPDATE"
            );
        }
    }
    assert!(
        ping_barrier_acked,
        "PING ACK must remain live while DATA is blocked by connection flow control"
    );

    let remaining_large = LARGE_BODY_BYTES - INITIAL_CONNECTION_WINDOW_BYTES;
    write_window_update(&mut tls, 0, remaining_large + b"sibling-ok".len());
    tls.flush()
        .expect("connection WINDOW_UPDATE should release all stalled response DATA");

    let mut sibling_body = Vec::new();
    let mut sibling_ended = false;
    let mut stream1_ended = false;
    for _ in 0..512 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "connection must remain live while connection credit is restored"
        );
        assert!(
            frame_type != H2_FRAME_RST_STREAM || (stream_id != 1 && stream_id != 3),
            "recovered response streams must not be reset"
        );
        if handle_control_frame(&mut tls, frame_type, flags, stream_id, &payload) {
            continue;
        }
        if stream_id == 1 && frame_type == H2_FRAME_DATA {
            stream1_body.extend_from_slice(&payload);
        }
        if stream_id == 3 && frame_type == H2_FRAME_HEADERS {
            sibling_headers_seen = true;
        }
        if stream_id == 3 && frame_type == H2_FRAME_DATA {
            assert!(
                sibling_headers_seen,
                "sibling DATA requires preceding headers"
            );
            sibling_body.extend_from_slice(&payload);
        }
        if stream_id == 1 && flags & H2_FLAG_END_STREAM != 0 {
            stream1_ended = true;
        }
        if stream_id == 3 && flags & H2_FLAG_END_STREAM != 0 {
            sibling_ended = true;
        }
        if stream1_ended && sibling_ended {
            break;
        }
    }

    assert!(
        stream1_ended,
        "stream 1 must finish after restoring only connection credit"
    );
    assert!(
        sibling_ended,
        "sibling must finish after restoring only connection credit"
    );
    assert_eq!(stream1_body.len(), LARGE_BODY_BYTES);
    assert!(
        stream1_body.iter().all(|byte| *byte == b'x'),
        "connection-flow-controlled response bytes must remain intact across stall/recovery"
    );
    assert_eq!(sibling_body, b"sibling-ok");

    origin_fixture
        .join()
        .expect("connection-flow-control origin fixture should complete");
}
