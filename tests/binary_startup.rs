//! Process-level startup characterization for the production gateway binary.
//!
//! These tests deliberately exercise the compiled binary instead of calling the library boundary
//! directly. A gateway process must not obtain network authority when the operator omits or points
//! `--config` at an invalid source.

use std::process::Command;

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
