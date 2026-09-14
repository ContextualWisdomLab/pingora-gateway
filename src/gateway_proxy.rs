//! Initial executable proxy application for the shared gateway.
//!
//! Version 1 activates one upstream per process because the transport-neutral edge contract owns
//! that invariant. This Pingora adapter does not invent routing or load-balancing domain rules.

use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::Version;
use pingora::prelude::{
    Error, ErrorType, HttpPeer, ProxyHttp, RequestHeader, ResponseHeader, Session,
};
use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError};
use crate::observability::{record_backpressure_rejection, record_request};
use crate::pingora_delivery::{build_peer_from_validated, PeerBuildError};
use crate::runtime_isolation::{
    BodyLimitExceeded, RequestAdmission, RequestAdmissionBudget, RequestBodyBudget,
    RuntimeIsolationLimits,
};

/// Stable process-local liveness endpoint.
pub const LIVENESS_PATH: &str = "/livez";
/// Stable readiness endpoint reached through the production Pingora serving path.
pub const READINESS_PATH: &str = "/readyz";

const MAX_SUPPORTED_MAX_FORWARDS: u32 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaxForwardsAction {
    Ignore,
    FinalRecipient,
    Forward(u32),
}

/// Per-request delivery state. Product domain state does not belong here.
#[derive(Debug)]
pub struct RequestContext {
    request_body: RequestBodyBudget,
    admission: Option<RequestAdmission>,
}

impl RequestContext {
    /// Creates isolation state without consuming an in-flight lease.
    ///
    /// Admission is deferred until a non-health application request enters the proxy path so
    /// process-local health checks remain observable even when the application budget is full.
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

