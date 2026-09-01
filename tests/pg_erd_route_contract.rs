use cwl_pingora_gateway::edge_routing::{RouteMatch, RoutePolicyError, RouteRule, RouteTable};

fn pg_erd_cloud_routes() -> Vec<RouteRule> {
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

#[test]
fn pg_erd_cloud_priority_contract_preserves_live_traefik_path_behavior() {
    let table = RouteTable::try_new(pg_erd_cloud_routes()).expect("live route contract is valid");

    assert_eq!(table.select_upstream("/healthz"), Some("backend"));
    assert_eq!(table.select_upstream("/api"), Some("backend"));
    assert_eq!(table.select_upstream("/api/erd"), Some("backend"));
    // Traefik PathPrefix(`/api`) is a raw path-prefix contract; parity therefore includes
    // `/apiary` until the owning product deliberately changes its edge contract.
    assert_eq!(table.select_upstream("/apiary"), Some("backend"));
    assert_eq!(table.select_upstream("/"), Some("frontend"));
    assert_eq!(table.select_upstream("/projects/42"), Some("frontend"));
}

#[test]
fn exact_route_does_not_match_longer_path_when_no_prefix_rule_applies() {
    let table = RouteTable::try_new(vec![RouteRule {
        name: "healthz".to_string(),
        priority: 110,
        matcher: RouteMatch::Exact("/healthz".to_string()),
        upstream: "backend".to_string(),
    }])
    .expect("single exact route is valid");

    assert_eq!(table.select_upstream("/healthz"), Some("backend"));
    assert_eq!(table.select_upstream("/healthz/deep"), None);
}

#[test]
fn equal_priorities_fail_closed_instead_of_inventing_tie_break_semantics() {
    let error = RouteTable::try_new(vec![
        RouteRule {
            name: "api".to_string(),
            priority: 100,
            matcher: RouteMatch::PathPrefix("/api".to_string()),
            upstream: "backend".to_string(),
        },
        RouteRule {
            name: "frontend".to_string(),
            priority: 100,
            matcher: RouteMatch::PathPrefix("/".to_string()),
            upstream: "frontend".to_string(),
        },
    ])
    .expect_err("ambiguous precedence must be rejected");

    assert_eq!(error, RoutePolicyError::DuplicatePriority { priority: 100 });
}

#[test]
fn malformed_route_authority_is_rejected_before_activation() {
    for rule in [
        RouteRule {
            name: "".to_string(),
            priority: 1,
            matcher: RouteMatch::PathPrefix("/".to_string()),
            upstream: "frontend".to_string(),
        },
        RouteRule {
            name: "frontend".to_string(),
            priority: 1,
            matcher: RouteMatch::PathPrefix("".to_string()),
            upstream: "frontend".to_string(),
        },
        RouteRule {
            name: "frontend".to_string(),
            priority: 1,
            matcher: RouteMatch::PathPrefix("relative".to_string()),
            upstream: "frontend".to_string(),
        },
        RouteRule {
            name: "frontend".to_string(),
            priority: 1,
            matcher: RouteMatch::PathPrefix("/".to_string()),
            upstream: "".to_string(),
        },
    ] {
        assert!(RouteTable::try_new(vec![rule]).is_err());
    }
}
