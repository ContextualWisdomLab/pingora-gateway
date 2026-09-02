//! Process-level fail-closed startup contract for the dedicated pg-erd migration binary.

use std::io::Write;
use std::process::Command;

use tempfile::NamedTempFile;

fn migration_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cwl-pingora-pg-erd-migration"))
}

fn config_with_backend_trust_bundle(trust_bundle_file: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary migration config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: 127.0.0.1:18080\nmetrics_listener: 127.0.0.1:18082\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 4\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: 127.0.0.1:18081\n    tls: true\n    sni: backend.example\n    trust_bundle_file: {trust_bundle_file}\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n  - name: frontend\n    address: 127.0.0.1:18083\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500"
    )
    .expect("migration config should be written");
    file
}

fn config_with_backend_address(backend_address: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary migration config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: 127.0.0.1:18080\nmetrics_listener: 127.0.0.1:18082\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 4\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: {backend_address}\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n  - name: frontend\n    address: 127.0.0.1:18083\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500"
    )
    .expect("migration config should be written");
    file
}

#[test]
fn migration_binary_fails_closed_when_config_is_omitted() {
    let output = migration_command()
        .output()
        .expect("compiled migration binary should be executable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing required --config <path> option"));
}

#[test]
fn migration_binary_fails_closed_when_config_file_cannot_be_read() {
    let output = migration_command()
        .args([
            "--config",
            "/definitely/missing/cwl-pingora-pg-erd-migration.yaml",
        ])
        .output()
        .expect("compiled migration binary should be executable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unable to read pg-erd migration configuration"));
}

#[test]
fn migration_binary_fails_closed_when_admin_contract_is_invalid() {
    let mut file = NamedTempFile::new().expect("temporary migration config should be writable");
    writeln!(file, "version: 2\nproduct_auth_mode: embedded")
        .expect("invalid migration config should be written");

    let output = migration_command()
        .args(["--config", file.path().to_str().expect("UTF-8 temp path")])
        .output()
        .expect("compiled migration binary should be executable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("pg-erd migration configuration is not valid YAML"));
}

#[test]
fn migration_binary_fails_closed_when_transport_trust_cannot_be_materialized() {
    let file = config_with_backend_trust_bundle("/definitely/missing/cwl-pingora-ca.pem");
    let output = migration_command()
        .args(["--config", file.path().to_str().expect("UTF-8 temp path")])
        .output()
        .expect("compiled migration binary should be executable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unable to read TLS trust bundle"));
}

#[test]
fn migration_binary_fails_closed_when_backend_overlaps_traffic_listener() {
    let file = config_with_backend_address("127.0.0.1:18080");
    let output = migration_command()
        .args(["--config", file.path().to_str().expect("UTF-8 temp path")])
        .output()
        .expect("compiled migration binary should be executable");

    assert!(
        !output.status.success(),
        "migration binary must reject gateway self-loop authority before opening listeners"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("upstream backend socket authority must not overlap listener"),
        "startup error should identify the rejected backend authority; got {stderr:?}"
    );
}
