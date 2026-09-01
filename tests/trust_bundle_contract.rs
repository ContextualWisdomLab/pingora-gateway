use cwl_pingora_gateway::edge_contract::{GatewayConfig, GatewayConfigError};

fn yaml(tls: bool, sni_line: &str, trust_bundle_line: &str) -> String {
    format!(
        "version: 1\nlistener: 127.0.0.1:6188\nmetrics_listener: 127.0.0.1:6192\nmax_request_body_bytes: 1048576\nupstreams:\n  - name: api\n    address: 127.0.0.1:8443\n    tls: {tls}\n{sni_line}{trust_bundle_line}    timeouts:\n      connection_ms: 1250\n      total_connection_ms: 2500\n      read_ms: 7500\n      write_ms: 6500\n      idle_ms: 15000\n"
    )
}

#[test]
fn parses_absolute_tls_trust_bundle_path() {
    let config = GatewayConfig::from_yaml(&yaml(
        true,
        "    sni: api.internal.example\n",
        "    trust_bundle_file: /etc/cwl/upstream-ca.pem\n",
    ))
    .expect("absolute trust bundle path is an admitted TLS contract");

    assert_eq!(
        config.upstreams[0]
            .trust_bundle_file
            .as_deref()
            .expect("trust bundle path should be preserved")
            .to_string_lossy(),
        "/etc/cwl/upstream-ca.pem"
    );
}

#[test]
fn rejects_empty_tls_trust_bundle_path() {
    assert_eq!(
        GatewayConfig::from_yaml(&yaml(
            true,
            "    sni: api.internal.example\n",
            "    trust_bundle_file: \"\"\n",
        )),
        Err(GatewayConfigError::EmptyTrustBundlePath {
            upstream_name: "api".to_string(),
        })
    );
}

#[test]
fn rejects_relative_tls_trust_bundle_path() {
    assert_eq!(
        GatewayConfig::from_yaml(&yaml(
            true,
            "    sni: api.internal.example\n",
            "    trust_bundle_file: certs/upstream-ca.pem\n",
        )),
        Err(GatewayConfigError::RelativeTrustBundlePath {
            upstream_name: "api".to_string(),
        })
    );
}

#[test]
fn rejects_trust_bundle_on_cleartext_upstream() {
    assert_eq!(
        GatewayConfig::from_yaml(&yaml(
            false,
            "",
            "    trust_bundle_file: /etc/cwl/upstream-ca.pem\n",
        )),
        Err(GatewayConfigError::UnexpectedTrustBundle {
            upstream_name: "api".to_string(),
        })
    );
}