    /// Answers process-local health probes without contacting the configured upstream.
    ///
    /// Health responses intentionally bypass application admission so operators can distinguish a
    /// live but saturated gateway from an unavailable process.
    async fn respond_healthy(session: &mut Session) -> pingora::Result<()> {
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

    /// Terminates an exhausted TRACE/OPTIONS forwarding budget at this gateway.
    ///
    /// Generic v1 does not implement local TRACE echo or resource-specific OPTIONS semantics. A
    /// zero Max-Forwards value therefore receives a local 501 rather than crossing another hop;
    /// this also avoids reflecting credential-bearing request fields from TRACE.
    async fn respond_max_forwards_final_recipient(session: &mut Session) -> pingora::Result<()> {
        let mut response = ResponseHeader::build(501, None)
            .expect("literal HTTP 501 response header must be valid");
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

    /// Acquires the shared in-flight lease before application request processing begins.
    ///
    /// Saturation fails locally with 503 and records bounded telemetry; no upstream selection or
    /// connection attempt occurs without a lease.
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

    /// Applies application admission before classifying TRACE/OPTIONS intermediary control.
    ///
    /// A zero or malformed `Max-Forwards` request can terminate locally, but it is still
    /// application traffic. Keeping classification behind the shared lease prevents local 4xx/5xx
    /// paths from bypassing the process-wide in-flight budget.
    fn admit_and_classify_max_forwards(
        &self,
        request: &RequestHeader,
        ctx: &mut RequestContext,
    ) -> pingora::Result<MaxForwardsAction> {
        self.admit_request(ctx)?;
        max_forwards_action(request)
    }

    /// Rejects an oversized declared body before streaming additional request bytes upstream.
    ///
    /// Chunked or otherwise undeclared bodies remain bounded independently by `RequestBodyBudget`
    /// as body progress arrives.
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

/// Maps request-body budget violations to the stable downstream 413 contract.
///
/// Observed and configured byte counts stay out of the client-visible error text so this adapter
/// does not expand the gateway's externally observable resource-policy surface.
fn body_rejection_to_pingora(rejection: BodyLimitExceeded) -> Box<Error> {
    let _ = (rejection.observed, rejection.limit);
    Error::explain(
        ErrorType::HTTPStatus(413),
        "request body exceeds configured max_request_body_bytes",
    )
}

/// Builds the stable client-visible error for an invalid TRACE/OPTIONS hop budget.
fn invalid_max_forwards() -> Box<Error> {
    Error::explain(
        ErrorType::HTTPStatus(400),
        "TRACE/OPTIONS Max-Forwards must contain exactly one decimal value",
    )
}

/// Classifies RFC 9110 `Max-Forwards` without mutating the received request.
///
/// Arbitrarily long decimal values are parsed with saturating arithmetic because only the bounded
/// forwarded value matters; malformed or duplicate fields fail closed before upstream selection.
fn max_forwards_action(request: &RequestHeader) -> pingora::Result<MaxForwardsAction> {
    if !matches!(request.method.as_str(), "TRACE" | "OPTIONS") {
        return Ok(MaxForwardsAction::Ignore);
    }

    let mut values = request.headers.get_all("max-forwards").iter();
    let Some(value) = values.next() else {
        return Ok(MaxForwardsAction::Ignore);
    };
    if values.next().is_some() {
        return Err(invalid_max_forwards());
    }

    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return Err(invalid_max_forwards());
    }
    if bytes.iter().all(|byte| *byte == b'0') {
        return Ok(MaxForwardsAction::FinalRecipient);
    }

    let received = bytes.iter().fold(0_u32, |current, digit| {
        current
            .saturating_mul(10)
            .saturating_add(u32::from(*digit - b'0'))
    });
    Ok(MaxForwardsAction::Forward(
        received.saturating_sub(1).min(MAX_SUPPORTED_MAX_FORWARDS),
    ))
}

/// Rewrites a forwarding-eligible TRACE/OPTIONS hop budget immediately before proxy delivery.
///
/// A final-recipient result is an invariant violation here because `request_filter` should already
/// have terminated it locally; returning 501 preserves fail-closed behavior if call ordering drifts.
fn apply_max_forwards_before_forward(request: &mut RequestHeader) -> pingora::Result<()> {
    match max_forwards_action(request)? {
        MaxForwardsAction::Ignore => Ok(()),
        MaxForwardsAction::Forward(value) => {
            request.insert_header("Max-Forwards", value.to_string())?;
            Ok(())
        }
        MaxForwardsAction::FinalRecipient => Err(Error::explain(
            ErrorType::HTTPStatus(501),
            "TRACE/OPTIONS Max-Forwards budget exhausted at gateway",
        )),
    }
}

/// Maps an actually received HTTP protocol version to the RFC 9110 `Via` received-protocol token.
fn gateway_via_value(version: Version) -> pingora::Result<&'static str> {
    match version {
        Version::HTTP_10 => Ok("1.0 cwl-pingora-gateway"),
        Version::HTTP_11 => Ok("1.1 cwl-pingora-gateway"),
        Version::HTTP_2 => Ok("2 cwl-pingora-gateway"),
        Version::HTTP_3 => Ok("3 cwl-pingora-gateway"),
        _ => Err(Error::explain(
            ErrorType::InvalidHTTPHeader,
            "unsupported HTTP version for RFC 9110 Via",
        )),
    }
}

/// Removes request-controlled forwarding identity and emits gateway-owned forwarding metadata.
///
/// Generic v1 intentionally makes no client-IP, client-certificate, or trusted-proxy provenance
/// claim. The entire `X-Forwarded-*` namespace is untrusted until a separately characterized and
/// versioned trust-source contract admits specific fields. RFC 9110 `Via` is different: it is a
/// protocol trace, so the received chain is preserved and this gateway appends a pseudonymous hop;
/// no `Via` member is accepted as authentication or authorization evidence.
fn sanitize_forwarding_headers(
    upstream_request: &mut RequestHeader,
    downstream_version: Version,
) -> pingora::Result<()> {
    let x_forwarded_headers = upstream_request
        .headers
        .keys()
        .filter(|name| {
            name.as_str()
                .as_bytes()
                .get(..b"x-forwarded-".len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"x-forwarded-"))
        })
        .cloned()
        .collect::<Vec<_>>();

    upstream_request.remove_header("Forwarded");
    upstream_request.remove_header("X-Real-IP");
    for header in &x_forwarded_headers {
        upstream_request.remove_header(header);
    }
    upstream_request.insert_header("Forwarded", "proto=http")?;
    upstream_request.append_header("Via", gateway_via_value(downstream_version)?)?;
    Ok(())
}

