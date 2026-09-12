//! Low-cardinality request observability shared by Pingora delivery adapters.
//!
//! This bounded context records transport outcomes and byte counts only. It deliberately excludes
//! request paths, query strings, headers, cookies, credentials, customer payloads, and product
//! domain identifiers from the shared gateway telemetry contract.

use std::sync::LazyLock;

use log::info;
use pingora::prelude::{Error, Session};
use pingora_prometheus::prometheus::{register_int_counter, IntCounter};

/// Stable low-cardinality outcome for one completed downstream request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// The Pingora request lifecycle completed without an error.
    Ok,
    /// The Pingora request lifecycle completed with an error.
    Error,
}

/// Payload-free observation produced at the end of a downstream request lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestObservation {
    status: u16,
    outcome: RequestOutcome,
    request_body_bytes: u64,
}

impl RequestObservation {
    /// Builds a deterministic observation from transport-only request completion facts.
    pub fn from_parts(status: u16, had_error: bool, request_body_bytes: u64) -> Self {
        Self {
            status,
            outcome: if had_error {
                RequestOutcome::Error
            } else {
                RequestOutcome::Ok
            },
            request_body_bytes,
        }
    }

    /// Returns the downstream response status, or zero when no response header was written.
    pub fn status(self) -> u16 {
        self.status
    }

    /// Returns the low-cardinality request outcome.
    pub fn outcome(self) -> RequestOutcome {
        self.outcome
    }

    /// Returns request-body bytes observed before completion or rejection.
    pub fn request_body_bytes(self) -> u64 {
        self.request_body_bytes
    }

    fn record(self) {
        REQUESTS_TOTAL.inc();
        REQUEST_BODY_BYTES_TOTAL.inc_by(self.request_body_bytes);
        if self.outcome == RequestOutcome::Error {
            REQUEST_ERRORS_TOTAL.inc();
        }

        let outcome = match self.outcome {
            RequestOutcome::Ok => "ok",
            RequestOutcome::Error => "error",
        };
        info!(
            "gateway_request status={} outcome={outcome} request_body_bytes={}",
            self.status, self.request_body_bytes
        );
    }
}

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

pub(crate) fn record_request(session: &Session, error: Option<&Error>, request_body_bytes: u64) {
    let status = session
        .response_written()
        .map_or(0, |response| response.status.as_u16());
    RequestObservation::from_parts(status, error.is_some(), request_body_bytes).record();
}

pub(crate) fn record_backpressure_rejection() {
    BACKPRESSURE_REJECTIONS_TOTAL.inc();
}

#[cfg(test)]
mod tests {
    use std::panic;

    use super::{register_counter, RequestObservation, RequestOutcome};

    #[test]
    fn request_observation_maps_error_state_without_payload_fields() {
        assert_eq!(
            RequestObservation::from_parts(204, false, 12),
            RequestObservation {
                status: 204,
                outcome: RequestOutcome::Ok,
                request_body_bytes: 12,
            }
        );
        assert_eq!(
            RequestObservation::from_parts(503, true, 0).outcome(),
            RequestOutcome::Error
        );
    }

    #[test]
    fn duplicate_metric_registration_fails_closed() {
        let name = "cwl_pingora_gateway_test_duplicate_registration_total";
        let help = "Coverage-only counter proving duplicate registration fails closed";
        let _first = register_counter(name, help);

        let duplicate = panic::catch_unwind(|| register_counter(name, help));

        assert!(duplicate.is_err());
    }
}
