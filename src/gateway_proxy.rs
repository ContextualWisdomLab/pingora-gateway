//! Initial executable proxy application for the shared gateway.
//!
//! Version 1 activates one upstream per process because the transport-neutral edge contract owns
//! that invariant. This Pingora adapter does not invent routing or load-balancing domain rules.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use pingora::prelude::{
    Error, ErrorType, HttpPeer, ProxyHttp, RequestHeader, ResponseHeader, Session,
};
use pingora_prometheus::prometheus::{register_int_counter, IntCounter};
use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError};
use crate::pingora_delivery::{build_peer_from_validated, PeerBuildError};

/// Stable process-local liveness endpoint.
pub const LIVENESS_PATH: &str = "/livez";
/// Stable readiness endpoint reached through the production Pingora serving path.
pub const READINESS_PATH: &str = "/readyz";

fn register_counter(name: &'static str, help: &'static str) -> IntCounter {
    register_int_counter!(name, help)
        .unwrap_or_else(|error| panic!("gateway metric {name} must register exactly once: {error}"))
}

static REQUESTS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_requests_total",
        "Completed downstream requests observed by the shared edge runtime",
    )
});

static REQUEST_ERRORS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_request_errors_total",
        "Completed downstream requests whose Pingora lifecycle ended with an error",
    )
});

static REQUEST_BODY_BYTES_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_request_body_bytes_total",
        "Downstream request body bytes observed before completion or rejection",
    )
});

static BACKPRESSURE_REJECTIONS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_backpressure_rejections_total",
        "Downstream requests rejected because max_in_flight_requests was exhausted",
    )
});

#[derive(Debug, Clone)]
struct RequestAdmissionBudget {
    in_flight: Arc<AtomicUsize>,
    limit: usize,
}

impl RequestAdmissionBudget {
    fn new(limit: usize) -> Self {
        Self {
            in_flight: Arc::new(AtomicUsize::new(0)),
            limit,
        }
    }

    fn acquire_or_reject(&self) -> pingora::Result<RequestAdmission> {
        let admitted = self.in_flight.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |current| (current < self.limit).then_some(current + 1),
        );

        match admitted {
            Ok(_) => Ok(RequestAdmission {
                in_flight: Arc::clone(&self.in_flight),
            }),
            Err(_) => {
                BACKPRESSURE_REJECTIONS_TOTAL.inc();
                Err(Error::explain(
                    ErrorType::HTTPStatus(503),
                    "gateway max_in_flight_requests budget exhausted",
                ))
            }
        }
    }
}

/// RAII admission lease held for the complete non-health request lifecycle.
#[derive(Debug)]
pub struct RequestAdmission {
    in_flight: Arc<AtomicUsize>,
}

impl Drop for RequestAdmission {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Per-request delivery state. Product domain state does not belong here.
#[derive(Debug, Default)]
pub struct RequestContext {
    request_body_bytes: u64,
    admission: Option<RequestAdmission>,
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
    max_request_body_bytes: u64,
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

        Ok(Self {
            upstream_peer: build_peer_from_validated(upstream)?,
            max_request_body_bytes: config.max_request_body_bytes,
            admission_budget: RequestAdmissionBudget::new(config.max_in_flight_requests),
        })
    }

    /// Returns a fresh clone of the prevalidated Pingora peer for one upstream connection attempt.
    pub fn build_upstream_peer(&self) -> HttpPeer {
        self.upstream_peer.clone()
    }

    async fn respond_healthy(session: &mut Session) -> pingora::Result<()> {
        // These literals are compile-time gateway invariants, not runtime inputs. Treat failure to
        // construct them as a programmer defect while preserving the real downstream write result.
        let mut response = ResponseHeader::build(200, None)
            .expect("literal HTTP 200 response header must be valid");
        response
            .insert_header("Content-Length", "0")
            .expect("literal Content-Length response header must be valid");
        response
            .insert_header("Cache-Control", "no-store")
            .expect("literal Cache-Control response header must be valid");
        session
            .write_response_header(Box::new(response), true)
            .await
    }

    fn reject_oversize_declared_body(&self, session: &Session) -> pingora::Result<()> {
        // Pingora's HTTP admission reconciles Content-Length framing and rejects invalid values
        // before ProxyHttp filters run. Keep this layer focused on the gateway's size policy while
        // the streamed-body filter remains the fail-closed backstop for absent framing.
        let declared = session
            .req_header()
            .headers
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|raw| raw.parse::<u64>().ok());
        if declared.is_some_and(|length| length > self.max_request_body_bytes) {
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
        ctx: &mut Self::CTX,
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
                ctx.admission = Some(self.admission_budget.acquire_or_reject()?);
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

#[cfg(test)]
mod tests {
    use std::panic;

    use super::{register_counter, RequestAdmissionBudget};

    #[test]
    fn duplicate_metric_registration_fails_closed() {
        let name = "cwl_pingora_gateway_test_duplicate_registration_total";
        let help = "Coverage-only counter proving duplicate registration fails closed";
        let _first = register_counter(name, help);

        let duplicate = panic::catch_unwind(|| register_counter(name, help));

        assert!(duplicate.is_err());
    }

    #[test]
    fn admission_budget_rejects_at_capacity_and_recovers_after_release() {
        let budget = RequestAdmissionBudget::new(1);
        let first = budget.acquire_or_reject().expect("first request is admitted");
        assert!(budget.acquire_or_reject().is_err());

        drop(first);

        assert!(budget.acquire_or_reject().is_ok());
    }
}
