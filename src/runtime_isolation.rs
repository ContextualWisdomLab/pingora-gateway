//! Transport-neutral runtime-isolation budgets shared by Pingora delivery adapters.
//!
//! This bounded context owns request-body and concurrent-request admission limits. It does not
//! select routes, mutate HTTP policy, authenticate callers, or make product-domain decisions.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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
}

/// Immutable request-isolation limits shared by gateway delivery adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeIsolationLimits {
    max_request_body_bytes: u64,
    max_in_flight_requests: usize,
}

impl RuntimeIsolationLimits {
    /// Validates explicit non-zero body and in-flight request budgets.
    pub fn try_new(
        max_request_body_bytes: u64,
        max_in_flight_requests: usize,
    ) -> Result<Self, RuntimeIsolationConfigError> {
        if max_request_body_bytes == 0 {
            return Err(RuntimeIsolationConfigError::ZeroMaxRequestBodyBytes);
        }
        if max_in_flight_requests == 0 {
            return Err(RuntimeIsolationConfigError::ZeroMaxInFlightRequests);
        }
        Ok(Self {
            max_request_body_bytes,
            max_in_flight_requests,
        })
    }

    pub(crate) fn from_validated(
        max_request_body_bytes: u64,
        max_in_flight_requests: usize,
    ) -> Self {
        Self {
            max_request_body_bytes,
            max_in_flight_requests,
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

#[cfg(test)]
mod tests {
    use super::{
        BodyLimitExceeded, RequestAdmissionBudget, RequestBodyBudget, RuntimeIsolationConfigError,
        RuntimeIsolationLimits,
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

        let limits = RuntimeIsolationLimits::try_new(1024, 2).expect("non-zero limits are valid");
        assert_eq!(limits.max_request_body_bytes(), 1024);
        assert_eq!(limits.max_in_flight_requests(), 2);
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
}
