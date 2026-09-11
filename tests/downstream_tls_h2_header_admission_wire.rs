//! Real-wire HTTP/2 decoded-header admission acceptance for the generic downstream TLS root.
//!
//! Pingora 0.9.0 bounds received decoded header lists at 64 KiB. This fixture proves the gateway
//! advertises that setting, rejects an oversized field section before origin contact, preserves the
//! HTTP/2 connection, and still admits a following compliant sibling stream.

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

const MAX_ORIGIN_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const H2_DEFAULT_MAX_FRAME_SIZE: usize = 16 * 1024;
const H2_MAX_STREAM_ID: u32 = 0x7fff_ffff;
const H2_FRAME_DATA: u8 = 0x0;
const H2_FRAME_HEADERS: u8 = 0x1;
const H2_FRAME_RST_STREAM: u8 = 0x3;
const H2_FRAME_SETTINGS: u8 = 0x4;
const H2_FRAME_GOAWAY: u8 = 0x7;
const H2_FRAME_CONTINUATION: u8 = 0x9;
const H2_FLAG_ACK: u8 = 0x1;
const H2_FLAG_END_STREAM: u8 = 0x1;
const H2_FLAG_END_HEADERS: u8 = 0x4;
const SETTINGS_MAX_HEADER_LIST_SIZE: u16 = 0x6;
const EXPECTED_MAX_HEADER_LIST_SIZE: u32 = 64 * 1024;
const OVERSIZED_VALUE_BYTES: usize = 65_500;

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
        "/CN=CWL Downstream H2 Header Admission Test CA",
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
    let stream_id = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & H2_MAX_STREAM_ID;
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .expect("H2 frame payload should be readable");
    (header[3], header[4], stream_id, payload)
}

fn settings_value(payload: &[u8], identifier: u16) -> Option<u32> {
    assert_eq!(
        payload.len() % 6,
        0,
        "SETTINGS payload must contain complete six-byte entries"
    );
    payload.chunks_exact(6).find_map(|entry| {
        let id = u16::from_be_bytes([entry[0], entry[1]]);
        (id == identifier).then(|| u32::from_be_bytes([entry[2], entry[3], entry[4], entry[5]]))
    })
}

fn acknowledge_server_settings_and_require_header_limit(stream: &mut impl ReadWrite) {
    for _ in 0..16 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(stream);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "server must not close a fresh compliant H2 connection"
        );
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            assert_eq!(
                settings_value(&payload, SETTINGS_MAX_HEADER_LIST_SIZE),
                Some(EXPECTED_MAX_HEADER_LIST_SIZE),
                "gateway must advertise Pingora's bounded 64 KiB decoded header-list limit"
            );
            write_h2_frame(stream, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            stream.flush().expect("SETTINGS acknowledgement should flush");
            return;
        }
    }
    panic!("server SETTINGS with decoded-header bound was not observed");
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn push_hpack_integer(out: &mut Vec<u8>, prefix_bits: u8, prefix_base: u8, mut value: usize) {
    let prefix_max = (1_usize << prefix_bits) - 1;
    assert_eq!(prefix_base as usize & prefix_max, 0);
    if value < prefix_max {
        out.push(prefix_base | value as u8);
        return;
    }
    out.push(prefix_base | prefix_max as u8);
    value -= prefix_max;
    while value >= 128 {
        out.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn push_hpack_string(out: &mut Vec<u8>, value: &[u8]) {
    push_hpack_integer(out, 7, 0, value.len());
    out.extend_from_slice(value);
}

fn hpack_get(path_index: u8) -> Vec<u8> {
    assert!(path_index == 0x84 || path_index == 0x85);
    let mut headers = vec![0x82, 0x87, path_index, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    headers
}

fn oversized_hpack_get() -> Vec<u8> {
    let mut headers = hpack_get(0x84);
    headers.push(0x00);
    push_hpack_string(&mut headers, b"x-cwl-pad");
    push_hpack_string(&mut headers, &vec![b'a'; OVERSIZED_VALUE_BYTES]);
    headers
}

fn write_header_block(stream: &mut impl Write, stream_id: u32, block: &[u8]) {
    let chunks: Vec<&[u8]> = block.chunks(H2_DEFAULT_MAX_FRAME_SIZE).collect();
    assert!(!chunks.is_empty());
    for (index, chunk) in chunks.iter().enumerate() {
        let first = index == 0;
        let last = index + 1 == chunks.len();
        let frame_type = if first {
            H2_FRAME_HEADERS
        } else {
            H2_FRAME_CONTINUATION
        };
        let mut flags = 0;
        if first {
            flags |= H2_FLAG_END_STREAM;
        }
        if last {
            flags |= H2_FLAG_END_HEADERS;
        }
        write_h2_frame(stream, frame_type, flags, stream_id, chunk);
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "upstream request headers exceeded the fixture deadline"
        );
        stream
            .set_read_timeout(Some(remaining))
            .expect("upstream read deadline should be set");
        let read = stream
            .read(&mut buffer)
            .expect("upstream request should be readable before fixture deadline");
        assert!(read > 0, "upstream closed before request headers completed");
        request.extend_from_slice(&buffer[..read]);
        assert!(
            request.len() <= MAX_ORIGIN_REQUEST_HEADER_BYTES,
            "upstream request headers exceeded fixture bound"
        );
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&request).into_owned();
        }
    }
}

