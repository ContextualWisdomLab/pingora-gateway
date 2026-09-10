//! Listener-lifecycle acceptance for downstream TLS delivery and coverage receipts.
//!
//! The real-wire protocol tests own handshake and proxy semantics. These tests keep instrumented
//! production binaries alive through listener construction and terminate them through Pingora's
//! normal SIGTERM path so child-process execution is persisted into exact-head coverage evidence.

#![cfg(unix)]

use std::fs;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::downstream_tls::DownstreamTlsConfigError;
use cwl_pingora_gateway::migration_admin::{PgErdMigrationConfig, PgErdMigrationConfigError};
use cwl_pingora_gateway::runtime_policy::V1_TERMINATION_BUDGET_SECONDS;
use tempfile::{tempdir, NamedTempFile};

struct LocalCertificate {
    _directory: tempfile::TempDir,
    certificate: PathBuf,
    private_key: PathBuf,
}

fn run_openssl(args: &[&str]) {
    let status = Command::new("openssl")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("CI must provide the explicitly installed openssl CLI");
    assert!(status.success(), "openssl command failed: {args:?}");
}

fn issue_certificate() -> LocalCertificate {
    let directory = tempdir().expect("certificate workspace should be available");
    let certificate = directory.path().join("server.crt");
    let private_key = directory.path().join("server.key");
    run_openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        private_key.to_str().expect("UTF-8 private-key path"),
        "-out",
        certificate.to_str().expect("UTF-8 certificate path"),
        "-subj",
        "/CN=gateway.test",
        "-days",
        "1",
        "-sha256",
    ]);
    LocalCertificate {
        _directory: directory,
        certificate,
        private_key,
    }
}

fn reserve_distinct_loopback_addresses() -> (SocketAddr, SocketAddr) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be available");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be available");
    let addresses = (
        traffic
            .local_addr()
            .expect("traffic reservation has an address"),
        metrics
            .local_addr()
            .expect("metrics reservation has an address"),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

fn reserve_loopback_address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .expect("fixture port should be available")
        .local_addr()
        .expect("fixture reservation has an address")
}

fn write_generic_config(
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    upstream: SocketAddr,
    certificate: &PathBuf,
    private_key: &PathBuf,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("generic config should be writable");
    writeln!(
        file,
        "version: 2\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 8\nservice_threads: 2\nupstream_keepalive_pool_size: 4\ndownstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: application\n    address: {upstream}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificate.display(),
        private_key.display(),
    )
    .expect("generic config should be written");
    file
}

fn write_pg_erd_config(
    version: u32,
    include_lifetime: bool,
    listener: SocketAddr,
    metrics_listener: SocketAddr,
    backend: SocketAddr,
    frontend: SocketAddr,
    certificate: &PathBuf,
    private_key: &PathBuf,
) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("pg-erd config should be writable");
    writeln!(file, "version: {version}").expect("version should be written");
    writeln!(file, "listener: {listener}").expect("listener should be written");
    writeln!(file, "metrics_listener: {metrics_listener}")
        .expect("metrics listener should be written");
    writeln!(file, "max_request_body_bytes: 1048576")
        .expect("request-body limit should be written");
    writeln!(file, "max_in_flight_requests: 8").expect("concurrency limit should be written");
    if include_lifetime {
        writeln!(file, "max_upstream_response_body_ms: 5000")
            .expect("response lifetime should be written");
    }
    writeln!(file, "service_threads: 2").expect("service threads should be written");
    writeln!(file, "upstream_keepalive_pool_size: 4").expect("pool size should be written");
    writeln!(
        file,
        "downstream_tls:\n  certificate_chain_file: {}\n  private_key_file: {}\n  alpn: h2_http1\nupstreams:\n  - name: backend\n    address: {backend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000\n  - name: frontend\n    address: {frontend}\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000",
        certificate.display(),
        private_key.display(),
    )
    .expect("pg-erd config should be written");
    file
}

