//! Initial executable proxy application for the shared gateway.
//!
//! Version 1 intentionally activates only one upstream per process. The configuration contract can
//! describe more than one explicit upstream for forward compatibility, but the runtime refuses an
//! ambiguous selection until a separately versioned route or load-balancing contract is accepted.

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
    /// Version 1 has no implicit routing or load-balancing policy and therefore accepts one target.
    #[error("version 1 requires exactly one upstream; received {actual}")]
    UnsupportedUpstreamCount {
        /// Number of configured upstreams presented for activation.
        actual: usize,
    },
}

/// Pingora HTTP application backed by one explicitly configured upstream.
#[derive(Debug, Clone)]
pub struct GatewayProxy {
    upstream: UpstreamConfig,
}

impl GatewayProxy {
    /// Builds the version-1 proxy application from an already parsed edge configuration.
    ///
    /// The constructor revalidates the public contract and then refuses to invent routing or load
    /// balancing semantics when more than one upstream is present.
    pub fn try_from_config(config: &GatewayConfig) -> std::result::Result<Self, GatewayProxyError> {
        config.validate()?;
        if config.upstreams.len() != 1 {
            return Err(GatewayProxyError::UnsupportedUpstreamCount {
                actual: config.upstreams.len(),
            });
        }

        Ok(Self {
            upstream: config.upstreams[0].clone(),
        })
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
