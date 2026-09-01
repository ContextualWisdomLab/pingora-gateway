//! Low-cardinality gateway telemetry.

use std::sync::atomic::{AtomicU64, Ordering};

/// Process-wide counters intentionally free of request-controlled labels.
#[derive(Debug, Default)]
pub struct Metrics {
    requests: AtomicU64,
    rejected: AtomicU64,
    proxy_errors: AtomicU64,
}

impl Metrics {
    /// Record a downstream request entering the gateway.
    pub fn request(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a policy or routing rejection.
    pub fn rejected(&self) {
        self.rejected.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a proxy failure after request admission.
    pub fn proxy_error(&self) {
        self.proxy_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Render Prometheus text exposition without credential-bearing dimensions.
    pub fn render(&self) -> String {
        format!(
            "# TYPE pingora_gateway_requests_total counter\npingora_gateway_requests_total {}\n# TYPE pingora_gateway_rejected_total counter\npingora_gateway_rejected_total {}\n# TYPE pingora_gateway_proxy_errors_total counter\npingora_gateway_proxy_errors_total {}\n",
            self.requests.load(Ordering::Relaxed),
            self.rejected.load(Ordering::Relaxed),
            self.proxy_errors.load(Ordering::Relaxed)
        )
    }
}
