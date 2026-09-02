//! Process-level startup characterization for the production gateway binary.
//!
//! These tests deliberately exercise the compiled binary instead of calling the library boundary
//! directly. A gateway process must not obtain network authority when the operator omits or points
//! `--config` at an invalid source.

use std::io::Write;
use std::process::Command;

use tempfile::NamedTempFile;

fn gateway_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
}

#[test]
fn binary_fails_closed_when_config_is_omitted() {
    let output = gateway_command()
        .output()
        .expect("compiled gateway binary should be executable");

    assert!(
        !output.status.success(),
        "gateway binary must fail before opening a listener without --config"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing required --config <path> option"),
        "stderr should identify the rejected startup contract; got {stderr:?}"
    );
}

#[test]
fn binary_fails_closed_when_config_file_cannot_be_read() {
    let output = gateway_command()
        .args(["--config", "/definitely/missing/cwl-pingora-gateway.yaml"])
        .output()
        .expect("compiled gateway binary should be executable");

    assert!(
        !output.status.success(),
        "gateway binary must fail before opening a listener when config cannot be read"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unable to read gateway configuration"),
        "stderr should preserve the fail-closed configuration boundary; got {stderr:?}"
    );
}

#[test]
fn binary_fails_closed_when_upstream_overlaps_traffic_listener() {
    let mut config = NamedTempFile::new().expect("temporary config should be writable");
    write!(
        config,
        "version: 1\nlistener: 127.0.0.1:6188\nmetrics_listener: 127.0.0.1:6192\nmax_request_body_bytes: 1048576\nmax_in_flight_requests: 128\nupstream_keepalive_pool_size: 32\nupstreams:\n  - name: api\n    address: 127.0.0.1:6188\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n"
    )
    .expect("self-loop config fixture should be written");

    let output = gateway_command()
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 temporary path"),
        ])
        .output()
        .expect("compiled gateway binary should be executable");

    assert!(
        !output.status.success(),
        "gateway binary must reject listener/upstream self-loop before server activation"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("upstream api socket authority must not overlap listener"),
        "startup error should identify the rejected network authority; got {stderr:?}"
    );
}
