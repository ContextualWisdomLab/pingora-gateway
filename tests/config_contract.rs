use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};

#[test]
fn parses_minimal_https_upstream_contract() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: true
    sni: api.internal.example
"#;

    let config = GatewayConfig::from_yaml(yaml).expect("valid gateway contract");

    assert_eq!(config.version, 1);
    assert_eq!(config.listener.to_string(), "127.0.0.1:6188");
    assert_eq!(config.upstreams.len(), 1);
    assert_eq!(config.upstreams[0].name, "api");
    assert!(config.upstreams[0].tls);
    assert_eq!(config.upstreams[0].sni.as_deref(), Some("api.internal.example"));
}

#[test]
fn rejects_unknown_contract_version() {
    let yaml = r#"
version: 2
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: false
"#;

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::UnsupportedVersion(2))
    );
}

#[test]
fn rejects_tls_upstream_without_sni() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8443
    tls: true
"#;

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::MissingTlsServerName {
            upstream_name: "api".to_string()
        })
    );
}

#[test]
fn rejects_empty_upstream_set() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
upstreams: []
"#;

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::NoUpstreams)
    );
}

#[test]
fn rejects_duplicate_upstream_names() {
    let yaml = r#"
version: 1
listener: 127.0.0.1:6188
upstreams:
  - name: api
    address: 127.0.0.1:8080
    tls: false
  - name: api
    address: 127.0.0.1:8081
    tls: false
"#;

    assert_eq!(
        GatewayConfig::from_yaml(yaml),
        Err(GatewayConfigError::DuplicateUpstreamName {
            upstream_name: "api".to_string()
        })
    );
}
