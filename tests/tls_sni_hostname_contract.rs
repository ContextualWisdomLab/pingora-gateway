use cwl_pingora_gateway::edge_contract::{UpstreamConfig, UpstreamTimeouts};

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
    for invalid_sni in [
        "127.0.0.1",
        "2001:db8::1",
        "api.internal.example.",
        "api internal.example",
        "_api.internal.example",
        "-api.internal.example",
        "api-.internal.example",
        "api..internal.example",
        "예.internal.example",
    ] {
        assert!(
            tls_upstream(invalid_sni).validate().is_err(),
            "TLS SNI must reject invalid RFC 6066 HostName {invalid_sni:?} before network authority"
        );
    }
}

#[test]
fn tls_sni_admits_ascii_dns_and_idna_a_labels() {
    for valid_sni in [
        "api.internal.example",
        "api-1.internal.example",
        "xn--bcher-kva.example",
    ] {
        tls_upstream(valid_sni)
            .validate()
            .unwrap_or_else(|error| panic!("valid RFC 6066 HostName {valid_sni:?}: {error}"));
    }
}
