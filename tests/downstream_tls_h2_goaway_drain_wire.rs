//! Real-wire HTTP/2 graceful-shutdown acceptance for the generic downstream TLS root.
//!
//! The fixture holds two admitted HTTP/2 streams open across the configured SIGTERM grace period,
//! requires the server to begin HTTP/2 drain with GOAWAY(NO_ERROR, 2^31-1), then releases both
//! origins and requires both admitted streams to complete before process exit. A second request is
//! sent after SIGTERM but before the client observes GOAWAY, proving the configured grace period
//! still admits work while the later GOAWAY boundary remains explicit.

#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::runtime_policy::{
    V1_GRACE_PERIOD_SECONDS, V1_TERMINATION_BUDGET_SECONDS,
};
use pingora::tls::ssl::{SslConnector, SslMethod, SslVerifyMode};
use tempfile::{tempdir, NamedTempFile};

const MAX_ORIGIN_REQUEST_HEADER_BYTES: usize = 64 * 1024;
const H2_FRAME_DATA: u8 = 0x0;
const H2_FRAME_HEADERS: u8 = 0x1;
const H2_FRAME_RST_STREAM: u8 = 0x3;
const H2_FRAME_SETTINGS: u8 = 0x4;
const H2_FRAME_GOAWAY: u8 = 0x7;
const H2_FLAG_ACK: u8 = 0x1;
const H2_FLAG_END_STREAM: u8 = 0x1;
const H2_ERROR_NO_ERROR: u32 = 0x0;
const H2_MAX_STREAM_ID: u32 = 0x7fff_ffff;

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
        "/CN=CWL Downstream H2 GOAWAY Test CA",
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
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 10000\n      write_ms: 5000\n      idle_ms: 10000",
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
                .set_read_timeout(Some(Duration::from_secs(V1_GRACE_PERIOD_SECONDS + 5)))
                .expect("downstream read timeout should cover the configured grace period");
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

