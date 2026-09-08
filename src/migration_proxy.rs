//! Pingora callback adapter for one characterized multi-route edge migration.
//!
//! The adapter composes transport-neutral routing, HTTP policy, transport binding, runtime
//! isolation, trusted forwarding metadata, and shared transport observability. It does not
//! introduce product authorization, service discovery, or business logic.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use pingora::prelude::{
    Error, ErrorType, HttpPeer, ProxyHttp, RequestHeader, ResponseHeader, Session,
};
use thiserror::Error;

use crate::forwarding_policy::{DownstreamScheme, ForwardingContext};
use crate::migration_delivery::MigrationDeliveryPlan;
use crate::observability::{record_backpressure_rejection, record_request};
use crate::pingora_delivery::reject_uncharacterized_http1_protocol_transition;
use crate::process_health::{respond_healthy, LIVENESS_PATH, READINESS_PATH};
use crate::runtime_isolation::{
    BodyLimitExceeded, RequestAdmission, RequestAdmissionBudget, RequestBodyBudget,
    ResponseBodyLifetimeBudget, ResponseBodyLifetimeExceeded, RuntimeIsolationLimits,
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

impl MigrationGatewayProxyError {
    /// Maps an adapter-local route miss to the bounded HTTP response exposed at the edge.
    pub fn into_pingora(self) -> Box<Error> {
        let Self::UnmatchedRoute { .. } = self;
        Error::explain(
            ErrorType::HTTPStatus(404),
            "request path does not match a characterized edge route",
        )
    }
}

/// Per-request state for the characterized multi-route Pingora adapter.
#[derive(Debug)]
pub struct MigrationRequestContext {
    request_body: RequestBodyBudget,
    response_body_lifetime: ResponseBodyLifetimeBudget,
    admission: Option<RequestAdmission>,
}

impl MigrationRequestContext {
    /// Creates isolated per-request accounting with no in-flight admission lease yet acquired.
    fn new(limits: RuntimeIsolationLimits) -> Self {
        Self {
            request_body: RequestBodyBudget::new(limits),
            response_body_lifetime: ResponseBodyLifetimeBudget::new(limits),
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
    /// Creates callbacks over an already validated delivery plan and runtime-isolation contract.
    ///
    /// Construction has no remaining fallible work: peer materialization already happened in
    /// `MigrationDeliveryPlan`, and `RuntimeIsolationLimits` can exist only after validation.
    pub fn new(delivery: MigrationDeliveryPlan, limits: RuntimeIsolationLimits) -> Self {
        Self {
            delivery,
            limits,
            admission_budget: RequestAdmissionBudget::new(limits),
        }
    }

    /// Backward-compatible constructor for callers that still consume the earlier result shape.
    ///
    /// No runtime error can be produced at this boundary; later fail-closed errors are request
    /// routing, body, forwarding, transport, or upstream failures handled by Pingora callbacks.
    pub fn try_new(
        delivery: MigrationDeliveryPlan,
        limits: RuntimeIsolationLimits,
    ) -> Result<Self, MigrationGatewayProxyError> {
        Ok(Self::new(delivery, limits))
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

    /// Replaces request-controlled forwarding identity with transport-derived metadata.
    pub fn apply_upstream_request_policy(
        &self,
        upstream_request: &mut RequestHeader,
        forwarding: &ForwardingContext,
    ) -> pingora::Result<()> {
        forwarding.apply(upstream_request)
    }

    /// Applies every characterized edge-owned response header using replacement semantics.
    pub fn apply_response_headers(&self, response: &mut ResponseHeader) -> pingora::Result<()> {
        for rule in self.delivery.response_header_rules() {
            response
                .insert_header(rule.name.clone(), rule.value.as_str())
                .expect("validated response-header policy must remain representable at delivery");
        }
        Ok(())
    }

    /// Acquires one process-local in-flight lease or rejects before upstream work begins.
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

    /// Rejects an already-declared request body that exceeds the configured byte budget.
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

/// Maps the transport-neutral body-limit violation to the stable fail-closed HTTP status.
fn body_rejection_to_pingora(rejection: BodyLimitExceeded) -> Box<Error> {
    let _ = (rejection.observed, rejection.limit);
    Error::explain(
        ErrorType::HTTPStatus(413),
        "request body exceeds configured max_request_body_bytes",
    )
}

/// Maps a versioned response-body lifetime breach to an upstream-scoped Pingora failure.
fn response_body_lifetime_to_pingora(rejection: ResponseBodyLifetimeExceeded) -> Box<Error> {
    let _ = (rejection.elapsed, rejection.limit);
    Error::new_up(ErrorType::Custom("UpstreamResponseBodyLifetimeExceeded"))
}

/// Starts the lifetime on the first final upstream response header without reset on later headers.
fn start_response_body_lifetime(
    is_informational: bool,
    ctx: &mut MigrationRequestContext,
    now: Instant,
) {
    if !is_informational {
        ctx.response_body_lifetime.start(now);
    }
}

/// Applies the lifetime only to actual body progress, not empty/end-of-stream bookkeeping callbacks.
fn enforce_response_body_lifetime(
    body: &Option<Bytes>,
    ctx: &MigrationRequestContext,
    now: Instant,
) -> Result<(), ResponseBodyLifetimeExceeded> {
    if body.as_ref().is_some_and(|chunk| !chunk.is_empty()) {
        ctx.response_body_lifetime.reject_if_expired(now)?;
    }
    Ok(())
}

#[async_trait]
impl ProxyHttp for MigrationGatewayProxy {
    type CTX = MigrationRequestContext;

    /// Creates request-local body and admission accounting for a new downstream exchange.
    fn new_ctx(&self) -> Self::CTX {
        MigrationRequestContext::new(self.limits)
    }

    /// Serves process health locally and admits ordinary traffic before any upstream selection.
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
                reject_uncharacterized_http1_protocol_transition(session.req_header())?;
                self.admit_request(ctx)?;
                Self::reject_oversize_declared_body(session, ctx)?;
                Ok(false)
            }
        }
    }

    /// Accounts streamed request bytes so chunked bodies cannot bypass the declared-length gate.
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
        let chunk_bytes = body.as_ref().map_or(0_usize, Bytes::len) as u64;
        ctx.request_body
            .observe_chunk(chunk_bytes)
            .map_err(body_rejection_to_pingora)
    }

    /// Resolves only a prevalidated peer admitted by the immutable characterized route plan.
    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        self.build_upstream_peer(session.req_header().uri.path())
            .map(Box::new)
            .map_err(MigrationGatewayProxyError::into_pingora)
    }

    /// Rebuilds forwarding identity from the accepted socket and Host authority before origin I/O.
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        let forwarding = ForwardingContext::from_downstream_transport(
            session.client_addr(),
            session.server_addr(),
            upstream_request,
            session.req_header(),
            DownstreamScheme::Http,
        )?;
        self.apply_upstream_request_policy(upstream_request, &forwarding)
    }

    /// Starts the versioned response-body lifetime at the first non-informational upstream header.
    async fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        start_response_body_lifetime(
            upstream_response.status.is_informational(),
            ctx,
            Instant::now(),
        );
        Ok(())
    }

    /// Applies the characterized edge-owned response fields with replacement semantics.
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

    /// Enforces the versioned lifetime at real upstream body-progress boundaries only.
    fn upstream_response_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Option<Duration>> {
        enforce_response_body_lifetime(body, ctx, Instant::now())
            .map_err(response_body_lifetime_to_pingora)?;
        Ok(None)
    }

    /// Emits only the shared low-cardinality completion observation for the finished request.
    async fn logging(&self, session: &mut Session, error: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        record_request(session, error, ctx.request_body.observed());
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use bytes::Bytes;
    use pingora::prelude::{ErrorSource, ErrorType};

    use super::{
        body_rejection_to_pingora, enforce_response_body_lifetime,
        response_body_lifetime_to_pingora, start_response_body_lifetime,
        MigrationGatewayProxyError, MigrationRequestContext,
    };
    use crate::runtime_isolation::{
        BodyLimitExceeded, ResponseBodyLifetimeExceeded, RuntimeIsolationLimits,
    };

    #[test]
    fn migration_context_starts_without_an_admission_lease() {
        let limits = RuntimeIsolationLimits::try_new(8, 1).expect("fixture limits are valid");
        let ctx = MigrationRequestContext::new(limits);
        assert_eq!(ctx.request_body.observed(), 0);
        assert!(ctx.admission.is_none());
    }

    #[test]
    fn informational_headers_do_not_start_or_reset_the_response_body_lifetime() {
        let limits = RuntimeIsolationLimits::try_new_with_response_body_limit(8, 1, 300)
            .expect("fixture limits are valid");
        let mut ctx = MigrationRequestContext::new(limits);
        let now = Instant::now();

        start_response_body_lifetime(true, &mut ctx, now);
        assert!(ctx
            .response_body_lifetime
            .reject_if_expired(now + Duration::from_secs(1))
            .is_ok());

        start_response_body_lifetime(false, &mut ctx, now);
        start_response_body_lifetime(false, &mut ctx, now + Duration::from_millis(250));
        assert!(ctx
            .response_body_lifetime
            .reject_if_expired(now + Duration::from_millis(299))
            .is_ok());
        assert!(ctx
            .response_body_lifetime
            .reject_if_expired(now + Duration::from_millis(300))
            .is_err());
    }

    #[test]
    fn response_lifetime_ignores_empty_callbacks_but_rejects_expired_body_progress() {
        let limits = RuntimeIsolationLimits::try_new_with_response_body_limit(8, 1, 300)
            .expect("fixture limits are valid");
        let mut ctx = MigrationRequestContext::new(limits);
        let started = Instant::now();
        start_response_body_lifetime(false, &mut ctx, started);
        let before_expiry = started + Duration::from_millis(299);
        let expired = started + Duration::from_millis(300);

        assert!(enforce_response_body_lifetime(&None, &ctx, expired).is_ok());
        assert!(enforce_response_body_lifetime(&Some(Bytes::new()), &ctx, expired).is_ok());
        assert!(enforce_response_body_lifetime(
            &Some(Bytes::from_static(b"x")),
            &ctx,
            before_expiry
        )
        .is_ok());
        assert!(
            enforce_response_body_lifetime(&Some(Bytes::from_static(b"x")), &ctx, expired).is_err()
        );
    }

    #[test]
    fn delivery_errors_map_to_fail_closed_http_errors() {
        let body_error = body_rejection_to_pingora(BodyLimitExceeded {
            observed: 2,
            limit: 1,
        });
        assert_eq!(body_error.etype, ErrorType::HTTPStatus(413));

        let route_error = MigrationGatewayProxyError::UnmatchedRoute {
            request_path: "/missing".to_string(),
        }
        .into_pingora();
        assert_eq!(route_error.etype, ErrorType::HTTPStatus(404));

        let lifetime_error = response_body_lifetime_to_pingora(ResponseBodyLifetimeExceeded {
            elapsed: Duration::from_millis(301),
            limit: Duration::from_millis(300),
        });
        assert_eq!(
            lifetime_error.etype,
            ErrorType::Custom("UpstreamResponseBodyLifetimeExceeded")
        );
        assert_eq!(lifetime_error.esource, ErrorSource::Upstream);
    }
}
