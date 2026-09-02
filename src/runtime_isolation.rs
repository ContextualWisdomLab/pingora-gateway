//! Transport-neutral runtime-isolation budgets shared by Pingora delivery adapters.
//!
//! This bounded context owns request-body, concurrent-request admission, and optional upstream
//! response-body lifetime limits. It does not select routes, mutate HTTP policy, authenticate
//! callers, or make product-domain decisions.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use thiserror::Error;

/// Invalid runtime-isolation configuration rejected before a listener gains network authority.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimeIsolationConfigError {
    /// A zero body limit would make every non-empty request invalid.
    #[error("max_request_body_bytes must be greater than zero")]
    ZeroMaxRequestBodyBytes,
    /// A zero in-flight limit would reject every proxied request.
    #[error("max_in_flight_requests must be greater than zero")]
    ZeroMaxInFlightRequests,
    /// A zero response-body lifetime would reject every non-empty upstream response immediately.
    #[error("max_upstream_response_body_ms must be greater than zero")]
    ZeroMaxUpstreamResponseBodyMs,
}

/// Immutable request-isolation limits shared by gateway delivery adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeIsolationLimits {
    max_request_body_bytes: u64,
    max_in_flight_requests: usize,
    max_upstream_response_body_ms: Option<u64>,
}

impl RuntimeIsolationLimits {
    /// Validates explicit non-zero body and in-flight request budgets.
    ///
    /// The generic v1 contract has no response-body lifetime field, so this constructor preserves
    /// that versioned behavior rather than inventing a hidden timeout.
    pub fn try_new(
        max_request_body_bytes: u64,
        max_in_flight_requests: usize,
    ) -> Result<Self, RuntimeIsolationConfigError> {
        Self::try_new_internal(max_request_body_bytes, max_in_flight_requests, None)
    }

    /// Validates request isolation plus an explicit upstream response-body lifetime budget.
    pub(crate) fn try_new_with_response_body_limit(
        max_request_body_bytes: u64,
        max_in_flight_requests: usize,
        max_upstream_response_body_ms: u64,
    ) -> Result<Self, RuntimeIsolationConfigError> {
        Self::try_new_internal(
            max_request_body_bytes,
            max_in_flight_requests,
            Some(max_upstream_response_body_ms),
        )
    }

    fn try_new_internal(
        max_request_body_bytes: u64,
        max_in_flight_requests: usize,
        max_upstream_response_body_ms: Option<u64>,
    ) -> Result<Self, RuntimeIsolationConfigError> {
        if max_request_body_bytes == 0 {
            return Err(RuntimeIsolationConfigError::ZeroMaxRequestBodyBytes);
        }
        if max_in_flight_requests == 0 {
            return Err(RuntimeIsolationConfigError::ZeroMaxInFlightRequests);
        }
        if max_upstream_response_body_ms == Some(0) {
            return Err(RuntimeIsolationConfigError::ZeroMaxUpstreamResponseBodyMs);
        }
        Ok(Self {
            max_request_body_bytes,
            max_in_flight_requests,
            max_upstream_response_body_ms,
        })
    }

    pub(crate) fn from_validated(
        max_request_body_bytes: u64,
        max_in_flight_requests: usize,
    ) -> Self {
        Self {
            max_request_body_bytes,
            max_in_flight_requests,
            max_upstream_response_body_ms: None,
        }
    }

    /// Returns the maximum admitted downstream request-body size in bytes.
    pub fn max_request_body_bytes(self) -> u64 {
        self.max_request_body_bytes
    }

    /// Returns the maximum number of concurrently admitted non-health requests.
    pub fn max_in_flight_requests(self) -> usize {
        self.max_in_flight_requests
    }

    /// Returns the configured upstream response-body lifetime, when the active contract owns one.
    pub fn max_upstream_response_body_ms(self) -> Option<u64> {
        self.max_upstream_response_body_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BodyLimitExceeded {
    pub(crate) observed: u64,
    pub(crate) limit: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct RequestAdmissionBudget {
    in_flight: Arc<AtomicUsize>,
    limit: usize,
}

impl RequestAdmissionBudget {
    pub(crate) fn new(limits: RuntimeIsolationLimits) -> Self {
        Self {
            in_flight: Arc::new(AtomicUsize::new(0)),
            limit: limits.max_in_flight_requests(),
        }
    }

    pub(crate) fn acquire(&self) -> Option<RequestAdmission> {
        self.in_flight
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < self.limit).then_some(current + 1)
            })
            .ok()
            .map(|_| RequestAdmission {
                in_flight: Arc::clone(&self.in_flight),
            })
    }
}

#[derive(Debug)]
pub(crate) struct RequestAdmission {
    in_flight: Arc<AtomicUsize>,
}

impl Drop for RequestAdmission {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Debug)]
pub(crate) struct RequestBodyBudget {
    observed: u64,
    limit: u64,
}

impl RequestBodyBudget {
    pub(crate) fn new(limits: RuntimeIsolationLimits) -> Self {
        Self {
            observed: 0,
            limit: limits.max_request_body_bytes(),
        }
    }

    pub(crate) fn reject_declared_length(&self, declared: u64) -> Result<(), BodyLimitExceeded> {
        if declared > self.limit {
            return Err(BodyLimitExceeded {
                observed: declared,
                limit: self.limit,
            });
        }
        Ok(())
    }

    pub(crate) fn observe_chunk(&mut self, chunk_bytes: u64) -> Result<(), BodyLimitExceeded> {
        self.observed = self.observed.saturating_add(chunk_bytes);
        if self.observed > self.limit {
            return Err(BodyLimitExceeded {
                observed: self.observed,
                limit: self.limit,
            });
        }
        Ok(())
    }

