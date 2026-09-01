use cwl_pingora_gateway::startup::{GatewayCommand, GatewayCommandError};
use std::ffi::OsString;
use std::fs;

#[test]
fn command_requires_an_explicit_config_path() {
    let args = [OsString::from("cwl-pingora-gateway")];

    assert_eq!(
        GatewayCommand::parse(args).unwrap_err(),
        GatewayCommandError::MissingConfigOption
    );
}

#[test]
fn command_rejects_unknown_or_ambiguous_arguments() {
    let unknown = [
        OsString::from("cwl-pingora-gateway"),
        OsString::from("--listen"),
        OsString::from("127.0.0.1:6188"),
    ];
    assert_eq!(
        GatewayCommand::parse(unknown).unwrap_err(),
        GatewayCommandError::UnexpectedArgument("--listen".to_string())
    );

    let duplicate = [
        OsString::from("cwl-pingora-gateway"),
        OsString::from("--config"),
        OsString::from("a.yaml"),
        OsString::from("--config"),
        OsString::from("b.yaml"),
    ];
    assert_eq!(
        GatewayCommand::parse(duplicate).unwrap_err(),
        GatewayCommandError::DuplicateConfigOption
    );
}

#[test]
fn command_loads_and_validates_the_explicit_config_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gateway.yaml");
    fs::write(
        &path,
        r#"
version: 1
listener: 127.0.0.1:6188
metrics_listener: 127.0.0.1:6192
max_request_body_bytes: 1048576
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: false
    timeouts:
      connection_ms: 1000
      total_connection_ms: 2000
      read_ms: 5000
      write_ms: 5000
      idle_ms: 10000
"#,
    )
    .unwrap();

    let args = [
        OsString::from("cwl-pingora-gateway"),
        OsString::from("--config"),
        path.clone().into_os_string(),
    ];
    let command = GatewayCommand::parse(args).expect("explicit config option must parse");
    let config = command.load_config().expect("valid config file must load");

    assert_eq!(command.config_path(), path.as_path());
    assert_eq!(config.listener.to_string(), "127.0.0.1:6188");
    assert_eq!(config.metrics_listener.to_string(), "127.0.0.1:6192");
    assert_eq!(config.max_request_body_bytes, 1_048_576);
}

#[test]
fn command_does_not_hide_a_missing_config_file() {
    let path = std::path::PathBuf::from("does-not-exist.yaml");
    let args = [
        OsString::from("cwl-pingora-gateway"),
        OsString::from("--config"),
        path.clone().into_os_string(),
    ];
    let command = GatewayCommand::parse(args).expect("argument shape is valid");

    let error = command.load_config().unwrap_err();
    assert_eq!(error.path(), Some(path.as_path()));
}
