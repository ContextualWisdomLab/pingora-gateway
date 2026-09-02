use pingora_gateway::edge_contract::{GatewayConfigError, UpstreamConfig, UpstreamTimeouts};
use pingora_gateway::edge_routing::{RouteMatch, RouteRule};
use pingora_gateway::http_policy::ResponseHeaderRule;
use pingora_gateway::migration_delivery::{MigrationDeliveryError, MigrationDeliveryPlan};
use pingora_gateway::migration_plan::EdgeMigrationPlan;
use pingora_gateway::pingora_delivery::PeerBuildError;

fn pg_erd_plan() -> EdgeMigrationPlan {
    EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), "frontend".to_string()],
        vec![
            RouteRule {
                name: "healthz".to_string(),
                priority: 110,
                matcher: RouteMatch::Exact("/healthz".to_string()),
                upstream: "backend".to_string(),
            },
            RouteRule {
                name: "api".to_string(),
                priority: 100,
                matcher: RouteMatch::PathPrefix("/api".to_string()),
                upstream: "backend".to_string(),
            },
            RouteRule {
                name: "frontend".to_string(),
                priority: 1,
                matcher: RouteMatch::PathPrefix("/".to_string()),
                upstream: "frontend".to_string(),
            },
        ],
        vec![ResponseHeaderRule {
            name: "X-Content-Type-Options".to_string(),
            value: "nosniff".to_string(),
        }],
    )
    .expect("captured pg-erd migration plan must be valid")
}

fn upstream(name: &str, port: u16) -> UpstreamConfig {
    UpstreamConfig {
        name: name.to_string(),
        address: format!("127.0.0.1:{port}")
            .parse()
            .expect("loopback fixture must parse"),
        tls: false,
        sni: None,
        trust_bundle_file: None,
        timeouts: UpstreamTimeouts {
            connection_ms: 100,
            total_connection_ms: 200,
            read_ms: 300,
            write_ms: 400,
            idle_ms: 500,
        },
    }
}

#[test]
fn pg_erd_plan_binds_only_complete_explicit_transport_authority() {
    let delivery = MigrationDeliveryPlan::try_new(
        pg_erd_plan(),
        vec![upstream("backend", 8000), upstream("frontend", 3000)],
    )
    .expect("both characterized upstreams have explicit transport authority");

    assert_eq!(delivery.upstream_count(), 2);
    assert_eq!(delivery.select_upstream_name("/healthz"), Some("backend"));
    assert_eq!(delivery.select_upstream_name("/api/items"), Some("backend"));
    assert_eq!(delivery.select_upstream_name("/apiary"), Some("backend"));
    assert_eq!(delivery.select_upstream_name("/"), Some("frontend"));
    assert_eq!(delivery.select_upstream_name("relative"), None);
    assert!(delivery.build_upstream_peer("/api/items").is_some());
    assert!(delivery.build_upstream_peer("relative").is_none());
    assert_eq!(
        delivery.response_header_value("x-content-type-options"),
        Some("nosniff")
    );
    assert_eq!(delivery.response_header_value("server"), None);
}

#[test]
fn pg_erd_plan_rejects_missing_transport_authority() {
    let error = MigrationDeliveryPlan::try_new(pg_erd_plan(), vec![upstream("backend", 8000)])
        .expect_err("a characterized upstream must not remain unbound");

    assert_eq!(
        error,
        MigrationDeliveryError::UpstreamAuthorityCountMismatch {
            expected: 2,
            actual: 1,
        }
    );
}

#[test]
fn pg_erd_plan_rejects_undeclared_transport_authority() {
    let error = MigrationDeliveryPlan::try_new(
        pg_erd_plan(),
        vec![upstream("backend", 8000), upstream("search", 9000)],
    )
    .expect_err("migration delivery must not invent service discovery authority");

    assert_eq!(
        error,
        MigrationDeliveryError::UnknownConfiguredUpstream {
            upstream_name: "search".to_string(),
        }
    );
}

#[test]
fn pg_erd_plan_rejects_duplicate_transport_authority() {
    let error = MigrationDeliveryPlan::try_new(
        pg_erd_plan(),
        vec![upstream("backend", 8000), upstream("backend", 8001)],
    )
    .expect_err("one stable upstream identity must bind to one transport authority");

    assert_eq!(
        error,
        MigrationDeliveryError::DuplicateConfiguredUpstream {
            upstream_name: "backend".to_string(),
        }
    );
}

#[test]
fn pg_erd_plan_rejects_invalid_admitted_transport_before_activation() {
    let mut invalid_backend = upstream("backend", 8000);
    invalid_backend.tls = true;

    let error = MigrationDeliveryPlan::try_new(
        pg_erd_plan(),
        vec![invalid_backend, upstream("frontend", 3000)],
    )
    .expect_err("admitted identity does not bypass TLS transport validation");

    assert_eq!(
        error,
        MigrationDeliveryError::UpstreamActivation {
            upstream_name: "backend".to_string(),
            source: PeerBuildError::InvalidConfiguration(
                GatewayConfigError::MissingTlsServerName {
                    upstream_name: "backend".to_string(),
                }
            ),
        }
    );
}