fn write_origin_response(stream: &mut TcpStream, body: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("upstream response should be writable");
    stream.flush().expect("upstream response should flush");
}

#[test]
fn decoded_header_limit_rejects_before_origin_and_preserves_connection() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");
    let (oversize_rejected_tx, oversize_rejected_rx) = mpsc::channel();

    let upstream_fixture = thread::spawn(move || {
        upstream_listener
            .set_nonblocking(true)
            .expect("origin observation must remain bounded");
        let rejection_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match oversize_rejected_rx.try_recv() {
                Ok(()) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    panic!("client disappeared before proving oversize rejection")
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
            match upstream_listener.accept() {
                Ok(_) => panic!("oversized decoded header list must be rejected before origin contact"),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => panic!("unexpected origin accept failure: {error}"),
            }
            assert!(
                Instant::now() < rejection_deadline,
                "oversized-header rejection evidence did not arrive within bound"
            );
            thread::sleep(Duration::from_millis(10));
        }

        let sibling_deadline = Instant::now() + Duration::from_secs(5);
        let mut sibling = loop {
            match upstream_listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < sibling_deadline,
                        "compliant sibling request did not reach origin after oversize rejection"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("sibling origin accept failed: {error}"),
            }
        };
        let request = read_request(&mut sibling);
        assert!(
            request.starts_with("GET /index.html HTTP/1.1\r\n"),
            "only the compliant sibling may reach origin: {request:?}"
        );
        write_origin_response(&mut sibling, "sibling-ok");
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
        "decoded-header admission evidence must run on negotiated h2"
    );

    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 connection preface should write");
    write_h2_frame(&mut tls, H2_FRAME_SETTINGS, 0, 0, &[]);
    tls.flush().expect("client SETTINGS should flush");
    acknowledge_server_settings_and_require_header_limit(&mut tls);

    let oversized = oversized_hpack_get();
    write_header_block(&mut tls, 1, &oversized);
    tls.flush().expect("oversized header block should flush");

    let mut oversized_response_seen = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, _payload) = read_h2_frame(&mut tls);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "stream-local decoded-header rejection must preserve the H2 connection"
        );
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            write_h2_frame(&mut tls, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            tls.flush().expect("late SETTINGS acknowledgement should flush");
            continue;
        }
        if stream_id == 1 && frame_type == H2_FRAME_HEADERS {
            assert!(
                flags & H2_FLAG_END_STREAM != 0,
                "oversized request rejection response must terminate stream 1"
            );
            oversized_response_seen = true;
            break;
        }
    }
    assert!(
        oversized_response_seen,
        "oversized decoded header list must receive a stream-local rejection response"
    );
    oversize_rejected_tx
        .send(())
        .expect("origin fixture should observe oversize rejection evidence");

    write_h2_frame(&mut tls, H2_FRAME_HEADERS, 0x5, 3, &hpack_get(0x85));
    tls.flush()
        .expect("compliant sibling request should flush on preserved connection");

    let mut sibling_body = Vec::new();
    let mut sibling_ended = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "connection must remain usable after decoded-header rejection"
        );
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 3,
            "compliant sibling stream must not be reset"
        );
        if stream_id == 3 && frame_type == H2_FRAME_DATA {
            sibling_body.extend_from_slice(&payload);
        }
        if stream_id == 3 && flags & H2_FLAG_END_STREAM != 0 {
            sibling_ended = true;
            break;
        }
    }

    assert!(
        sibling_ended,
        "compliant sibling stream must complete after oversized-header rejection"
    );
    assert_eq!(sibling_body, b"sibling-ok");
    upstream_fixture
        .join()
        .expect("decoded-header admission origin fixture should complete");
}
