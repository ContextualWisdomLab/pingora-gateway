use std::io::Write;
use std::process::Command;

use tempfile::NamedTempFile;

#[test]
fn binary_fails_before_listeners_when_tls_trust_bundle_cannot_be_read() {
    let mut config = NamedTempFile::new().expect("temporary gateway config");
    writeln!(
        config,
        "version: 1\nlistener: 127.0.0.1:6188\nmetrics_listener: 127.0.0.1:6192\nmax_request_body_bytes: 1048576\nupstreams:\n  - name: api\n    address: 127.0.0.1:8443\n    tls: true\n    sni: api.internal.example\n    trust_bundle_file: /definitely/missing/cwl-local-ca.pem\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000"
    )
    .expect("gateway config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args([
            "--config",
            config.path().to_str().expect("UTF-8 config path"),
        ])
        .output()
        .expect("compiled gateway binary should be executable");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unable to read TLS trust bundle"),
        "trust material must fail closed before listeners; got {stderr:?}"
    );
}
