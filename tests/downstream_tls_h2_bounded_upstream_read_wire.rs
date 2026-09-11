//! Real-wire HTTP/2 bounded-read-ahead acceptance under exhausted downstream connection credit.
//!
//! The client gives every stream ample credit but withholds shared connection WINDOW_UPDATE after
//! exactly the RFC 9113 initial connection window is consumed. A large H1 origin response must then
//! experience socket backpressure before the origin can hand the complete body to the gateway. The
//! client restores only connection credit and requires the original response to complete intact.
//! This is causal read-ahead/backpressure evidence; it does not invent a universal RSS ceiling.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
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
const CLIENT_STREAM_WINDOW_BYTES: u32 = 128 * 1024 * 1024;
const INITIAL_CONNECTION_WINDOW_BYTES: usize = 65_535;
const PRESSURE_BODY_BYTES: usize = 64 * 1024 * 1024;
const ORIGIN_CHUNK_BYTES: usize = 64 * 1024;
const MAX_ORIGIN_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const PING_BARRIER: &[u8; 8] = b"mem-bound";

/// Ensures a spawned gateway cannot survive a failed or completed fixture.
struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Retains temporary certificate material for the full negotiated-H2 acceptance lifetime.
struct LocalCertificates {
    _directory: tempfile::TempDir,
    ca_cert: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
}

/// Executes the explicitly installed OpenSSL CLI and fails closed on certificate-fixture errors.
fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

/// Issues a one-day local CA/server identity so the test exercises certificate-verified H2.
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
        "/CN=CWL Downstream H2 Bounded Read Ahead Test CA",
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

/// Reserves traffic and metrics authorities simultaneously to avoid ephemeral-port TOCTOU reuse.
fn reserve_distinct_loopback_listeners() -> (TcpListener, TcpListener) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    assert_ne!(
        traffic.local_addr().expect("traffic reservation address"),
        metrics.local_addr().expect("metrics reservation address")
    );
    (traffic, metrics)
}

/// Writes the same versioned generic TLS/H2 runtime contract used by the neighboring H2 fixtures.
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

/// Releases reserved listener sockets only at process launch, then starts the compiled gateway.
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

/// Connects only when the compiled listener accepts a CA-verified TLS session negotiated as H2.
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

/// Encodes one raw HTTP/2 frame without relying on a client library's flow-control behavior.
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

/// Reads one complete HTTP/2 frame so DATA credit and control-plane behavior remain observable.
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

/// Grants additional flow-control credit to exactly the requested connection or stream scope.
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

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// Acknowledges peer control frames required to keep the raw H2 connection standards-compliant.
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
        assert_eq!(payload.len(), 8, "PING payload must be exactly eight octets");
        write_h2_frame(stream, H2_FRAME_PING, H2_FLAG_ACK, 0, payload);
        stream.flush().expect("PING acknowledgement should flush");
        return true;
    }
    false
}

/// Advertises a stream window larger than the entire pressure body so only connection credit binds.
fn send_client_settings(stream: &mut impl Write) {
    let mut payload = Vec::with_capacity(6);
    payload.extend_from_slice(&SETTINGS_INITIAL_WINDOW_SIZE.to_be_bytes());
    payload.extend_from_slice(&CLIENT_STREAM_WINDOW_BYTES.to_be_bytes());
    write_h2_frame(stream, H2_FRAME_SETTINGS, 0, 0, &payload);
}

/// Completes both sides of the SETTINGS exchange before any acceptance traffic is admitted.
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
                assert!(payload.is_empty(), "SETTINGS ACK must have an empty payload");
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

