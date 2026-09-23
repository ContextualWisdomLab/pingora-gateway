use cwl_pingora_gateway::{
    edge_contract::{GatewayConfig, GatewayConfigError, UpstreamConfig, UpstreamTimeouts},
    gateway_proxy::{GatewayProxy, GatewayProxyError},
    pingora_delivery::PeerBuildError,
};
use pingora::upstreams::peer::{Peer, ALPN};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

fn upstream(name: &str, port: u16) -> UpstreamConfig {
    UpstreamConfig {
        name: name.to_string(),
        address: SocketAddr::from(([127, 0, 0, 1], port)),
        tls: false,
        sni: None,
        trust_bundle_file: None,
        timeouts: UpstreamTimeouts {
            connection_ms: 1_000,
            total_connection_ms: 2_000,
            read_ms: 5_000,
            write_ms: 5_000,
            idle_ms: 10_000,
        },
    }
}

fn gateway_config(upstream: UpstreamConfig) -> GatewayConfig {
    GatewayConfig {
        version: 1,
        listener: SocketAddr::from(([127, 0, 0, 1], 6188)),
        metrics_listener: SocketAddr::from(([127, 0, 0, 1], 6192)),
        max_request_body_bytes: 1_048_576,
        max_in_flight_requests: 128,
        upstream_keepalive_pool_size: 32,
        upstreams: vec![upstream],
    }
}

#[test]
fn version_one_proxy_builds_the_configured_peer_without_hidden_defaults() {
    let config = gateway_config(upstream("api", 8080));

    let proxy = GatewayProxy::try_from_config(&config).expect("single upstream is unambiguous");
    let peer = proxy.build_upstream_peer();

    assert_eq!(peer.address().to_string(), "127.0.0.1:8080");
    assert_eq!(peer.options.alpn, ALPN::H1);
    assert_eq!(
        peer.options.read_timeout,
        Some(Duration::from_millis(5_000))
    );
}

#[test]
fn proxy_activation_revalidates_programmatically_constructed_contracts() {
    let mut config = gateway_config(upstream("api", 8080));
    config.metrics_listener = config.listener;

    let error = GatewayProxy::try_from_config(&config).unwrap_err();
    assert_eq!(
        error,
        GatewayProxyError::InvalidConfiguration(GatewayConfigError::ListenerCollision)
    );
}

#[test]
fn proxy_activation_fails_before_listeners_when_trust_material_is_missing() {
    let mut tls = upstream("api", 8443);
    tls.tls = true;
    tls.sni = Some("api.internal.example".to_string());
    tls.trust_bundle_file = Some(PathBuf::from("/definitely/missing/cwl-local-ca.pem"));
    let config = gateway_config(tls);

    assert!(matches!(
        GatewayProxy::try_from_config(&config).unwrap_err(),
        GatewayProxyError::UpstreamActivation(PeerBuildError::ReadTrustBundle { .. })
    ));
}
