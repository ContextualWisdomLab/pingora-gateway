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

/// Registers one process-global counter and fails closed if its stable metric name is duplicated.
fn register_counter(name: &'static str, help: &'static str) -> IntCounter {
    register_int_counter!(name, help)
        .unwrap_or_else(|error| panic!("gateway metric {name} must register exactly once: {error}"))
}

/// Counts completed downstream requests across success and error outcomes.
static REQUESTS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_requests_total",
        "Completed downstream requests observed by the shared edge runtime",
    )
});

/// Counts downstream requests whose Pingora lifecycle completes with an error.
static REQUEST_ERRORS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_request_errors_total",
        "Completed downstream requests whose Pingora lifecycle ended with an error",
    )
});

/// Accumulates downstream request-body bytes observed before completion or rejection.
static REQUEST_BODY_BYTES_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_request_body_bytes_total",
        "Downstream request body bytes observed before completion or rejection",
    )
});

/// Counts requests rejected because the configured process admission budget is exhausted.
static BACKPRESSURE_REJECTIONS_TOTAL: LazyLock<IntCounter> = LazyLock::new(|| {
    register_counter(
        "cwl_pingora_gateway_backpressure_rejections_total",
        "Downstream requests rejected because max_in_flight_requests was exhausted",
    )
});

/// Process-local admission controller for non-health downstream request lifecycles.
#[derive(Debug, Clone)]
struct RequestAdmissionBudget {
    /// Shared count of currently admitted non-health requests.
    in_flight: Arc<AtomicUsize>,
    /// Maximum simultaneous admissions allowed by the validated edge contract.
    limit: usize,
}

impl RequestAdmissionBudget {
    /// Creates an empty admission budget with the already-validated positive process limit.
    fn new(limit: usize) -> Self {
        Self {
            in_flight: Arc::new(AtomicUsize::new(0)),
            limit,
        }
    }

    /// Atomically acquires one request lease or returns HTTP 503 when capacity is exhausted.
    fn acquire_or_reject(&self) -> pingora::Result<RequestAdmission> {
        let admitted =
            self.in_flight
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                    (current < self.limit).then_some(current + 1)
                });

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
    /// Shared counter decremented exactly once when this request lifecycle releases its lease.
    in_flight: Arc<AtomicUsize>,
}

impl Drop for RequestAdmission {
    /// Releases one previously acquired admission slot at request-context teardown.
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Per-request delivery state. Product domain state does not belong here.
#[derive(Debug, Default)]
pub struct RequestContext {
    /// Saturating count of downstream request-body bytes observed by streaming filters.
    request_body_bytes: u64,
    /// Admission lease retained until the complete non-health request lifecycle ends.
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
    /// Prevalidated immutable upstream transport authority cloned per connection attempt.
    upstream_peer: HttpPeer,
    /// Maximum cumulative downstream request-body bytes admitted for one request lifecycle.
    max_request_body_bytes: u64,
    /// Process-local concurrent request admission controller.
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

    /// Serves an empty non-cacheable HTTP 200 response for process health endpoints.
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

    /// Rejects an already-framed body whose declared length exceeds the configured byte budget.
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

    /// Starts each request with no body bytes observed and no admission lease acquired.
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::default()
    }

    /// Handles health requests locally and acquires admission before proxying all other requests.
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

    /// Accumulates streamed body bytes and rejects the request as soon as the limit is exceeded.
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

    /// Supplies the single prevalidated transport peer admitted by the version-1 contract.
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        Ok(Box::new(self.build_upstream_peer()))
    }

    /// Removes client-controlled forwarding identity before adding the gateway-owned protocol fact.
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

    /// Emits bounded low-cardinality completion metrics and one coarse access-log record.
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
        let first = budget
            .acquire_or_reject()
            .expect("first request is admitted");
        assert!(budget.acquire_or_reject().is_err());

        drop(first);

        assert!(budget.acquire_or_reject().is_ok());
    }
}
