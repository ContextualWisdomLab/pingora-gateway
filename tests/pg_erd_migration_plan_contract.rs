use pingora_gateway::edge_routing::{RouteMatch, RoutePolicyError, RouteRule};
use pingora_gateway::http_policy::{HeaderPolicyError, ResponseHeaderRule};
use pingora_gateway::migration_plan::{EdgeMigrationPlan, MigrationPlanError};

fn pg_erd_routes() -> Vec<RouteRule> {
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
    ]
}

fn pg_erd_headers() -> Vec<ResponseHeaderRule> {
    vec![
        ResponseHeaderRule {
            name: "X-Content-Type-Options".to_string(),
            value: "nosniff".to_string(),
        },
        ResponseHeaderRule {
            name: "X-Frame-Options".to_string(),
            value: "DENY".to_string(),
        },
        ResponseHeaderRule {
            name: "Referrer-Policy".to_string(),
            value: "no-referrer".to_string(),
        },
        ResponseHeaderRule {
            name: "Permissions-Policy".to_string(),
            value: "geolocation=(), microphone=(), camera=()".to_string(),
        },
    ]
}

#[test]
fn pg_erd_cloud_plan_preserves_route_and_header_contract() {
    let plan = EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), "frontend".to_string()],
        pg_erd_routes(),
        pg_erd_headers(),
    )
    .expect("captured pg-erd-cloud edge contract must compose");

    assert_eq!(plan.upstream_count(), 2);
    assert!(plan.contains_upstream("backend"));
    assert!(plan.contains_upstream(" frontend "));
    assert!(!plan.contains_upstream("unknown"));
    assert_eq!(plan.select_upstream("/healthz"), Some("backend"));
    assert_eq!(plan.select_upstream("/api/users"), Some("backend"));
    assert_eq!(plan.select_upstream("/apiary"), Some("backend"));
    assert_eq!(plan.select_upstream("/"), Some("frontend"));
    assert_eq!(plan.select_upstream("not-an-http-path"), None);
    assert_eq!(
        plan.response_header_value("x-content-type-options"),
        Some("nosniff")
    );
    assert_eq!(plan.response_header_value("X-Frame-Options"), Some("DENY"));
    assert_eq!(
        plan.response_header_value("Permissions-Policy"),
        Some("geolocation=(), microphone=(), camera=()")
    );
    assert_eq!(plan.response_header_value("Server"), None);
}

#[test]
fn migration_plan_requires_explicit_upstream_authority() {
    let error = EdgeMigrationPlan::try_new(Vec::new(), pg_erd_routes(), pg_erd_headers())
        .expect_err("a migration plan without upstream authority must fail closed");

    assert_eq!(error, MigrationPlanError::NoUpstreams);
}

#[test]
fn migration_plan_rejects_empty_upstream_identity() {
    let error = EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), "   ".to_string()],
        pg_erd_routes(),
        pg_erd_headers(),
    )
    .expect_err("empty upstream identity must fail closed");

    assert_eq!(error, MigrationPlanError::EmptyUpstreamName);
}

#[test]
fn migration_plan_rejects_duplicate_normalized_upstream_identity() {
    let error = EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), " backend ".to_string()],
        pg_erd_routes(),
        pg_erd_headers(),
    )
    .expect_err("duplicate normalized upstream identity must fail closed");

    assert_eq!(
        error,
        MigrationPlanError::DuplicateUpstreamName {
            upstream_name: "backend".to_string(),
        }
    );
}

#[test]
fn migration_plan_rejects_route_to_unknown_upstream() {
    let mut routes = pg_erd_routes();
    routes[1].upstream = "shadow-backend".to_string();

    let error = EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), "frontend".to_string()],
        routes,
        pg_erd_headers(),
    )
    .expect_err("route targets must belong to the explicit upstream authority set");

    assert_eq!(
        error,
        MigrationPlanError::UnknownRouteUpstream {
            route_name: "api".to_string(),
            upstream_name: "shadow-backend".to_string(),
        }
    );
}

#[test]
fn migration_plan_preserves_route_policy_fail_closed_errors() {
    let error = EdgeMigrationPlan::try_new(
        vec!["backend".to_string()],
        Vec::new(),
        pg_erd_headers(),
    )
    .expect_err("invalid route tables must remain invalid when composed");

    assert_eq!(
        error,
        MigrationPlanError::RoutePolicy(RoutePolicyError::NoRoutes)
    );
}

#[test]
fn migration_plan_preserves_http_policy_fail_closed_errors() {
    let error = EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), "frontend".to_string()],
        pg_erd_routes(),
        Vec::new(),
    )
    .expect_err("invalid response policies must remain invalid when composed");

    assert_eq!(
        error,
        MigrationPlanError::HeaderPolicy(HeaderPolicyError::NoHeaders)
    );
}
