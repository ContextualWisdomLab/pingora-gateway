//! Explicit Pingora process policy for the version-1 shared edge runtime.
//!
//! Pingora's upstream defaults are framework defaults, not CWL product semantics. In particular,
//! the pinned Pingora line initializes `ServerConf::max_retries` to 16, each service runtime to one
//! worker thread, and the upstream keepalive pool to 128, while leaving graceful shutdown timing
//! unset. The shared runtime overrides values deliberately so a framework upgrade cannot silently
//! change replay, capacity, worker topology, or drain behavior.

use pingora::server::configuration::ServerConf;

/// Number of total upstream attempts admitted by the version-1 proxy runtime.
///
/// Pingora names this field `max_retries`, but its proxy loop executes while the attempt counter is
/// lower than this value. A value of one therefore means one initial attempt and zero retries.
pub const V1_MAX_UPSTREAM_ATTEMPTS: usize = 1;

/// Compatibility worker count used by version-1 configs that predate explicit topology control.
pub const V1_DEFAULT_SERVICE_THREADS: usize = 1;

/// Time between Pingora broadcasting service shutdown and beginning runtime shutdown.
///
/// Services observe the shutdown watch before this process-level sleep. An HTTP/2 connection can
/// therefore enqueue its initial graceful GOAWAY immediately while admitted or racing streams keep
/// draining during this interval. This value is not an artificial delay before GOAWAY emission.
pub const V1_GRACE_PERIOD_SECONDS: u64 = 5;

/// Timeout passed to each Pingora service runtime after the grace period.
///
/// The pinned Pingora server subsequently sleeps for this duration after `Runtime::shutdown_timeout`
/// returns. With runtimes shut down in parallel, the worst-case process budget is therefore the
/// grace period plus twice this value. Ten seconds keeps that bound below the 30-second external
/// termination budget while still allowing bounded in-flight cleanup.
pub const V1_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS: u64 = 10;

/// External hard-kill budget required from the process supervisor for a graceful SIGTERM exit.
pub const V1_TERMINATION_BUDGET_SECONDS: u64 = 30;

const _: () = assert!(
    V1_GRACE_PERIOD_SECONDS + 2 * V1_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS
        < V1_TERMINATION_BUDGET_SECONDS
);

/// Builds the compatibility Pingora server configuration for callers without explicit topology.
///
/// Production composition roots use the validated Admin Config path. This wrapper preserves
/// existing library callers at the historical one-worker topology rather than exposing an
/// unchecked worker-count mutation surface or deriving worker count from host CPU availability.
pub fn build_server_conf(upstream_keepalive_pool_size: usize) -> ServerConf {
    build_server_conf_with_service_threads(upstream_keepalive_pool_size, V1_DEFAULT_SERVICE_THREADS)
}

/// Builds Pingora process configuration after the owning Admin Config boundary has validated the
/// explicit service-worker topology.
///
/// This constructor is crate-private so external callers cannot bypass Admin Config validation and
/// inject zero or unbounded worker counts directly into `ServerConf::threads`.
pub(crate) fn build_server_conf_with_service_threads(
    upstream_keepalive_pool_size: usize,
    service_threads: usize,
) -> ServerConf {
    ServerConf {
        max_retries: V1_MAX_UPSTREAM_ATTEMPTS,
        threads: service_threads,
        upstream_keepalive_pool_size,
        grace_period_seconds: Some(V1_GRACE_PERIOD_SECONDS),
        graceful_shutdown_timeout_seconds: Some(V1_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS),
        ..ServerConf::default()
    }
}