fn parse_goaway(payload: &[u8]) -> (u32, u32) {
    assert!(
        payload.len() >= 8,
        "GOAWAY payload must contain last_stream_id and error code"
    );
    let last_stream_id = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
        & H2_MAX_STREAM_ID;
    let error_code = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
    (last_stream_id, error_code)
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

fn wait_for_exit(process: &mut Child, deadline: Instant) -> std::process::ExitStatus {
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not terminate before the external hard-kill budget"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn sigterm_h2_goaway_drains_admitted_streams_and_bounds_new_work() {
    let certificates = issue_gateway_certificate();
    let upstream_listener = TcpListener::bind("127.0.0.1:0").expect("upstream should bind");
    let upstream = upstream_listener.local_addr().expect("upstream address");

    let (first_seen_tx, first_seen_rx) = mpsc::channel();
    let (second_seen_tx, second_seen_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let upstream_fixture = thread::spawn(move || {
        let (mut first, _) = upstream_listener
            .accept()
            .expect("stream 1 should open an H1 origin connection");
        let first_request = read_request(&mut first);
        assert!(
            first_request.starts_with("GET / HTTP/1.1\r\n"),
            "stream 1 should reach the expected origin path: {first_request:?}"
        );
        first_seen_tx
            .send(())
            .expect("test controller should observe stream 1 admission");

        upstream_listener
            .set_nonblocking(true)
            .expect("second origin acceptance must be bounded");
        let second_deadline = Instant::now() + Duration::from_secs(3);
        let mut second = loop {
            match upstream_listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < second_deadline,
                        "stream 3 was not admitted during the configured SIGTERM grace period"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("second origin accept failed: {error}"),
            }
        };
        let second_request = read_request(&mut second);
        assert!(
            second_request.starts_with("GET /index.html HTTP/1.1\r\n"),
            "stream 3 should reach the expected origin path: {second_request:?}"
        );
        second_seen_tx
            .send(())
            .expect("test controller should observe stream 3 admission");

        release_rx
            .recv_timeout(Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS))
            .expect("controller should release admitted responses before hard-kill budget");
        write_origin_response(&mut first, "first-ok");
        write_origin_response(&mut second, "second-ok");
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
        "GOAWAY acceptance must run on a negotiated h2 connection"
    );

    tls.write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .expect("H2 connection preface should write");
    write_h2_frame(&mut tls, H2_FRAME_SETTINGS, 0, 0, &[]);
    write_h2_frame(&mut tls, H2_FRAME_HEADERS, 0x5, 1, &hpack_get(0x84));
    tls.flush().expect("stream 1 request should flush");
    first_seen_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("stream 1 should reach origin before SIGTERM");

    let signal_sent_at = Instant::now();
    let termination_deadline = signal_sent_at + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.0.id().to_string()])
        .status()
        .expect("system kill command should send SIGTERM");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");

    write_h2_frame(&mut tls, H2_FRAME_HEADERS, 0x5, 3, &hpack_get(0x85));
    tls.flush()
        .expect("stream 3 should be sent during the configured grace period");
    second_seen_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("stream 3 should be admitted before H2 drain begins");

    let mut initial_goaway = None;
    for _ in 0..64 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            write_h2_frame(&mut tls, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            tls.flush().expect("SETTINGS acknowledgement should flush");
            continue;
        }
        assert!(
            frame_type != H2_FRAME_RST_STREAM || (stream_id != 1 && stream_id != 3),
            "graceful shutdown must not reset either admitted stream"
        );
        if frame_type == H2_FRAME_GOAWAY {
            assert_eq!(stream_id, 0, "GOAWAY is a connection-level frame");
            initial_goaway = Some(parse_goaway(&payload));
            break;
        }
    }

    let (initial_last_stream_id, initial_error_code) =
        initial_goaway.expect("SIGTERM must initiate HTTP/2 drain with GOAWAY");
    assert_eq!(
        initial_last_stream_id, H2_MAX_STREAM_ID,
        "initial graceful GOAWAY should advertise the RFC 9113 race-safe maximum stream id"
    );
    assert_eq!(
        initial_error_code, H2_ERROR_NO_ERROR,
        "administrative graceful shutdown should use GOAWAY(NO_ERROR)"
    );
    assert!(
        signal_sent_at.elapsed() >= Duration::from_secs(V1_GRACE_PERIOD_SECONDS),
        "GOAWAY must not bypass the configured pre-shutdown grace period"
    );

    release_tx
        .send(())
        .expect("admitted origins should be released after initial GOAWAY evidence");

    let mut first_body = Vec::new();
    let mut second_body = Vec::new();
    let mut first_ended = false;
    let mut second_ended = false;
    let mut final_goaway = None;

    for _ in 0..128 {
        let (frame_type, flags, stream_id, payload) = read_h2_frame(&mut tls);
        if frame_type == H2_FRAME_SETTINGS && stream_id == 0 && flags & H2_FLAG_ACK == 0 {
            write_h2_frame(&mut tls, H2_FRAME_SETTINGS, H2_FLAG_ACK, 0, &[]);
            tls.flush().expect("SETTINGS acknowledgement should flush");
            continue;
        }
        assert!(
            frame_type != H2_FRAME_RST_STREAM || (stream_id != 1 && stream_id != 3),
            "graceful drain must preserve admitted streams"
        );
        if frame_type == H2_FRAME_DATA && stream_id == 1 {
            first_body.extend_from_slice(&payload);
        }
        if frame_type == H2_FRAME_DATA && stream_id == 3 {
            second_body.extend_from_slice(&payload);
        }
        if stream_id == 1 && flags & H2_FLAG_END_STREAM != 0 {
            first_ended = true;
        }
        if stream_id == 3 && flags & H2_FLAG_END_STREAM != 0 {
            second_ended = true;
        }
        if frame_type == H2_FRAME_GOAWAY {
            let (last_stream_id, error_code) = parse_goaway(&payload);
            assert_eq!(
                error_code, H2_ERROR_NO_ERROR,
                "final graceful GOAWAY should retain NO_ERROR"
            );
            assert!(
                last_stream_id <= initial_last_stream_id,
                "GOAWAY last_stream_id must never increase"
            );
            if last_stream_id != H2_MAX_STREAM_ID {
                final_goaway = Some(last_stream_id);
            }
        }
        if first_ended && second_ended && final_goaway.is_some() {
            break;
        }
    }

    assert!(first_ended, "stream 1 must finish during graceful H2 drain");
    assert!(second_ended, "stream 3 must finish during graceful H2 drain");
    assert_eq!(first_body, b"first-ok");
    assert_eq!(second_body, b"second-ok");
    assert_eq!(
        final_goaway,
        Some(3),
        "final GOAWAY must identify the highest client stream that may have been processed"
    );

    upstream_fixture
        .join()
        .expect("GOAWAY/drain origin fixture should complete");
    let exit_status = wait_for_exit(&mut process.0, termination_deadline);
    assert!(
        exit_status.success(),
        "SIGTERM graceful H2 shutdown should exit successfully: {exit_status}"
    );
}
