use cwl_pingora_gateway::runtime_policy::{
    build_server_conf, V1_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS, V1_GRACE_PERIOD_SECONDS,
    V1_MAX_UPSTREAM_ATTEMPTS,
};

#[test]
fn version_one_overrides_pingora_retry_capacity_and_shutdown_defaults() {
    let conf = build_server_conf(32);

    assert_eq!(V1_MAX_UPSTREAM_ATTEMPTS, 1);
    assert_eq!(conf.max_retries, V1_MAX_UPSTREAM_ATTEMPTS);
    assert_eq!(conf.upstream_keepalive_pool_size, 32);
    assert_eq!(conf.grace_period_seconds, Some(V1_GRACE_PERIOD_SECONDS));
    assert_eq!(
        conf.graceful_shutdown_timeout_seconds,
        Some(V1_GRACEFUL_SHUTDOWN_TIMEOUT_SECONDS)
    );
}
