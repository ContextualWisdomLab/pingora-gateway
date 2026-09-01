use cwl_pingora_gateway::{
    edge_contract::UpstreamConfig,
    pingora_delivery::{
        build_peer, DEFAULT_CONNECTION_TIMEOUT, DEFAULT_IDLE_TIMEOUT, DEFAULT_READ_TIMEOUT,
        DEFAULT_TOTAL_CONNECTION_TIMEOUT, DEFAULT_WRITE_TIMEOUT,
    },
};
use std::net::SocketAddr;

fn upstream(tls: bool, sni: Option<&str>) -> UpstreamConfig {
    UpstreamConfig {
        name: "api".to_string(),
        address: "127.0.0.1:8443".parse::<SocketAddr>().unwrap(),
        tls,
        sni: sni.map(str::to_string),
    }
}

#[test]
fn tls_peer_verifies_identity_and_bounds_upstream_io() {
    let peer = build_peer(&upstream(true, Some("api.internal.example")));

    assert!(peer.is_tls());
    assert_eq!(peer.sni, "api.internal.example");
    assert!(peer.options.verify_cert);
    assert!(peer.options.verify_hostname);
    assert_eq!(peer.options.connection_timeout, Some(DEFAULT_CONNECTION_TIMEOUT));
    assert_eq!(
        peer.options.total_connection_timeout,
        Some(DEFAULT_TOTAL_CONNECTION_TIMEOUT)
    );
    assert_eq!(peer.options.read_timeout, Some(DEFAULT_READ_TIMEOUT));
    assert_eq!(peer.options.write_timeout, Some(DEFAULT_WRITE_TIMEOUT));
    assert_eq!(peer.options.idle_timeout, Some(DEFAULT_IDLE_TIMEOUT));
    assert!(peer.options.http_upstream_request_policy.strip_hop_by_hop);
    assert!(
        peer.options
            .http_upstream_request_policy
            .strip_connection_nominated
    );
    assert!(
        peer.options
            .http_upstream_request_policy
            .reject_malformed_connection_nominations
    );
}

#[test]
fn cleartext_peer_does_not_invent_a_tls_identity() {
    let peer = build_peer(&upstream(false, None));

    assert!(!peer.is_tls());
    assert!(peer.sni.is_empty());
}