fn spawn(binary: &str, config: &NamedTempFile) -> Child {
    Command::new(binary)
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

fn output(binary: &str, config: &NamedTempFile) -> Output {
    Command::new(binary)
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 config path"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("compiled gateway binary should run")
}

fn wait_until_listening(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway process exited before listener readiness: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "gateway did not expose its downstream TLS listener within 10s"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn terminate_gracefully(process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(V1_TERMINATION_BUDGET_SECONDS);
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.id().to_string()])
        .status()
        .expect("system kill command should send SIGTERM");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");

    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            assert!(
                status.success(),
                "SIGTERM graceful shutdown should exit successfully: {status}"
            );
            return;
        }
        assert!(
            Instant::now() < deadline,
            "TLS listener did not terminate inside the external hard-kill budget"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn both_tls_composition_roots_construct_listeners_and_exit_through_graceful_shutdown() {
    let certificate = issue_certificate();

    let (generic_listener, generic_metrics) = reserve_distinct_loopback_addresses();
    let generic_config = write_generic_config(
        generic_listener,
        generic_metrics,
        reserve_loopback_address(),
        &certificate.certificate,
        &certificate.private_key,
    );
    let mut generic = spawn(env!("CARGO_BIN_EXE_cwl-pingora-gateway"), &generic_config);
    wait_until_listening(generic_listener, &mut generic);
    terminate_gracefully(&mut generic);

    let (pg_listener, pg_metrics) = reserve_distinct_loopback_addresses();
    let pg_config = write_pg_erd_config(
        3,
        true,
        pg_listener,
        pg_metrics,
        reserve_loopback_address(),
        reserve_loopback_address(),
        &certificate.certificate,
        &certificate.private_key,
    );
    let mut pg = spawn(
        env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"),
        &pg_config,
    );
    wait_until_listening(pg_listener, &mut pg);
    terminate_gracefully(&mut pg);
}

#[test]
fn both_tls_composition_roots_fail_closed_when_certificate_material_cannot_be_loaded() {
    let certificate = issue_certificate();
    let missing_certificate = certificate.certificate.with_file_name("missing.crt");

    let (generic_listener, generic_metrics) = reserve_distinct_loopback_addresses();
    let generic_config = write_generic_config(
        generic_listener,
        generic_metrics,
        reserve_loopback_address(),
        &missing_certificate,
        &certificate.private_key,
    );
    let generic = output(env!("CARGO_BIN_EXE_cwl-pingora-gateway"), &generic_config);
    assert!(!generic.status.success());
    assert!(
        String::from_utf8_lossy(&generic.stderr).contains("downstream TLS"),
        "generic startup error should identify the downstream TLS boundary"
    );

    let (pg_listener, pg_metrics) = reserve_distinct_loopback_addresses();
    let pg_config = write_pg_erd_config(
        3,
        true,
        pg_listener,
        pg_metrics,
        reserve_loopback_address(),
        reserve_loopback_address(),
        &missing_certificate,
        &certificate.private_key,
    );
    let pg = output(
        env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"),
        &pg_config,
    );
    assert!(!pg.status.success());
    assert!(
        String::from_utf8_lossy(&pg.stderr).contains("downstream TLS"),
        "pg-erd startup error should identify the downstream TLS boundary"
    );
}

#[test]
fn pg_erd_tls_version_and_material_validation_remain_fail_closed() {
    let certificate = issue_certificate();
    let (listener, metrics_listener) = reserve_distinct_loopback_addresses();
    let backend = reserve_loopback_address();
    let frontend = reserve_loopback_address();

    let legacy = write_pg_erd_config(
        1,
        false,
        listener,
        metrics_listener,
        backend,
        frontend,
        &certificate.certificate,
        &certificate.private_key,
    );
    let legacy_yaml = fs::read_to_string(legacy.path()).expect("legacy fixture should be readable");
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&legacy_yaml),
        Err(PgErdMigrationConfigError::DownstreamTlsRequiresVersion3)
    );

    let (listener, metrics_listener) = reserve_distinct_loopback_addresses();
    let missing_lifetime = write_pg_erd_config(
        3,
        false,
        listener,
        metrics_listener,
        backend,
        frontend,
        &certificate.certificate,
        &certificate.private_key,
    );
    let missing_lifetime_yaml =
        fs::read_to_string(missing_lifetime.path()).expect("lifetime fixture should be readable");
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&missing_lifetime_yaml),
        Err(PgErdMigrationConfigError::MissingUpstreamResponseBodyLifetime)
    );

    let (listener, metrics_listener) = reserve_distinct_loopback_addresses();
    let relative_certificate = PathBuf::from("relative.crt");
    let invalid_material = write_pg_erd_config(
        3,
        true,
        listener,
        metrics_listener,
        backend,
        frontend,
        &relative_certificate,
        &certificate.private_key,
    );
    let invalid_material_yaml =
        fs::read_to_string(invalid_material.path()).expect("material fixture should be readable");
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&invalid_material_yaml),
        Err(PgErdMigrationConfigError::DownstreamTls(
            DownstreamTlsConfigError::RelativeCertificateChainFile
        ))
    );
}