    pub(crate) fn observed(&self) -> u64 {
        self.observed
    }
}

/// Elapsed-time guard for the body phase of one admitted upstream response.
#[derive(Debug)]
pub(crate) struct ResponseBodyLifetimeBudget {
    limit: Option<Duration>,
    started_at: Option<Instant>,
}

/// Evidence that a response-body progress callback arrived after its configured lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResponseBodyLifetimeExceeded {
    pub(crate) elapsed: Duration,
    pub(crate) limit: Duration,
}

impl ResponseBodyLifetimeBudget {
    /// Creates a dormant response-body budget from the active runtime-isolation contract.
    pub(crate) fn new(limits: RuntimeIsolationLimits) -> Self {
        Self {
            limit: limits.max_upstream_response_body_ms().map(Duration::from_millis),
            started_at: None,
        }
    }

    /// Starts the body lifetime once; repeated response-filter callbacks cannot reset the deadline.
    pub(crate) fn start(&mut self, now: Instant) {
        if self.limit.is_some() && self.started_at.is_none() {
            self.started_at = Some(now);
        }
    }

    /// Rejects the first observed body-progress boundary at or beyond the configured lifetime.
    pub(crate) fn reject_if_expired(
        &self,
        now: Instant,
    ) -> Result<(), ResponseBodyLifetimeExceeded> {
        let (Some(limit), Some(started_at)) = (self.limit, self.started_at) else {
            return Ok(());
        };
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed >= limit {
            return Err(ResponseBodyLifetimeExceeded { elapsed, limit });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        BodyLimitExceeded, RequestAdmissionBudget, RequestBodyBudget, ResponseBodyLifetimeBudget,
        ResponseBodyLifetimeExceeded, RuntimeIsolationConfigError, RuntimeIsolationLimits,
    };

    #[test]
    fn limits_fail_closed_on_zero_budgets_and_preserve_valid_values() {
        assert_eq!(
            RuntimeIsolationLimits::try_new(0, 1),
            Err(RuntimeIsolationConfigError::ZeroMaxRequestBodyBytes)
        );
        assert_eq!(
            RuntimeIsolationLimits::try_new(1, 0),
            Err(RuntimeIsolationConfigError::ZeroMaxInFlightRequests)
        );
        assert_eq!(
            RuntimeIsolationLimits::try_new_with_response_body_limit(1, 1, 0),
            Err(RuntimeIsolationConfigError::ZeroMaxUpstreamResponseBodyMs)
        );

        let limits = RuntimeIsolationLimits::try_new(1024, 2).expect("non-zero limits are valid");
        assert_eq!(limits.max_request_body_bytes(), 1024);
        assert_eq!(limits.max_in_flight_requests(), 2);
        assert_eq!(limits.max_upstream_response_body_ms(), None);

        let bounded = RuntimeIsolationLimits::try_new_with_response_body_limit(2048, 3, 750)
            .expect("explicit positive response lifetime must be valid");
        assert_eq!(bounded.max_request_body_bytes(), 2048);
        assert_eq!(bounded.max_in_flight_requests(), 3);
        assert_eq!(bounded.max_upstream_response_body_ms(), Some(750));
    }

    #[test]
    fn admission_budget_recovers_after_request_release() {
        let limits = RuntimeIsolationLimits::try_new(1024, 1).expect("fixture limits are valid");
        let budget = RequestAdmissionBudget::new(limits);
        let first = budget.acquire().expect("first request is admitted");

        assert!(budget.acquire().is_none());
        drop(first);
        assert!(budget.acquire().is_some());
    }

    #[test]
    fn body_budget_enforces_declared_and_streamed_limits() {
        let limits = RuntimeIsolationLimits::try_new(4, 1).expect("fixture limits are valid");
        let mut body = RequestBodyBudget::new(limits);

        assert!(body.reject_declared_length(4).is_ok());
        assert_eq!(
            body.reject_declared_length(5).unwrap_err(),
            BodyLimitExceeded {
                observed: 5,
                limit: 4,
            }
        );

        body.observe_chunk(2).expect("first chunk is within budget");
        body.observe_chunk(2).expect("exact limit remains admitted");
        assert_eq!(body.observed(), 4);
        assert_eq!(
            body.observe_chunk(1).unwrap_err(),
            BodyLimitExceeded {
                observed: 5,
                limit: 4,
            }
        );
    }

    #[test]
    fn response_body_lifetime_is_dormant_without_a_versioned_budget() {
        let limits = RuntimeIsolationLimits::try_new(4, 1).expect("fixture limits are valid");
        let mut budget = ResponseBodyLifetimeBudget::new(limits);
        let now = Instant::now();

        budget.start(now);
        assert!(budget
            .reject_if_expired(now + Duration::from_secs(60))
            .is_ok());
    }

    #[test]
    fn response_body_lifetime_starts_once_and_rejects_at_the_limit() {
        let limits = RuntimeIsolationLimits::try_new_with_response_body_limit(4, 1, 300)
            .expect("fixture limits are valid");
        let mut budget = ResponseBodyLifetimeBudget::new(limits);
        let started = Instant::now();
        budget.start(started);
        budget.start(started + Duration::from_millis(250));

        assert!(budget
            .reject_if_expired(started + Duration::from_millis(299))
            .is_ok());
        assert_eq!(
            budget
                .reject_if_expired(started + Duration::from_millis(300))
                .unwrap_err(),
            ResponseBodyLifetimeExceeded {
                elapsed: Duration::from_millis(300),
                limit: Duration::from_millis(300),
            }
        );
    }
}
