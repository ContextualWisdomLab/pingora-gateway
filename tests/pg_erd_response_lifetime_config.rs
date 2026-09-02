use cwl_pingora_gateway::migration_admin::{
    PgErdMigrationConfig, PgErdMigrationConfigError, PG_ERD_MIGRATION_CONFIG_VERSION,
    PG_ERD_RESPONSE_LIFETIME_CONFIG_VERSION,
};
use cwl_pingora_gateway::runtime_isolation::RuntimeIsolationConfigError;

fn config_yaml(version: u32, lifetime_line: &str) -> String {
    format!(
        "version: {version}\nlistener: 127.0.0.1:18080\nmetrics_listener: 127.0.0.1:19090\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 8\n{lifetime_line}upstream_keepalive_pool_size: 4\nupstreams:\n  - name: backend\n    address: 127.0.0.1:18000\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n  - name: frontend\n    address: 127.0.0.1:13000\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n"
    )
}

#[test]
fn version_two_requires_and_preserves_an_explicit_positive_response_body_lifetime() {
    let configured = PgErdMigrationConfig::from_yaml(&config_yaml(
        PG_ERD_RESPONSE_LIFETIME_CONFIG_VERSION,
        "max_upstream_response_body_ms: 750\n",
    ))
    .expect("version 2 with an explicit positive lifetime must parse");
    assert_eq!(configured.max_upstream_response_body_ms(), Some(750));
    configured
        .build_proxy()
        .expect("version-2 runtime budgets must materialize before listeners open");

    assert_eq!(
        PgErdMigrationConfig::from_yaml(&config_yaml(
            PG_ERD_RESPONSE_LIFETIME_CONFIG_VERSION,
            "max_upstream_response_body_ms: 0\n",
        )),
        Err(PgErdMigrationConfigError::RuntimeIsolation(
            RuntimeIsolationConfigError::ZeroMaxUpstreamResponseBodyMs,
        ))
    );

    assert_eq!(
        PgErdMigrationConfig::from_yaml(&config_yaml(
            PG_ERD_RESPONSE_LIFETIME_CONFIG_VERSION,
            "",
        )),
        Err(PgErdMigrationConfigError::MissingUpstreamResponseBodyLifetime)
    );
}

#[test]
fn version_one_cannot_silently_acquire_version_two_response_semantics() {
    let legacy = PgErdMigrationConfig::from_yaml(&config_yaml(PG_ERD_MIGRATION_CONFIG_VERSION, ""))
        .expect("legacy version 1 remains readable without a hidden response lifetime");
    assert_eq!(legacy.max_upstream_response_body_ms(), None);

    assert_eq!(
        PgErdMigrationConfig::from_yaml(&config_yaml(
            PG_ERD_MIGRATION_CONFIG_VERSION,
            "max_upstream_response_body_ms: 750\n",
        )),
        Err(PgErdMigrationConfigError::ResponseBodyLifetimeRequiresVersion2)
    );
}
