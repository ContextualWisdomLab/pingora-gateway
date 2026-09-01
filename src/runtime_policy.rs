//! Explicit Pingora process policy for the version-1 shared edge runtime.
//!
//! Pingora's upstream defaults are framework defaults, not CWL product semantics. In particular,
//! the pinned Pingora line initializes `ServerConf::max_retries` to 16 and leaves graceful
//! shutdown timing unset. The shared runtime overrides those values deliberately so a framework
//! upgrade cannot silently change request replay or drain behavior.

use pingora::server::configuration::ServerConf;

/// Number of total upstream attempts admitted by the version-1 proxy runtime.
///
/// Pingora names this field `max_retries`, but its proxy loop executes while the attempt counter is
/// lower than this value. A value of one therefore means one initial attempt and zero retries.
pub const V1_MAX_UPSTREAM_ATTEMPTS: usize = 1;

/// Time allowed after SIGTERM before runtime shutdown begins.
pub const V1_GRACE_PERIOD_SECONDS: u64 = 5;

/// Maximum time Pingora waits for service runtimes to finish after the grace period.
pub const V1_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS: u64 = 30;

/// Builds the Pingora server configuration admitted by the version-1 runtime policy.
///
/// Retry behavior is deliberately fixed to a single upstream attempt. Product-specific retry
/// semantics require idempotency knowledge and therefore remain outside the generic gateway until
/// a later version introduces an explicit, reviewed contract. Graceful shutdown is bounded rather
/// than inheriting Pingora's framework fallback.
pub fn build_server_conf() -> ServerConf {
    ServerConf {
        max_retries: V1_MAX_UPSTREAM_ATTEMPTS,
        grace_period_seconds: Some(V1_GRACE_PERIOD_SECONDS),
        graceful_shutdown_timeout_seconds: Some(V1_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS),
        ..ServerConf::default()
    }
}
