//! Pingora callback adapter for one characterized multi-route edge migration.
//!
//! The adapter composes transport-neutral routing, HTTP policy, transport binding, runtime
//! isolation, and shared transport observability. It does not introduce product authorization,
//! service discovery, or business logic.

use async_trait::async_trait;
use bytes::Bytes;
use pingora::prelude::{
    Error, ErrorType, HttpPeer, ProxyHttp, RequestHeader, ResponseHeader, Session,
};
use thiserror::Error;

use crate::migration_delivery::MigrationDeliveryPlan;
use crate::observability::{record_backpressure_rejection, record_request};
use crate::runtime_isolation::{
    BodyLimitExceeded, RequestAdmission, RequestAdmissionBudget, RequestBodyBudget,
    RuntimeIsolationLimits,
};

/// Fail-closed callback errors for a characterized migration runtime.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MigrationGatewayProxyError {
    /// The request path did not match any route in the characterized edge contract.
    #[error("request path does not match a characterized edge route: {request_path}")]
    UnmatchedRoute {
        /// Exact request path that could not resolve to an admitted upstream identity.
        request_path: String,
    },
}

/// Per-request state for the characterized multi-route Pingora adapter.
#[derive(Debug)]
pub struct MigrationRequestContext {
    request_body: RequestBodyBudget,
    admission: Option<RequestAdmission>,
}

impl MigrationRequestContext {
    fn new(limits: RuntimeIsolationLimits) -> Self {
        Self {
            request_body: RequestBodyBudget::new(limits),
            admission: None,
        }
    }
}

/// Pingora HTTP application backed only by a prevalidated migration delivery plan.
#[derive(Debug, Clone)]
pub struct MigrationGatewayProxy {
    delivery: MigrationDeliveryPlan,
    limits: RuntimeIsolationLimits,
    admission_budget: RequestAdmissionBudget,
}

impl MigrationGatewayProxy {
    /// Activates callbacks over an already validated delivery plan and runtime-isolation contract.
    pub fn try_new(
        delivery: MigrationDeliveryPlan,
        limits: RuntimeIsolationLimits,
    ) -> Result<Self, MigrationGatewayProxyError> {
        Ok(Self {
            delivery,
            limits,
            admission_budget: RequestAdmissionBudget::new(limits),
        })
    }

    /// Selects and clones the concrete peer admitted for one characterized request path.
    pub fn build_upstream_peer(
        &self,
        request_path: &str,
    ) -> Result<HttpPeer, MigrationGatewayProxyError> {
        self.delivery
            .build_upstream_peer(request_path)
            .ok_or_else(|| MigrationGatewayProxyError::UnmatchedRoute {
                request_path: request_path.to_string(),
            })
    }

    /// Replaces untrusted forwarding identity with the gateway-owned transport assertion.
    pub fn apply_upstream_request_policy(
        &self,
        upstream_request: &mut RequestHeader,
    ) -> pingora::Result<()> {
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

    /// Applies every characterized edge-owned response header using replacement semantics.
    pub fn apply_response_headers(&self, response: &mut ResponseHeader) -> pingora::Result<()> {
        for rule in self.delivery.response_header_rules() {
            response.insert_header(rule.name.clone(), rule.value.as_str())?;
        }
        Ok(())
    }

    fn admit_request(&self, ctx: &mut MigrationRequestContext) -> pingora::Result<()> {
        if let Some(admission) = self.admission_budget.acquire() {
            ctx.admission = Some(admission);
            return Ok(());
        }

        record_backpressure_rejection();
        Err(Error::explain(
            ErrorType::HTTPStatus(503),
            "migration gateway max_in_flight_requests budget exhausted",
        ))
    }

    fn reject_oversize_declared_body(
        session: &Session,
        ctx: &MigrationRequestContext,
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

fn unmatched_route_to_pingora(_error: MigrationGatewayProxyError) -> Box<Error> {
    Error::explain(
        ErrorType::HTTPStatus(404),
        "request path does not match a characterized edge route",
    )
}

#[async_trait]
impl ProxyHttp for MigrationGatewayProxy {
    type CTX = MigrationRequestContext;

    fn new_ctx(&self) -> Self::CTX {
        MigrationRequestContext::new(self.limits)
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        self.admit_request(ctx)?;
        Self::reject_oversize_declared_body(session, ctx)?;
        Ok(false)
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
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        self.build_upstream_peer(session.req_header().uri.path())
            .map(Box::new)
            .map_err(unmatched_route_to_pingora)
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
        self.apply_upstream_request_policy(upstream_request)
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        self.apply_response_headers(upstream_response)
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
    use pingora::prelude::ErrorType;

    use super::{
        body_rejection_to_pingora, unmatched_route_to_pingora, MigrationGatewayProxyError,
        MigrationRequestContext,
    };
    use crate::runtime_isolation::{BodyLimitExceeded, RuntimeIsolationLimits};

    #[test]
    fn migration_context_starts_without_an_admission_lease() {
        let limits = RuntimeIsolationLimits::try_new(8, 1).expect("fixture limits are valid");
        let ctx = MigrationRequestContext::new(limits);
        assert_eq!(ctx.request_body.observed(), 0);
        assert!(ctx.admission.is_none());
    }

    #[test]
    fn delivery_errors_map_to_fail_closed_http_errors() {
        let body_error = body_rejection_to_pingora(BodyLimitExceeded {
            observed: 2,
            limit: 1,
        });
        assert_eq!(body_error.etype, ErrorType::HTTPStatus(413));

        let route_error = unmatched_route_to_pingora(MigrationGatewayProxyError::UnmatchedRoute {
            request_path: "/missing".to_string(),
        });
        assert_eq!(route_error.etype, ErrorType::HTTPStatus(404));
    }
}
