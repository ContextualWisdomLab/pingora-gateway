//! Initial executable proxy application for the shared gateway.
//!
//! Version 1 activates one upstream per process because the transport-neutral edge contract owns
//! that invariant. This Pingora adapter does not invent routing or load-balancing domain rules.

use async_trait::async_trait;
use bytes::Bytes;
use pingora::prelude::{Error, ErrorType, HttpPeer, ProxyHttp, RequestHeader, ResponseHeader, Session};
use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError, UpstreamConfig};
use crate::pingora_delivery::build_peer;

/// Stable process-local liveness endpoint.
pub const LIVENESS_PATH: &str = "/livez";
/// Stable readiness endpoint reached through the production Pingora serving path.
pub const READINESS_PATH: &str = "/readyz";

/// Per-request delivery state. Product domain state does not belong here.
#[derive(Debug, Default)]
pub struct RequestContext {
    request_body_bytes: u64,
}

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
    max_request_body_bytes: u64,
}

impl GatewayProxy {
    /// Builds the version-1 delivery adapter from a validated edge configuration.
    ///
    /// Contract validation owns upstream-count and network-authority rules. The adapter only
    /// copies admitted values into Pingora-facing state.
    pub fn try_from_config(config: &GatewayConfig) -> std::result::Result<Self, GatewayProxyError> {
        config.validate()?;
        let upstream = config
            .upstreams
            .first()
            .cloned()
            .ok_or(GatewayConfigError::NoUpstreams)?;

        Ok(Self {
            upstream,
            max_request_body_bytes: config.max_request_body_bytes,
        })
    }

    /// Constructs a fresh Pingora peer using the versioned upstream network-authority contract.
    pub fn build_upstream_peer(&self) -> std::result::Result<HttpPeer, GatewayConfigError> {
        build_peer(&self.upstream)
    }

    async fn respond_healthy(session: &mut Session) -> pingora::Result<()> {
        let mut response = ResponseHeader::build(200, None)?;
        response.insert_header("Content-Length", "0")?;
        response.insert_header("Cache-Control", "no-store")?;
        session
            .write_response_header(Box::new(response), true)
            .await
    }

    fn reject_oversize_declared_body(&self, session: &Session) -> pingora::Result<()> {
        let Some(value) = session.req_header().headers.get("content-length") else {
            return Ok(());
        };
        let raw = value.to_str().map_err(|_| {
            Error::explain(
                ErrorType::HTTPStatus(400),
                "Content-Length is not valid visible ASCII",
            )
        })?;
        let declared = raw.parse::<u64>().map_err(|_| {
            Error::explain(
                ErrorType::HTTPStatus(400),
                "Content-Length is not a valid unsigned integer",
            )
        })?;
        if declared > self.max_request_body_bytes {
            return Err(Error::explain(
                ErrorType::HTTPStatus(413),
                "request body exceeds configured max_request_body_bytes",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl ProxyHttp for GatewayProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::default()
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        match session.req_header().uri.path() {
            LIVENESS_PATH | READINESS_PATH => {
                Self::respond_healthy(session).await?;
                Ok(true)
            }
            _ => {
                self.reject_oversize_declared_body(session)?;
                Ok(false)
            }
        }
    }

    async fn request_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        let chunk_bytes = body.as_ref().map_or(0_u64, |chunk| chunk.len() as u64);
        ctx.request_body_bytes = ctx.request_body_bytes.saturating_add(chunk_bytes);
        if ctx.request_body_bytes > self.max_request_body_bytes {
            return Err(Error::explain(
                ErrorType::HTTPStatus(413),
                "streamed request body exceeds configured max_request_body_bytes",
            ));
        }
        Ok(())
    }

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

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        for header in [
            "Forwarded",
            "X-Forwarded-For",
            "X-Forwarded-Host",
            "X-Forwarded-Proto",
            "X-Real-IP",
        ] {
            upstream_request.remove_header(header);
        }
        upstream_request.insert_header("Forwarded", "proto=http")?;
        Ok(())
    }
}
