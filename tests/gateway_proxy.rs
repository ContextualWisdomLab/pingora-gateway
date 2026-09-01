use cwl_pingora_gateway::{
    edge_contract::{GatewayConfig, GatewayConfigError, UpstreamConfig, UpstreamTimeouts},
    gateway_proxy::{GatewayProxy, GatewayProxyError},
};
use pingora::upstreams::peer::{Peer, ALPN};
use std::net::SocketAddr;
use std::time::Duration;

fn upstream(name: &str, port: u16) -> UpstreamConfig {
    UpstreamConfig {
        name: name.to_string(),
        address: SocketAddr::from(([127, 0, 0, 1], port)),
        tls: false,
        sni: None,
        timeouts: UpstreamTimeouts {
            connection_ms: 1_000,
            total_connection_ms: 2_000,
            read_ms: 5_000,
            write_ms: 5_000,
            idle_ms: 10_000,
        },
    }
}

#[test]
fn version_one_proxy_builds_the_configured_peer_without_hidden_defaults() {
    let config = GatewayConfig {
        version: 1,
        listener: SocketAddr::from(([127, 0, 0, 1], 6188)),
        metrics_listener: SocketAddr::from(([127, 0, 0, 1], 6192)),
        max_request_body_bytes: 1_048_576,
        upstreams: vec![upstream("api", 8080)],
    };

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
    let config = GatewayConfig {
        version: 1,
        listener: SocketAddr::from(([127, 0, 0, 1], 6188)),
        metrics_listener: SocketAddr::from(([127, 0, 0, 1], 6188)),
        max_request_body_bytes: 1_048_576,
        upstreams: vec![upstream("api", 8080)],
    };

    let error = GatewayProxy::try_from_config(&config).unwrap_err();
    assert_eq!(
        error,
        GatewayProxyError::InvalidConfiguration(GatewayConfigError::ListenerCollision)
    );
}