/// Appends this intermediary to a forwarded upstream response without rewriting the received chain.
fn append_response_via(upstream_response: &mut ResponseHeader) -> pingora::Result<()> {
    upstream_response.append_header("Via", gateway_via_value(upstream_response.version)?)?;
    Ok(())
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
                Self::respond_healthy(session).await?;
                Ok(true)
            }
            _ => {
                let max_forwards =
                    self.admit_and_classify_max_forwards(session.req_header(), ctx)?;
                Self::reject_oversize_declared_body(session, ctx)?;
                if max_forwards == MaxForwardsAction::FinalRecipient {
                    Self::respond_max_forwards_final_recipient(session).await?;
                    return Ok(true);
                }
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
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        apply_max_forwards_before_forward(upstream_request)?;
        sanitize_forwarding_headers(upstream_request, session.req_header().version)
    }

    async fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        append_response_via(upstream_response)
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
    use super::{
        append_response_via, apply_max_forwards_before_forward, body_rejection_to_pingora,
        max_forwards_action, sanitize_forwarding_headers, MaxForwardsAction, RequestContext,
        MAX_SUPPORTED_MAX_FORWARDS,
    };
    use crate::runtime_isolation::{
        BodyLimitExceeded, RequestAdmissionBudget, RuntimeIsolationLimits,
    };
    use pingora::http::Version;
    use pingora::prelude::{ErrorType, ProxyHttp, RequestHeader, ResponseHeader};

    #[test]
    fn generic_forwarding_sanitization_removes_all_client_controlled_proxy_identity() {
        let mut request =
            RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
        for (name, value) in [
            ("Forwarded", "for=attacker"),
            ("X-Forwarded-For", "203.0.113.77"),
            ("X-Forwarded-Host", "attacker.example"),
            ("X-Forwarded-Port", "4444"),
            ("X-Forwarded-Proto", "https"),
            ("X-Forwarded-Server", "attacker-proxy"),
            ("X-Forwarded-Prefix", "/attacker-base"),
            ("X-Forwarded-PathBase", "/attacker-path-base"),
            (
                "X-Forwarded-Client-Cert",
                "By=spiffe://attacker;Hash=deadbeef;URI=spiffe://attacker/client",
            ),
            ("X-Real-IP", "203.0.113.77"),
            ("Via", "1.0 previous-hop"),
            ("X-Application-Context", "must-survive"),
        ] {
            request
                .insert_header(name, value)
                .expect("fixture forwarding header must be valid");
        }

        sanitize_forwarding_headers(&mut request, Version::HTTP_11)
            .expect("gateway-owned forwarding metadata must remain valid");

        assert_eq!(request.headers["forwarded"].to_str().unwrap(), "proto=http");
        let via_values = request
            .headers
            .get_all("via")
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            via_values,
            vec!["1.0 previous-hop", "1.1 cwl-pingora-gateway"],
            "RFC 9110 Via chain must preserve the received trace and append this gateway hop"
        );
        for name in [
            "x-forwarded-for",
            "x-forwarded-host",
            "x-forwarded-port",
            "x-forwarded-proto",
            "x-forwarded-server",
            "x-forwarded-prefix",
            "x-forwarded-pathbase",
            "x-forwarded-client-cert",
            "x-real-ip",
        ] {
            assert!(
                !request.headers.contains_key(name),
                "{name} must not retain client-controlled identity"
            );
        }
        assert_eq!(
            request.headers["x-application-context"].to_str().unwrap(),
            "must-survive",
            "non-forwarding application metadata must remain untouched"
        );
    }

    #[test]
    fn trace_max_forwards_is_decremented_before_forwarding() {
        let mut request =
            RequestHeader::build("TRACE", b"/", None).expect("fixture request must be valid");
        request
            .insert_header("Max-Forwards", "2")
            .expect("fixture Max-Forwards must be valid");

        assert_eq!(
            max_forwards_action(&request).expect("valid Max-Forwards is admitted"),
            MaxForwardsAction::Forward(1)
        );
        apply_max_forwards_before_forward(&mut request)
            .expect("positive Max-Forwards can be forwarded");
        assert_eq!(request.headers["max-forwards"].to_str().unwrap(), "1");
    }

    #[test]
    fn options_max_forwards_zero_terminates_at_gateway() {
        let mut request =
            RequestHeader::build("OPTIONS", b"/", None).expect("fixture request must be valid");
        request
            .insert_header("Max-Forwards", "000")
            .expect("fixture Max-Forwards must be valid");

        assert_eq!(
            max_forwards_action(&request).expect("zero Max-Forwards is structurally valid"),
            MaxForwardsAction::FinalRecipient
        );
    }

    #[test]
    fn huge_valid_max_forwards_is_capped_to_gateway_supported_value() {
        let mut request =
            RequestHeader::build("TRACE", b"/", None).expect("fixture request must be valid");
        request
            .insert_header("Max-Forwards", "999999999999999999999999999999999999")
            .expect("fixture Max-Forwards must be valid");

        assert_eq!(
            max_forwards_action(&request).expect("decimal Max-Forwards is valid"),
            MaxForwardsAction::Forward(MAX_SUPPORTED_MAX_FORWARDS)
        );
    }

    #[test]
    fn malformed_or_duplicate_trace_max_forwards_fails_closed() {
        let mut malformed =
            RequestHeader::build("TRACE", b"/", None).expect("fixture request must be valid");
        malformed
            .insert_header("Max-Forwards", "1x")
            .expect("fixture header bytes must be valid");
        let malformed_error = max_forwards_action(&malformed).unwrap_err();
        assert_eq!(malformed_error.etype, ErrorType::HTTPStatus(400));

        let mut duplicate =
            RequestHeader::build("OPTIONS", b"/", None).expect("fixture request must be valid");
        duplicate
            .append_header("Max-Forwards", "2")
            .expect("first fixture value must be valid");
        duplicate
            .append_header("Max-Forwards", "1")
            .expect("second fixture value must be valid");
        let duplicate_error = max_forwards_action(&duplicate).unwrap_err();
        assert_eq!(duplicate_error.etype, ErrorType::HTTPStatus(400));
    }

    #[test]
    fn non_trace_options_methods_leave_max_forwards_unchanged() {
        let mut request =
            RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
        request
            .insert_header("Max-Forwards", "0")
            .expect("fixture Max-Forwards must be valid");

        assert_eq!(
            max_forwards_action(&request).expect("GET may ignore Max-Forwards"),
            MaxForwardsAction::Ignore
        );
        apply_max_forwards_before_forward(&mut request)
            .expect("GET Max-Forwards is not proxy control");
        assert_eq!(request.headers["max-forwards"].to_str().unwrap(), "0");
    }

    #[test]
    fn request_via_hop_uses_the_received_http_version() {
        let mut request =
            RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");

        sanitize_forwarding_headers(&mut request, Version::HTTP_10)
            .expect("HTTP/1.0 Via metadata must remain valid");

        assert_eq!(
            request.headers["via"].to_str().unwrap(),
            "1.0 cwl-pingora-gateway"
        );
    }

    #[test]
    fn response_via_preserves_received_chain_and_appends_gateway() {
        let mut response =
            ResponseHeader::build(200, None).expect("fixture response must be valid");
        response.set_version(Version::HTTP_11);
        response
            .insert_header("Via", "1.0 origin-proxy")
            .expect("fixture Via header must be valid");

        append_response_via(&mut response).expect("gateway Via header must remain valid");

        let via_values = response
            .headers
            .get_all("via")
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            via_values,
            vec!["1.0 origin-proxy", "1.1 cwl-pingora-gateway"]
        );
    }

    #[test]
    fn max_forwards_final_recipient_holds_application_admission_lease() {
        let config = crate::edge_contract::GatewayConfig::from_yaml(
            r#"
version: 1
listener: 127.0.0.1:18180
metrics_listener: 127.0.0.1:18182
max_request_body_bytes: 8
max_in_flight_requests: 1
upstream_keepalive_pool_size: 1
upstreams:
  - name: test
    address: 127.0.0.1:18181
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

        let mut final_request =
            RequestHeader::build("OPTIONS", b"/", None).expect("fixture request must be valid");
        final_request
            .insert_header("Max-Forwards", "0")
            .expect("fixture Max-Forwards must be valid");
        let mut final_ctx = proxy.new_ctx();
        assert_eq!(
            proxy
                .admit_and_classify_max_forwards(&final_request, &mut final_ctx)
                .expect("final-recipient request is admitted"),
            MaxForwardsAction::FinalRecipient
        );
        assert!(final_ctx.admission.is_some());

        let ordinary_request =
            RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
        let mut saturated_ctx = proxy.new_ctx();
        let saturated_error = proxy
            .admit_and_classify_max_forwards(&ordinary_request, &mut saturated_ctx)
            .unwrap_err();
        assert_eq!(saturated_error.etype, ErrorType::HTTPStatus(503));

        drop(final_ctx);

        let mut recovered_ctx = proxy.new_ctx();
        assert_eq!(
            proxy
                .admit_and_classify_max_forwards(&ordinary_request, &mut recovered_ctx)
                .expect("released lease restores admission"),
            MaxForwardsAction::Ignore
        );
    }

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
