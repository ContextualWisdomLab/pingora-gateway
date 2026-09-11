//! Real-wire HTTP/2 cancellation acceptance for the generic downstream TLS composition root.
//!
//! The test admits two concurrent streams on one verified TLS/H2 connection, waits until the
//! sibling stream has begun its response, then cancels the still-pending stream with RST_STREAM
//! CANCEL. Acceptance requires the sibling stream to complete unchanged and the cancelled stream's
//! dedicated HTTP/1 origin connection to be released within a bounded interval.

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
const H2_FRAME_DATA: u8 = 0x0;
const H2_FRAME_HEADERS: u8 = 0x1;
const H2_FRAME_RST_STREAM: u8 = 0x3;
const H2_FRAME_SETTINGS: u8 = 0x4;
const H2_FLAG_ACK: u8 = 0x1;
const H2_FLAG_END_STREAM: u8 = 0x1;
const H2_ERROR_CANCEL: u32 = 0x8;

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
        "/CN=CWL Downstream H2 Reset Test CA",
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
    header[5..9].copy_from_slice(&(stream_id & 0x7fff_ffff).to_be_bytes());
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
    let stream_id = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7fff_ffff;
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .expect("H2 frame payload should be readable");
    (header[3], header[4], stream_id, payload)
}

fn hpack_get(path_index: u8) -> Vec<u8> {
    assert!(path_index == 0x84 || path_index == 0x85);
    let mut headers = vec![0x82, 0x87, path_index, 0x01, 0x0c];
    headers.extend_from_slice(b"gateway.test");
    headers
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
            .expect("upstream request should be readable before the fixture deadline");
        assert!(read > 0, "upstream closed before request headers completed");
        request.extend_from_slice(&buffer[..read]);
        assert!(
            request.len() <= MAX_ORIGIN_REQUEST_HEADER_BYTES,
            "upstream request headers exceeded the fixture bound"
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

fn require_cancelled_origin_release(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("cancelled origin release must be bounded");
    let mut byte = [0_u8; 1];
    match stream.read(&mut byte) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe
            ) => {}
        Ok(read) => panic!("cancelled origin received unexpected downstream bytes: {read}"),
        Err(error) => panic!("cancelled origin connection was not released within bound: {error}"),
    }
}

#[test]
fn cancelling_one_h2_stream_releases_its_origin_and_preserves_the_live_sibling() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");
    let (reset_flushed_tx, reset_flushed_rx) = mpsc::channel();

    let upstream_fixture = thread::spawn(move || {
        let (mut first, _) = upstream_listener
            .accept()
            .expect("first H2 stream should open an H1 origin connection");
        let first_request = read_request(&mut first);

        upstream_listener
            .set_nonblocking(true)
            .expect("second-connection acceptance must be bounded");
        let second_deadline = Instant::now() + Duration::from_secs(3);
        let mut second = loop {
            match upstream_listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < second_deadline,
                        "sibling H2 stream was serialized behind the still-pending stream"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("second origin accept failed: {error}"),
            }
        };
        let second_request = read_request(&mut second);

        let first_is_cancelled = first_request.starts_with("GET / HTTP/1.1\r\n");
        let second_is_cancelled = second_request.starts_with("GET / HTTP/1.1\r\n");
        assert_ne!(
            first_is_cancelled, second_is_cancelled,
            "exactly one origin request must correspond to cancelled stream path /: first={first_request:?}, second={second_request:?}"
        );
        let first_is_sibling = first_request.starts_with("GET /index.html HTTP/1.1\r\n");
        let second_is_sibling = second_request.starts_with("GET /index.html HTTP/1.1\r\n");
        assert_ne!(
            first_is_sibling, second_is_sibling,
            "exactly one origin request must correspond to sibling stream path /index.html: first={first_request:?}, second={second_request:?}"
        );

        if first_is_cancelled {
            write_origin_response(&mut second, "sibling-ok");
        } else {
            write_origin_response(&mut first, "sibling-ok");
        }
        reset_flushed_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("release evidence must start only after RST_STREAM(CANCEL) flush completes");
        if first_is_cancelled {
            require_cancelled_origin_release(&mut first);
        } else {
            require_cancelled_origin_release(&mut second);
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
        "cancellation evidence must run on one negotiated h2 connection"
    );

    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 connection preface should write");
    write_h2_frame(&mut tls, H2_FRAME_SETTINGS, 0, 0, &[]);
    write_h2_frame(&mut tls, H2_FRAME_HEADERS, 0x5, 1, &hpack_get(0x84));
    write_h2_frame(&mut tls, H2_FRAME_HEADERS, 0x5, 3, &hpack_get(0x85));
    tls.flush()
        .expect("both H2 requests must be sent before cancellation");

    let mut sibling_response_started = false;
    let mut sibling_body = Vec::new();
    let mut sibling_ended = false;
    let mut reset_sent = false;

    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            write_h2_frame(&mut tls, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            tls.flush().expect("SETTINGS acknowledgement should flush");
            continue;
        }

        assert!(
            frame_type != H2_FRAME_RST_STREAM || stream_id != 3,
            "cancelling stream 1 must not reset sibling stream 3"
        );

        if stream_id == 3 {
            if frame_type == H2_FRAME_HEADERS {
                sibling_response_started = true;
            }
            if frame_type == H2_FRAME_DATA {
                sibling_body.extend_from_slice(&payload);
            }
            if flags & H2_FLAG_END_STREAM != 0 {
                sibling_ended = true;
            }
        }

        if sibling_response_started && !reset_sent {
            write_h2_frame(
                &mut tls,
                H2_FRAME_RST_STREAM,
                0,
                1,
                &H2_ERROR_CANCEL.to_be_bytes(),
            );
            tls.flush().expect("RST_STREAM CANCEL should flush");
            reset_flushed_tx
                .send(())
                .expect("origin fixture should observe completed reset flush");
            reset_sent = true;
        }

        if sibling_ended {
            break;
        }
    }

    assert!(
        sibling_response_started,
        "origin fixture must prove both streams were dispatched before cancellation"
    );
    assert!(
        reset_sent,
        "stream 1 must be cancelled after sibling response starts"
    );
    assert!(
        sibling_ended,
        "stream 3 must complete after stream 1 cancellation"
    );
    assert_eq!(sibling_body, b"sibling-ok");

    upstream_fixture
        .join()
        .expect("cancellation/release origin fixture should complete");
}
