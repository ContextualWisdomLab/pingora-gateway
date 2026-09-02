use cwl_pingora_gateway::migration_admin::{
    PgErdMigrationConfig, PgErdMigrationConfigError, PG_ERD_MIGRATION_CONFIG_VERSION,
};

fn config_yaml(upstreams: &str) -> String {
    format!(
        r#"version: {PG_ERD_MIGRATION_CONFIG_VERSION}
listener: 127.0.0.1:8080
metrics_listener: 127.0.0.1:9090
max_request_body_bytes: 1048576
max_in_flight_requests: 128
upstream_keepalive_pool_size: 64
upstreams:
{upstreams}"#
    )
}

fn upstream_yaml(name: &str, port: u16) -> String {
    format!(
        r#"  - name: {name}
    address: 127.0.0.1:{port}
    tls: false
    timeouts:
      connection_ms: 100
      total_connection_ms: 200
      read_ms: 300
      write_ms: 400
      idle_ms: 500
"#
    )
}

fn valid_yaml() -> String {
    config_yaml(&format!(
        "{}{}",
        upstream_yaml("backend", 8000),
        upstream_yaml("frontend", 3000)
    ))
}

#[test]
fn pg_erd_admin_config_builds_only_the_characterized_migration_runtime() {
    let config = PgErdMigrationConfig::from_yaml(&valid_yaml())
        .expect("the characterized pg-erd admin config must parse");
    assert_eq!(config.listener().to_string(), "127.0.0.1:8080");
    assert_eq!(config.metrics_listener().to_string(), "127.0.0.1:9090");
    assert_eq!(config.upstream_keepalive_pool_size(), 64);

    let proxy = config
        .build_proxy()
        .expect("validated admin config must build the bounded migration proxy");
    assert_eq!(
        proxy
            .build_upstream_peer("/healthz")
            .expect("healthz must select backend")
            .address()
            .to_string(),
        "127.0.0.1:8000"
    );
    assert_eq!(
        proxy
            .build_upstream_peer("/apiary")
            .expect("raw Traefik prefix parity must remain explicit")
            .address()
            .to_string(),
        "127.0.0.1:8000"
    );
    assert_eq!(
        proxy
            .build_upstream_peer("/projects/42")
            .expect("fallback must select frontend")
            .address()
            .to_string(),
        "127.0.0.1:3000"
    );
}

#[test]
fn pg_erd_admin_config_rejects_listener_collision_and_zero_keepalive() {
    let listener_collision = valid_yaml().replace(
        "metrics_listener: 127.0.0.1:9090",
        "metrics_listener: 127.0.0.1:8080",
    );
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&listener_collision),
        Err(PgErdMigrationConfigError::ListenerCollision)
    );

    let zero_keepalive = valid_yaml().replace(
        "upstream_keepalive_pool_size: 64",
        "upstream_keepalive_pool_size: 0",
    );
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&zero_keepalive),
        Err(PgErdMigrationConfigError::InvalidUpstreamKeepalivePoolSize)
    );
}

#[test]
fn pg_erd_admin_config_rejects_missing_extra_duplicate_or_renamed_transport_authority() {
    let only_backend = config_yaml(&upstream_yaml("backend", 8000));
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&only_backend),
        Err(PgErdMigrationConfigError::TransportAuthorityCountMismatch {
            expected: 2,
            actual: 1,
        })
    );

    let extra = config_yaml(&format!(
        "{}{}{}",
        upstream_yaml("backend", 8000),
        upstream_yaml("frontend", 3000),
        upstream_yaml("shadow", 4000)
    ));
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&extra),
        Err(PgErdMigrationConfigError::TransportAuthorityCountMismatch {
            expected: 2,
            actual: 3,
        })
    );

    let duplicate = config_yaml(&format!(
        "{}{}",
        upstream_yaml("backend", 8000),
        upstream_yaml("backend", 8001)
    ));
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&duplicate),
        Err(PgErdMigrationConfigError::DuplicateTransportAuthority {
            upstream_name: "backend".to_string(),
        })
    );

    let renamed = config_yaml(&format!(
        "{}{}",
        upstream_yaml("api", 8000),
        upstream_yaml("frontend", 3000)
    ));
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&renamed),
        Err(PgErdMigrationConfigError::UnknownTransportAuthority {
            upstream_name: "api".to_string(),
        })
    );
}

#[test]
fn pg_erd_admin_config_rejects_unknown_fields_and_future_versions() {
    let unknown = valid_yaml().replace(
        "max_request_body_bytes: 1048576",
        "max_request_body_bytes: 1048576\nproduct_auth_mode: embedded",
    );
    assert!(matches!(
        PgErdMigrationConfig::from_yaml(&unknown),
        Err(PgErdMigrationConfigError::Parse(_))
    ));

    let future = valid_yaml().replace(
        &format!("version: {PG_ERD_MIGRATION_CONFIG_VERSION}"),
        "version: 2",
    );
    assert_eq!(
        PgErdMigrationConfig::from_yaml(&future),
        Err(PgErdMigrationConfigError::UnsupportedVersion(2))
    );
}