/// Encodes the static-table GET `/` request used by neighboring raw-H2 acceptance fixtures.
fn hpack_root_get() -> Vec<u8> {
    let mut headers = vec![0x82, 0x87, 0x84, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    headers
}

/// Reads a complete bounded H1 origin header block despite arbitrary TCP segmentation.
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

/// Proves downstream zero connection credit propagates backpressure to a large H1 origin response.
#[test]
fn exhausted_connection_window_bounds_upstream_read_ahead_and_recovers() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");
    let connection_window_exhausted = Arc::new(AtomicBool::new(false));
    let origin_completed = Arc::new(AtomicBool::new(false));
    let (backpressured_tx, backpressured_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();

    let origin_exhausted = Arc::clone(&connection_window_exhausted);
    let origin_done = Arc::clone(&origin_completed);
    let origin_fixture = thread::spawn(move || {
        let (mut origin, _) = upstream_listener
            .accept()
            .expect("gateway should connect to the bounded-pressure origin");
        let request = read_origin_headers(&mut origin);
        assert!(
            request.starts_with("GET / HTTP/1.1\r\n"),
            "pressure stream must own the root origin request: {request:?}"
        );
        write!(
            origin,
            "HTTP/1.1 200 OK\r\nContent-Length: {PRESSURE_BODY_BYTES}\r\nConnection: close\r\n\r\n"
        )
        .expect("pressure response headers should be writable");
        origin
            .flush()
            .expect("pressure response headers should flush before the body probe");
        origin
            .set_nonblocking(true)
            .expect("origin must expose immediate socket backpressure without a timer threshold");

        let chunk = [b'x'; ORIGIN_CHUNK_BYTES];
        let mut written = 0_usize;
        let mut backpressure_reported = false;
        while written < PRESSURE_BODY_BYTES {
            let remaining = PRESSURE_BODY_BYTES - written;
            match origin.write(&chunk[..remaining.min(chunk.len())]) {
                Ok(0) => panic!("origin socket closed before the pressure body completed"),
                Ok(count) => written += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if origin_exhausted.load(Ordering::Acquire) && !backpressure_reported {
                        assert!(
                            written < PRESSURE_BODY_BYTES,
                            "origin must be backpressured before handing the full body to the gateway"
                        );
                        backpressured_tx
                            .send(written)
                            .expect("backpressure observation channel should remain connected");
                        backpressure_reported = true;
                        resume_rx
                            .recv_timeout(Duration::from_secs(5))
                            .expect("client must restore connection credit within the gateway write budget");
                        origin
                            .set_nonblocking(false)
                            .expect("origin should return to blocking writes for recovery");
                        origin
                            .set_write_timeout(Some(Duration::from_secs(5)))
                            .expect("recovery write timeout should match the configured upstream write budget");
                    } else {
                        thread::yield_now();
                    }
                }
                Err(error) => panic!("pressure origin write failed: {error}"),
            }
        }
        assert!(
            backpressure_reported,
            "the 64 MiB origin body completed without downstream pressure reaching the origin socket"
        );
        origin.flush().expect("recovered pressure body should flush");
        origin_done.store(true, Ordering::Release);
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
        "bounded-read-ahead evidence must run on negotiated h2"
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
        &hpack_root_get(),
    );
    tls.flush().expect("pressure request should flush");

    let mut response_headers_seen = false;
    let mut body_bytes = 0_usize;
    for _ in 0..256 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(frame_type, H2_FRAME_GOAWAY, "connection must remain live");
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 1,
            "pressure stream must stall through flow control rather than reset"
        );
        if handle_control_frame(&mut tls, frame_type, flags, stream_id, &payload) {
            continue;
        }
        if stream_id == 1 && frame_type == H2_FRAME_HEADERS {
            response_headers_seen = true;
        }
        if stream_id == 1 && frame_type == H2_FRAME_DATA {
            assert!(response_headers_seen, "response DATA requires preceding headers");
            assert!(
                flags & H2_FLAG_END_STREAM == 0,
                "64 MiB response must not complete inside the initial connection window"
            );
            assert!(
                payload.iter().all(|byte| *byte == b'x'),
                "pressure response DATA must preserve origin bytes"
            );
            body_bytes += payload.len();
            assert!(
                body_bytes <= INITIAL_CONNECTION_WINDOW_BYTES,
                "server exceeded the RFC initial connection window without connection credit"
            );
            if body_bytes == INITIAL_CONNECTION_WINDOW_BYTES {
                break;
            }
        }
    }
    assert!(response_headers_seen, "pressure response headers must be observed");
    assert_eq!(
        body_bytes, INITIAL_CONNECTION_WINDOW_BYTES,
        "client must consume exactly the initial connection window before withholding credit"
    );

    connection_window_exhausted.store(true, Ordering::Release);
    assert!(
        !origin_completed.load(Ordering::Acquire),
        "origin handed the complete 64 MiB body to the gateway before downstream credit was restored"
    );
    let backpressured_at = backpressured_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("zero downstream connection credit must propagate bounded read-ahead to the origin");
    assert!(
        (INITIAL_CONNECTION_WINDOW_BYTES..PRESSURE_BODY_BYTES).contains(&backpressured_at),
        "origin backpressure must occur after real response progress but before full-body handoff: {backpressured_at}"
    );

    write_h2_frame(&mut tls, H2_FRAME_PING, 0, 0, PING_BARRIER);
    tls.flush()
        .expect("PING barrier should flush while response DATA remains connection-stalled");
    let mut ping_acked = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "bounded response pressure must not poison the H2 control plane"
        );
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 1,
            "bounded pressure must not reset the admitted response stream"
        );
        if frame_type == H2_FRAME_PING
            && stream_id == 0
            && flags & H2_FLAG_ACK != 0
            && payload.as_slice() == PING_BARRIER
        {
            ping_acked = true;
            break;
        }
        if handle_control_frame(&mut tls, frame_type, flags, stream_id, &payload) {
            continue;
        }
        if stream_id == 1 && frame_type == H2_FRAME_DATA {
            assert!(
                payload.is_empty(),
                "non-empty DATA must not cross an exhausted connection window before WINDOW_UPDATE"
            );
        }
    }
    assert!(
        ping_acked,
        "PING ACK must remain live while connection DATA credit is exhausted"
    );

    let remaining_body = PRESSURE_BODY_BYTES - INITIAL_CONNECTION_WINDOW_BYTES;
    write_window_update(&mut tls, 0, remaining_body);
    tls.flush()
        .expect("connection WINDOW_UPDATE should restore the exact remaining response credit");
    resume_tx
        .send(())
        .expect("origin recovery channel should remain connected");

    let mut ended = false;
    for _ in 0..8192 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "connection must remain live while bounded response pressure recovers"
        );
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 1,
            "recovered pressure stream must not reset"
        );
        if handle_control_frame(&mut tls, frame_type, flags, stream_id, &payload) {
            continue;
        }
        if stream_id == 1 && frame_type == H2_FRAME_DATA {
            assert!(
                payload.iter().all(|byte| *byte == b'x'),
                "recovered response DATA must preserve origin bytes"
            );
            body_bytes += payload.len();
        }
        if stream_id == 1 && flags & H2_FLAG_END_STREAM != 0 {
            ended = true;
            break;
        }
    }

    assert!(ended, "pressure response must complete after restoring connection credit");
    assert_eq!(body_bytes, PRESSURE_BODY_BYTES);
    origin_fixture
        .join()
        .expect("bounded-read-ahead origin fixture should complete");
    assert!(
        origin_completed.load(Ordering::Acquire),
        "origin must complete only after downstream connection credit is restored"
    );
}
