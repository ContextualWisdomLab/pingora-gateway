//! Config-only acceptance through the compiled binary, without granting listener authority.

use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Reaps the owned child even when an assertion or deadline fails.
struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Captures the small CLI result with a bounded wait, preventing accidental serve-mode hangs.
fn run_gateway(args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"));
    command.args(args);
    run_command(command, Duration::from_secs(5))
}

/// Bounds each owned child by its operation-specific budget; gateway execution retains five seconds.
fn run_command(mut command: Command, timeout: Duration) -> Output {
    let mut process = GatewayProcess(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("compiled gateway must start"),
    );
    let deadline = Instant::now() + timeout;
    loop {
        if process
            .0
            .try_wait()
            .expect("child state is readable")
            .is_some()
        {
            let stdout = process.0.stdout.take().expect("stdout is captured");
            let stderr = process.0.stderr.take().expect("stderr is captured");
            use std::io::Read;
            let mut output = Output {
                status: process.0.wait().expect("child is reapable"),
                stdout: Vec::new(),
                stderr: Vec::new(),
            };
            stdout
                .take(64 * 1024)
                .read_to_end(&mut output.stdout)
                .unwrap();
            stderr
                .take(64 * 1024)
                .read_to_end(&mut output.stderr)
                .unwrap();
            return output;
        }
        assert!(
            Instant::now() < deadline,
            "command did not exit within {timeout:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

/// Writes the same strict cleartext contract that normal serving accepts.
fn config_source(listener: SocketAddr, metrics: SocketAddr) -> String {
    format!(
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics}\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: fixture\n    address: 127.0.0.1:1\n    tls: false\n    timeouts:\n      connection_ms: 1000\n      total_connection_ms: 2000\n      read_ms: 5000\n      write_ms: 5000\n      idle_ms: 10000\n"
    )
}

/// Checks both documented flag orderings against identical local inputs.
fn check_in_both_orders(path: &Path) -> [Output; 2] {
    let path = path.to_str().expect("fixture config path is UTF-8");
    [
        run_gateway(&["--check", "--config", path]),
        run_gateway(&["--config", path, "--check"]),
    ]
}

/// Requires check mode and normal serving to reject identical local inputs identically.
fn assert_same_failure(path: &Path, diagnostic: &str, sentinel: &str) {
    let normal = run_gateway(&["--config", path.to_str().unwrap()]);
    assert_eq!(normal.status.code(), Some(2));
    assert!(normal.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&normal.stderr);
    assert!(stderr.contains(diagnostic), "{stderr}");
    assert!(!stderr.contains(sentinel), "{stderr}");
    for output in check_in_both_orders(path) {
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, normal.stderr);
    }
}

#[test]
fn check_rejects_the_same_strict_config_failures_as_serving() {
    let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let path = directory.path().join("gateway.yaml");
    let valid = config_source(
        "127.0.0.1:6188".parse().unwrap(),
        "127.0.0.1:6192".parse().unwrap(),
    );
    for source in [
        "[invalid yaml config-credential-sentinel".to_string(),
        format!("{valid}unknown_field: config-credential-sentinel\n"),
        valid.replace("max_request_body_bytes: 8", "max_request_body_bytes: 0"),
        valid
            .replace("name: fixture", "name: config-credential-sentinel")
            .replace("connection_ms: 1000", "connection_ms: 0"),
    ] {
        fs::write(&path, source).unwrap();
        assert_same_failure(&path, "gateway configuration", "config-credential-sentinel");
    }
}

#[test]
fn check_rejects_unreadable_config_like_serving() {
    let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    assert_same_failure(
        &directory.path().join("missing.yaml"),
        "unable to read gateway configuration",
        "config-credential-sentinel",
    );
    assert_same_failure(
        directory.path(),
        "unable to read gateway configuration",
        "config-credential-sentinel",
    );
}

#[test]
fn check_rejects_missing_repeated_and_unknown_arguments() {
    for (args, diagnostic) in [
        (vec!["--check"], "missing required --config"),
        (
            vec!["--check", "--config"],
            "--config requires a path value",
        ),
        (vec!["--config"], "--config requires a path value"),
        (
            vec!["--check", "--config", "a.yaml", "--config", "b.yaml"],
            "--config may be specified exactly once",
        ),
        (
            vec!["--config", "a.yaml", "--check", "--check"],
            "unexpected startup argument",
        ),
        (
            vec!["--check", "--config", "a.yaml", "--unknown"],
            "unexpected startup argument",
        ),
        (
            vec!["--unknown", "--check", "--config", "a.yaml"],
            "unexpected startup argument",
        ),
        (
            vec!["--check", "--config", "a.yaml", "extra"],
            "unexpected startup argument",
        ),
    ] {
        let output = run_gateway(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(diagnostic),
            "{args:?}"
        );
    }
}

/// Converts the cleartext fixture into an explicit custom-trust TLS input, without a TLS server.
fn tls_config_source(listener: SocketAddr, metrics: SocketAddr, trust: &Path) -> String {
    config_source(listener, metrics).replace(
        "    tls: false",
        &format!(
            "    tls: true\n    sni: origin.test\n    trust_bundle_file: {}",
            trust.display()
        ),
    )
}

#[test]
fn check_loads_valid_custom_trust_without_binding_or_contacting_origin() {
    let traffic = TcpListener::bind("127.0.0.1:0").unwrap();
    let metrics = TcpListener::bind("127.0.0.1:0").unwrap();
    let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let certificate = directory.path().join("ca.pem");
    let key = directory.path().join("ca.key");
    let mut openssl = Command::new("openssl");
    openssl.args([
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        key.to_str().unwrap(),
        "-out",
        certificate.to_str().unwrap(),
        "-subj",
        "/CN=Preflight Fixture CA",
        "-days",
        "1",
        "-sha256",
    ]);
    // RSA fixture generation has variable runtime and is not part of the gateway's five-second
    // preflight exit contract. Keep this test-only setup separately bounded without relaxing any
    // gateway invocation, startup, shutdown, or repository release gate.
    let issued = run_command(openssl, Duration::from_secs(30));
    assert!(
        issued.status.success(),
        "{}",
        String::from_utf8_lossy(&issued.stderr)
    );
    let path = directory.path().join("gateway.yaml");
    fs::write(
        &path,
        tls_config_source(
            traffic.local_addr().unwrap(),
            metrics.local_addr().unwrap(),
            &certificate,
        ),
    )
    .unwrap();
    for output in check_in_both_orders(&path) {
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"gateway configuration is valid\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn check_rejects_the_same_unreadable_empty_and_malformed_trust_as_serving() {
    let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let path = directory.path().join("gateway.yaml");
    let trust = directory.path().join("ca.pem");
    let write_config = |trust: &Path| {
        fs::write(
            &path,
            tls_config_source(
                "127.0.0.1:6188".parse().unwrap(),
                "127.0.0.1:6192".parse().unwrap(),
                trust,
            ),
        )
        .unwrap()
    };
    write_config(&trust);
    assert_same_failure(
        &path,
        "unable to read TLS trust bundle",
        "trust-credential-sentinel",
    );
    write_config(directory.path());
    assert_same_failure(
        &path,
        "unable to read TLS trust bundle",
        "trust-credential-sentinel",
    );
    write_config(&trust);
    for material in [
        "",
        "trust-credential-sentinel",
        "-----BEGIN CERTIFICATE-----\ntrust-credential-sentinel\n-----END CERTIFICATE-----\n",
    ] {
        fs::write(&trust, material).unwrap();
        assert_same_failure(
            &path,
            "invalid TLS trust bundle",
            "trust-credential-sentinel",
        );
    }
}

#[test]
fn command_selects_check_only_explicitly_and_preserves_config_path() {
    use cwl_pingora_gateway::startup::GatewayCommand;
    use std::ffi::OsString;
    for (arguments, check_only) in [
        (vec!["gateway", "--config", "gateway.yaml"], false),
        (vec!["gateway", "--check", "--config", "gateway.yaml"], true),
        (vec!["gateway", "--config", "gateway.yaml", "--check"], true),
    ] {
        let arguments: Vec<OsString> = arguments.into_iter().map(OsString::from).collect();
        let command = GatewayCommand::parse(&arguments).unwrap();
        assert_eq!(command.check_only(), check_only);
        assert_eq!(command.config_path(), Path::new("gateway.yaml"));
    }
}

#[test]
fn invalid_yaml_diagnostics_do_not_echo_configuration_values() {
    let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let path = directory.path().join("gateway.yaml");
    fs::write(&path, "version: config-credential-sentinel\n").unwrap();
    let normal = run_gateway(&["--config", path.to_str().unwrap()]);
    for output in check_in_both_orders(&path).into_iter().chain([normal]) {
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("gateway configuration"));
        assert!(!stderr.contains("config-credential-sentinel"), "{stderr}");
    }
}

#[test]
fn config_flag_followed_by_an_option_is_a_missing_path() {
    for args in [
        vec!["--config", "--check"],
        vec!["--check", "--config", "--unknown"],
        vec!["--config", "--config", "gateway.yaml"],
        vec!["--check", "--config", ""],
    ] {
        let output = run_gateway(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            output.stderr, b"--config requires a path value\n",
            "{args:?}"
        );
    }
}

#[test]
fn duplicate_check_is_rejected_before_loading_config() {
    let output = run_gateway(&["--check", "--check", "--config", "not-read.yaml"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"unexpected startup argument: --check\n");
}

#[test]
fn check_succeeds_with_both_listener_ports_already_occupied() {
    let traffic = TcpListener::bind("127.0.0.1:0").unwrap();
    let metrics = TcpListener::bind("127.0.0.1:0").unwrap();
    let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let path = directory.path().join("gateway.yaml");
    fs::write(
        &path,
        config_source(traffic.local_addr().unwrap(), metrics.local_addr().unwrap()),
    )
    .unwrap();

    for output in check_in_both_orders(&path) {
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"gateway configuration is valid\n");
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
