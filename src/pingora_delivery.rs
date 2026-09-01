//! Pingora delivery adapter for validated edge contracts.
//!
//! This module is intentionally the only place where the transport-neutral edge contract is
//! converted into Pingora connection objects. Consumer products should depend on the contract,
//! not on Pingora types.

use std::time::Duration;

use pingora::upstreams::peer::{HttpPeer, HttpUpstreamRequestPolicy, ALPN};

use crate::edge_contract::{GatewayConfigError, UpstreamConfig};

/// Builds a Pingora upstream peer from one validated edge-contract upstream.
///
/// The adapter revalidates the upstream so callers cannot bypass fail-closed contract checks by
/// constructing an [`UpstreamConfig`] directly. TLS certificate and hostname verification remain
/// enabled explicitly, HTTP/1.1 is the only accepted upstream protocol in the initial contract,
/// and every timeout comes from the versioned configuration rather than a hidden default.
pub fn build_peer(upstream: &UpstreamConfig) -> Result<HttpPeer, GatewayConfigError> {
    upstream.validate()?;
    Ok(build_peer_from_validated(upstream))
}

/// Constructs Pingora transport state after the enclosing gateway contract has already validated
/// this upstream.
///
/// Keeping this helper crate-private avoids repeating a logically impossible validation failure on
/// every proxied request while preserving [`build_peer`] as the fail-closed public entry point.
pub(crate) fn build_peer_from_validated(upstream: &UpstreamConfig) -> HttpPeer {
    let mut peer = HttpPeer::new(
        upstream.address,
        upstream.tls,
        upstream.sni.clone().unwrap_or_default(),
    );

    peer.options.verify_cert = true;
    peer.options.verify_hostname = true;
    peer.options.alpn = ALPN::H1;
    peer.options.connection_timeout = Some(Duration::from_millis(upstream.timeouts.connection_ms));
    peer.options.total_connection_timeout =
        Some(Duration::from_millis(upstream.timeouts.total_connection_ms));
    peer.options.read_timeout = Some(Duration::from_millis(upstream.timeouts.read_ms));
    peer.options.write_timeout = Some(Duration::from_millis(upstream.timeouts.write_ms));
    peer.options.idle_timeout = Some(Duration::from_millis(upstream.timeouts.idle_ms));
    peer.options.http_upstream_request_policy = HttpUpstreamRequestPolicy::standard();

    peer
}
