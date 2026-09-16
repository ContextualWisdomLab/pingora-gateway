use cwl_pingora_gateway::edge_contract::{GatewayConfigError, UpstreamConfig, UpstreamTimeouts};

fn tls_upstream(sni: &str) -> UpstreamConfig {
    UpstreamConfig {
        name: "api".to_string(),
        address: "127.0.0.1:8443"
            .parse()
            .expect("loopback TLS upstream must parse"),
        tls: true,
        sni: Some(sni.to_string()),
        trust_bundle_file: None,
        timeouts: UpstreamTimeouts {
            connection_ms: 100,
            total_connection_ms: 200,
            read_ms: 300,
            write_ms: 400,
            idle_ms: 500,
        },
    }
}

#[test]
fn tls_sni_must_be_an_rfc_6066_dns_hostname() {
    let overlong_label = format!("{}.example", "a".repeat(64));
    let overlong_name = [
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(63),
    ]
    .join(".");
    let invalid_snis = vec![
        "127.0.0.1".to_string(),
        "2001:db8::1".to_string(),
        "api.internal.example.".to_string(),
        "api internal.example".to_string(),
        "_api.internal.example".to_string(),
        "-api.internal.example".to_string(),
        "api-.internal.example".to_string(),
        "api..internal.example".to_string(),
        "예.internal.example".to_string(),
        overlong_label,
        overlong_name,
    ];

    for invalid_sni in invalid_snis {
        assert_eq!(
            tls_upstream(&invalid_sni).validate(),
            Err(GatewayConfigError::InvalidTlsServerName {
                upstream_name: "api".to_string(),
            }),
            "TLS SNI must reject invalid RFC 6066 HostName {invalid_sni:?} before network authority"
        );
    }
}

#[test]
fn tls_sni_admits_ascii_dns_and_idna_a_labels() {
    let max_length_name = [
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61),
    ]
    .join(".");
    let valid_snis = vec![
        "api.internal.example".to_string(),
        "API.INTERNAL.EXAMPLE".to_string(),
        "api-1.internal.example".to_string(),
        "xn--bcher-kva.example".to_string(),
        format!("{}.example", "a".repeat(63)),
        max_length_name,
    ];

    for valid_sni in valid_snis {
        tls_upstream(&valid_sni)
            .validate()
            .unwrap_or_else(|error| panic!("valid RFC 6066 HostName {valid_sni:?}: {error}"));
    }
}
