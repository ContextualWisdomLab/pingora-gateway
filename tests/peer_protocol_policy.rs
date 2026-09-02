//! Transport-delivery regression for the current fail-closed HTTP/1 protocol-transition contract.
//!
//! Request admission already rejects uncharacterized Upgrade attempts. The immutable Pingora peer
//! must encode the same boundary so a later callback/refactor cannot silently re-enable supplier
//! WebSocket forwarding below the gateway-owned admission policy.

use cwl_pingora_gateway::edge_contract::{UpstreamConfig, UpstreamTimeouts};
use cwl_pingora_gateway::pingora_delivery::build_peer;
use pingora::upstreams::peer::HttpUpstreamRequestPolicy;

fn cleartext_upstream() -> UpstreamConfig {
    UpstreamConfig {
        name: "origin".to_string(),
        address: "127.0.0.1:18081"
            .parse()
            .expect("loopback upstream must parse"),
        tls: false,
        sni: None,
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
fn validated_peer_denies_http1_upgrades_at_transport_boundary() {
    let peer = build_peer(&cleartext_upstream()).expect("valid upstream should build a peer");

    assert_eq!(
        peer.options.http_upstream_request_policy,
        HttpUpstreamRequestPolicy::deny_upgrades(),
        "the delivery peer must not retain Pingora's WebSocketOnly default beneath the gateway's fail-closed Upgrade admission"
    );
}
