//! Initial executable proxy application for the shared gateway.
//!
//! Version 1 activates one upstream per process because the transport-neutral edge contract owns
//! that invariant. This Pingora adapter does not invent routing or load-balancing domain rules.

use std::sync::LazyLock;

use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use pingora::prelude::{Error, ErrorType, HttpPeer, ProxyHttp, RequestHeader, ResponseHeader, Session};
use pingora_prometheus::prometheus::{register_int_counter, IntCounter};
use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError};
use crate::pingora_delivery::build_peer_from_validated;

/// Stable process-local liveness endpoint.
pub const LIVENESS_PATH: &str = "/livez";
/// Stable readiness endpoint reached through the production Pingora serving path.
pub const READINESS_PATH: &str = "/readyz";

static REQUESTS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "cwl_pingora_gateway_requests_total",
        "Completed downstream requests observed by the shared edge runtime"
    )
    .expect("gateway request metric must register exactly once")
});

static REQUEST_ERRORS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "cwl_pingora_gateway_request_errors_total",
        "Completed downstream requests whose Pingora lifecycle ended with an error"
    )
    .expect("gateway request error metric must register exactly once")
});

static REQUEST_BODY_BYTES_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "cwl_pingora_gateway_request_body_bytes_total",
        "Downstream request body bytes observed before completion or rejection"
    )
    .expect("gateway request body byte metric must register exactly once")
});

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
    upstream_peer: HttpPeer,
    max_request_body_bytes: u64,
}

impl GatewayProxy {
    /// Builds the version-1 delivery adapter from a validated edge configuration.
    ///
    /// Contract validation owns upstream-count and network-authority rules. The adapter constructs
    /// immutable Pingora transport state once during activation rather than repeating validation on
    /// every proxied request.
    pub fn try_from_config(config: &GatewayConfig) -> std::result::Result<Self, GatewayProxyError> {
        config.validate()?;
        let upstream = &config.upstreams[0];

        Ok(Self {
            upstream_peer: build_peer_from_validated(upstream),
            max_request_body_bytes: config.max_request_body_bytes,
        })
    }

    /// Returns a fresh clone of the prevalidated Pingora peer for one upstream connection attempt.
    pub fn build_upstream_peer(&self) -> HttpPeer {
        self.upstream_peer.clone()
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
        Ok(Box::new(self.build_upstream_peer()))
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

    async fn logging(&self, session: &mut Session, error: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        let status = session
            .response_written()
            .map_or(0, |response| response.status.as_u16());
        let outcome = if error.is_some() { "error" } else { "ok" };

        REQUESTS_TOTAL.inc();
        REQUEST_BODY_BYTES_TOTAL.inc_by(ctx.request_body_bytes);
        if error.is_some() {
            REQUEST_ERRORS_TOTAL.inc();
        }

        info!(
            "gateway_request status={status} outcome={outcome} request_body_bytes={}",
            ctx.request_body_bytes
        );
    }
}
