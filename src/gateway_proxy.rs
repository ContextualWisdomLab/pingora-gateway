//! Initial executable proxy application for the shared gateway.
//!
//! Version 1 activates one upstream per process because the transport-neutral edge contract owns
//! that invariant. This Pingora adapter does not invent routing or load-balancing domain rules.

use async_trait::async_trait;
use bytes::Bytes;
use pingora::prelude::{Error, ErrorType, HttpPeer, ProxyHttp, RequestHeader, Session};
use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError};
use crate::observability::{record_backpressure_rejection, record_request};
use crate::pingora_delivery::{build_peer_from_validated, PeerBuildError};
use crate::process_health::respond_healthy;
pub use crate::process_health::{LIVENESS_PATH, READINESS_PATH};
use crate::runtime_isolation::{
    BodyLimitExceeded, RequestAdmission, RequestAdmissionBudget, RequestBodyBudget,
    RuntimeIsolationLimits,
};

/// Per-request delivery state. Product domain state does not belong here.
#[derive(Debug)]
pub struct RequestContext {
    request_body: RequestBodyBudget,
    admission: Option<RequestAdmission>,
}

impl RequestContext {
    fn new(limits: RuntimeIsolationLimits) -> Self {
        Self {
            request_body: RequestBodyBudget::new(limits),
            admission: None,
        }
    }
}

/// Activation failures that occur after the transport-neutral configuration is parsed.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GatewayProxyError {
    /// The edge configuration itself violates a fail-closed invariant.
    #[error("invalid edge configuration: {0}")]
    InvalidConfiguration(#[from] GatewayConfigError),
    /// A validated upstream could not be materialized safely as Pingora transport authority.
    #[error("unable to activate upstream transport: {0}")]
    UpstreamActivation(#[from] PeerBuildError),
}

/// Pingora HTTP application backed by one explicitly configured upstream.
#[derive(Debug, Clone)]
pub struct GatewayProxy {
    upstream_peer: HttpPeer,
    limits: RuntimeIsolationLimits,
    admission_budget: RequestAdmissionBudget,
}

impl GatewayProxy {
    /// Builds the version-1 delivery adapter from a validated edge configuration.
    ///
    /// Contract validation owns upstream-count and network-authority rules. The adapter constructs
    /// immutable Pingora transport state once during activation rather than repeating validation on
    /// every proxied request. Explicit trust material is also loaded before any listener is opened.
    pub fn try_from_config(config: &GatewayConfig) -> std::result::Result<Self, GatewayProxyError> {
        config.validate()?;
        let upstream = &config.upstreams[0];
        let limits = RuntimeIsolationLimits::from_validated(
            config.max_request_body_bytes,
            config.max_in_flight_requests,
        );

        Ok(Self {
            upstream_peer: build_peer_from_validated(upstream)?,
            limits,
            admission_budget: RequestAdmissionBudget::new(limits),
        })
    }

    /// Returns a fresh clone of the prevalidated Pingora peer for one upstream connection attempt.
    pub fn build_upstream_peer(&self) -> HttpPeer {
        self.upstream_peer.clone()
    }

    fn admit_request(&self, ctx: &mut RequestContext) -> pingora::Result<()> {
        if let Some(admission) = self.admission_budget.acquire() {
            ctx.admission = Some(admission);
            return Ok(());
        }

        record_backpressure_rejection();
        Err(Error::explain(
            ErrorType::HTTPStatus(503),
            "gateway max_in_flight_requests budget exhausted",
        ))
    }

    fn reject_oversize_declared_body(
        session: &Session,
        ctx: &RequestContext,
    ) -> pingora::Result<()> {
        let declared = session
            .req_header()
            .headers
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|raw| raw.parse::<u64>().ok());
        if let Some(length) = declared {
            ctx.request_body
                .reject_declared_length(length)
                .map_err(body_rejection_to_pingora)?;
        }
        Ok(())
    }
}

fn body_rejection_to_pingora(rejection: BodyLimitExceeded) -> Box<Error> {
    let _ = (rejection.observed, rejection.limit);
    Error::explain(
        ErrorType::HTTPStatus(413),
        "request body exceeds configured max_request_body_bytes",
    )
}

#[async_trait]
impl ProxyHttp for GatewayProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new(self.limits)
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        match session.req_header().uri.path() {
            LIVENESS_PATH | READINESS_PATH => {
                respond_healthy(session).await?;
                Ok(true)
            }
            _ => {
                self.admit_request(ctx)?;
                Self::reject_oversize_declared_body(session, ctx)?;
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
        ctx.request_body
            .observe_chunk(chunk_bytes)
            .map_err(body_rejection_to_pingora)
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
        record_request(session, error, ctx.request_body.observed());
    }
}

#[cfg(test)]
mod tests {
    use super::{body_rejection_to_pingora, RequestContext};
    use crate::runtime_isolation::{
        BodyLimitExceeded, RequestAdmissionBudget, RuntimeIsolationLimits,
    };
    use pingora::prelude::{ErrorType, ProxyHttp};

    #[test]
    fn admission_budget_rejects_at_capacity_and_recovers_after_release() {
        let limits = RuntimeIsolationLimits::try_new(1024, 1).expect("fixture limits are valid");
        let budget = RequestAdmissionBudget::new(limits);
        let first = budget.acquire().expect("first request is admitted");
        assert!(budget.acquire().is_none());

        drop(first);

        assert!(budget.acquire().is_some());
    }

    #[test]
    fn request_context_starts_with_shared_runtime_isolation_budget() {
        let limits = RuntimeIsolationLimits::try_new(8, 2).expect("fixture limits are valid");
        let proxy_ctx = RequestContext::new(limits);
        assert_eq!(proxy_ctx.request_body.observed(), 0);
        assert!(proxy_ctx.admission.is_none());
    }

    #[test]
    fn body_rejection_maps_to_payload_too_large() {
        let error = body_rejection_to_pingora(BodyLimitExceeded {
            observed: 2,
            limit: 1,
        });
        assert_eq!(error.etype, ErrorType::HTTPStatus(413));
    }

    #[test]
    fn gateway_proxy_context_constructor_uses_runtime_limits() {
        let config = crate::edge_contract::GatewayConfig::from_yaml(
            r#"
version: 1
listener: 127.0.0.1:18080
metrics_listener: 127.0.0.1:18082
max_request_body_bytes: 8
max_in_flight_requests: 2
upstream_keepalive_pool_size: 1
upstreams:
  - name: test
    address: 127.0.0.1:18081
    tls: false
    timeouts:
      connection_ms: 1
      total_connection_ms: 1
      read_ms: 1
      write_ms: 1
      idle_ms: 1
"#,
        )
        .expect("fixture config is valid");
        let proxy = super::GatewayProxy::try_from_config(&config).expect("proxy activates");
        let ctx = proxy.new_ctx();
        assert_eq!(ctx.request_body.observed(), 0);
    }
}
