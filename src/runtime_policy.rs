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

/// Time allowed after SIGTERM before runtime shutdown begins.
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
/// New production composition roots should call [`build_server_conf_with_service_threads`] with a
/// validated Admin Config value. This wrapper preserves existing library callers at the historical
/// one-worker topology rather than deriving worker count from host CPU availability.
pub fn build_server_conf(upstream_keepalive_pool_size: usize) -> ServerConf {
    build_server_conf_with_service_threads(upstream_keepalive_pool_size, V1_DEFAULT_SERVICE_THREADS)
}

/// Builds the Pingora server configuration with an explicit validated service-worker topology.
///
/// Pingora creates a distinct runtime for each service and gives each runtime `service_threads`
/// workers. The Admin Config boundary, not this constructor, rejects zero before listener
/// authority is granted. The constructor remains deterministic and never derives topology from the
/// host CPU count, which keeps capacity and NUMA evidence reproducible across environments.
pub fn build_server_conf_with_service_threads(
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
