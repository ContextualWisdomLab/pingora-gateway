//! Real-wire HTTP/2 request-body admission acceptance for the generic downstream TLS root.
//!
//! Production request-body policy is protocol-neutral: declared and streamed bodies share the
//! configured `max_request_body_bytes` budget. This fixture exercises both paths over negotiated
//! HTTP/2 and proves that HTTP-level rejection does not poison a compliant sibling stream.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
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
const REQUEST_BODY_LIMIT_BYTES: usize = 8;
const OVERSIZED_BODY: &[u8] = b"123456789";
const MAX_ORIGIN_REQUEST_HEADER_BYTES: usize = 64 * 1024;

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
        "/CN=CWL Downstream H2 Body Admission Test CA",
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
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: {REQUEST_BODY_LIMIT_BYTES}\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
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

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn hpack_base_request(method_index: u8, path_index: u8) -> Vec<u8> {
    assert!(method_index == 0x82 || method_index == 0x83);
    assert!(path_index == 0x84 || path_index == 0x85);
    let mut headers = vec![method_index, 0x87, path_index, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    headers
}

fn declared_oversized_post_headers() -> Vec<u8> {
    let mut headers = hpack_base_request(0x83, 0x84);
    // HPACK static-table index 28 is `content-length`; the value is the decimal body length.
    headers.extend_from_slice(&[0x0f, 0x0d, 0x01, b'9']);
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
            "origin request headers exceeded the fixture bound"
        );
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8_lossy(&request).into_owned();
        }
    }
}

fn wait_for_stream_end(stream: &mut impl ReadWrite, target_stream_id: u32) {
    let mut response_headers_seen = false;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, _payload) = read_h2_frame(stream);
        assert_ne!(
            frame_type, H2_FRAME_GOAWAY,
            "request-body rejection must remain stream-local"
        );
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            write_h2_frame(stream, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            stream
                .flush()
                .expect("late SETTINGS acknowledgement should flush");
            continue;
        }
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != target_stream_id,
            "request-body policy must surface as an HTTP response rather than resetting the stream"
        );
        if stream_id == target_stream_id && frame_type == H2_FRAME_HEADERS {
            response_headers_seen = true;
        }
        if stream_id == target_stream_id && flags & H2_FLAG_END_STREAM != 0 {
            assert!(
                response_headers_seen,
                "request-body rejection must include HTTP response headers"
            );
            return;
        }
    }
    panic!("target stream did not receive a bounded HTTP-level rejection response");
}

#[test]
fn declared_and_streamed_body_limits_reject_without_poisoning_h2_connection() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");

    let origin_fixture = thread::spawn(move || {
        upstream_listener
            .set_nonblocking(true)
            .expect("origin acceptance must remain bounded");
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut held_streamed_requests = Vec::new();
        loop {
            match upstream_listener.accept() {
                Ok((mut connection, _)) => {
                    let request = read_origin_headers(&mut connection);
                    if request.starts_with("GET /index.html HTTP/1.1\r\n") {
                        connection
                            .write_all(
                                b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nsibling-ok",
                            )
                            .expect("sibling origin response should be writable");
                        connection.flush().expect("sibling origin response should flush");
                        return;
                    }
                    assert!(
                        request.starts_with("POST / HTTP/1.1\r\n"),
                        "only the streamed oversized request may contact origin before sibling recovery: {request:?}"
                    );
                    held_streamed_requests.push(connection);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "compliant sibling did not reach origin within the configured read budget"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("origin accept failed: {error}"),
            }
        }
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
        "body-admission evidence must run on negotiated h2"
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
        &declared_oversized_post_headers(),
    );
    tls.flush().expect("declared oversized request should flush");
    wait_for_stream_end(&mut tls, 1);

    write_h2_frame(
        &mut tls,
        H2_FRAME_HEADERS,
        H2_FLAG_END_HEADERS,
        3,
        &hpack_base_request(0x83, 0x84),
    );
    write_h2_frame(
        &mut tls,
        H2_FRAME_DATA,
        H2_FLAG_END_STREAM,
        3,
        OVERSIZED_BODY,
    );
    tls.flush().expect("streamed oversized request should flush");
    wait_for_stream_end(&mut tls, 3);

    write_h2_frame(
        &mut tls,
        H2_FRAME_HEADERS,
        H2_FLAG_END_STREAM | H2_FLAG_END_HEADERS,
        5,
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
            "connection must remain usable after request-body rejections"
        );
        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 5,
            "compliant sibling stream must not be reset"
        );
        if stream_id == 5 && frame_type == H2_FRAME_HEADERS {
            sibling_headers_seen = true;
        }
        if stream_id == 5 && frame_type == H2_FRAME_DATA {
            sibling_body.extend_from_slice(&payload);
        }
        if stream_id == 5 && flags & H2_FLAG_END_STREAM != 0 {
            sibling_ended = true;
            break;
        }
    }

    assert!(sibling_headers_seen, "compliant sibling must receive response headers");
    assert!(sibling_ended, "compliant sibling must complete on the same H2 connection");
    assert_eq!(sibling_body, b"sibling-ok");
    origin_fixture
        .join()
        .expect("H2 request-body origin fixture should complete");
}
