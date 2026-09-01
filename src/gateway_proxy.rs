//! Initial executable proxy application for the shared gateway.
//!
//! Version 1 activates one upstream per process because the transport-neutral edge contract owns
//! that invariant. This Pingora adapter does not invent routing or load-balancing domain rules.

use async_trait::async_trait;
use pingora::prelude::{Error, ErrorType, HttpPeer, ProxyHttp, Session};
use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError, UpstreamConfig};
use crate::pingora_delivery::build_peer;

/// Activation failures that occur after the transport-neutral configuration is parsed.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GatewayProxyError {
    /// The edge configuration itself violates a fail-closed invariant.
    #[error("invalid edge configuration: {0}")]
    InvalidConfiguration(#[from] GatewayConfigError),
}

/// Pingora HTTP application backed by one explicitly configured upstream.
#[derive(Debug, Clone)]
pub struct GatewayProxy {
    upstream: UpstreamConfig,
}

impl GatewayProxy {
    /// Builds the version-1 delivery adapter from a validated edge configuration.
    ///
    /// Contract validation owns upstream-count and network-authority rules. The adapter only
    /// copies the admitted upstream value into Pingora-facing state.
    pub fn try_from_config(config: &GatewayConfig) -> std::result::Result<Self, GatewayProxyError> {
        config.validate()?;
        let upstream = config
            .upstreams
            .first()
            .cloned()
            .ok_or(GatewayConfigError::NoUpstreams)?;

        Ok(Self { upstream })
    }

    /// Constructs a fresh Pingora peer using the versioned upstream network-authority contract.
    pub fn build_upstream_peer(&self) -> std::result::Result<HttpPeer, GatewayConfigError> {
        build_peer(&self.upstream)
    }
}

#[async_trait]
impl ProxyHttp for GatewayProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        self.build_upstream_peer().map(Box::new).map_err(|error| {
            Error::because(
                ErrorType::InternalError,
                "validated edge contract could not construct upstream peer",
                error,
            )
        })
    }
}
