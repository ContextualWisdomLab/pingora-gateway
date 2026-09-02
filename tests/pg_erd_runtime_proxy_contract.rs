use std::net::SocketAddr;

use cwl_pingora_gateway::edge_contract::{UpstreamConfig, UpstreamTimeouts};
use cwl_pingora_gateway::edge_routing::{RouteMatch, RouteRule};
use cwl_pingora_gateway::http_policy::ResponseHeaderRule;
use cwl_pingora_gateway::migration_delivery::MigrationDeliveryPlan;
use cwl_pingora_gateway::migration_plan::EdgeMigrationPlan;
use cwl_pingora_gateway::migration_proxy::{MigrationGatewayProxy, MigrationGatewayProxyError};
use cwl_pingora_gateway::observability::{RequestObservation, RequestOutcome};
use cwl_pingora_gateway::runtime_isolation::RuntimeIsolationLimits;
use pingora::prelude::{ProxyHttp, RequestHeader, ResponseHeader};
use pingora::upstreams::peer::Peer;

fn upstream(name: &str, port: u16) -> UpstreamConfig {
    UpstreamConfig {
        name: name.to_string(),
        address: SocketAddr::from(([127, 0, 0, 1], port)),
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

fn pg_erd_delivery() -> MigrationDeliveryPlan {
    let plan = EdgeMigrationPlan::try_new(
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
        ],
    )
    .expect("captured pg-erd edge contract must compose");

    MigrationDeliveryPlan::try_new(
        plan,
        vec![upstream("backend", 8000), upstream("frontend", 3000)],
    )
    .expect("every characterized upstream must have one concrete peer")
}

fn proxy() -> MigrationGatewayProxy {
    let limits = RuntimeIsolationLimits::try_new(1_048_576, 128)
        .expect("non-zero runtime isolation limits must be valid");
    MigrationGatewayProxy::try_new(pg_erd_delivery(), limits)
        .expect("validated delivery and limits must activate callbacks")
}

#[test]
fn migration_proxy_is_a_real_pingora_http_application() {
    fn assert_proxy_http<T: ProxyHttp>() {}

    assert_proxy_http::<MigrationGatewayProxy>();
}

#[test]
fn migration_proxy_selects_characterized_concrete_peers_without_path_authority() {
    let proxy = proxy();

    assert_eq!(
        proxy
            .build_upstream_peer("/healthz")
            .expect("healthz route must select backend")
            .address()
            .to_string(),
        "127.0.0.1:8000"
    );
    assert_eq!(
        proxy
            .build_upstream_peer("/apiary")
            .expect("raw Traefik prefix parity must be preserved")
            .address()
            .to_string(),
        "127.0.0.1:8000"
    );
    assert_eq!(
        proxy
            .build_upstream_peer("/projects/42")
            .expect("frontend fallback must select frontend")
            .address()
            .to_string(),
        "127.0.0.1:3000"
    );
}

#[test]
fn migration_proxy_rejects_unmatched_paths_instead_of_inventing_a_destination() {
    let plan = EdgeMigrationPlan::try_new(
        vec!["backend".to_string()],
        vec![RouteRule {
            name: "healthz".to_string(),
            priority: 1,
            matcher: RouteMatch::Exact("/healthz".to_string()),
            upstream: "backend".to_string(),
        }],
        vec![ResponseHeaderRule {
            name: "X-Content-Type-Options".to_string(),
            value: "nosniff".to_string(),
        }],
    )
    .expect("bounded route contract must be valid");
    let delivery = MigrationDeliveryPlan::try_new(plan, vec![upstream("backend", 8000)])
        .expect("backend transport authority must bind");
    let limits = RuntimeIsolationLimits::try_new(1024, 1).expect("limits must be valid");
    let proxy = MigrationGatewayProxy::try_new(delivery, limits).expect("proxy must activate");

    assert_eq!(
        proxy
            .build_upstream_peer("/missing")
            .expect_err("unmatched path must remain unroutable"),
        MigrationGatewayProxyError::UnmatchedRoute {
            request_path: "/missing".to_string(),
        }
    );
}

#[test]
fn migration_proxy_applies_the_complete_characterized_response_policy() {
    let proxy = proxy();
    let mut response =
        ResponseHeader::build(200, None).expect("literal response status must be valid");
    response
        .insert_header("X-Frame-Options", "SAMEORIGIN")
        .expect("fixture header must be valid");

    proxy
        .apply_response_headers(&mut response)
        .expect("validated headers must be applicable");

    for (name, expected) in [
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "no-referrer"),
        (
            "permissions-policy",
            "geolocation=(), microphone=(), camera=()",
        ),
    ] {
        assert_eq!(
            response
                .headers
                .get(name)
                .expect("characterized response field must exist")
                .to_str()
                .expect("characterized response value must be text"),
            expected
        );
    }
}

#[test]
fn migration_proxy_replaces_untrusted_forwarding_identity() {
    let proxy = proxy();
    let mut request =
        RequestHeader::build("GET", b"/api", None).expect("fixture request must be valid");
    for (name, value) in [
        ("Forwarded", "for=203.0.113.7;proto=https"),
        ("X-Forwarded-For", "203.0.113.7"),
        ("X-Forwarded-Host", "attacker.example"),
        ("X-Forwarded-Proto", "https"),
        ("X-Real-IP", "203.0.113.7"),
    ] {
        request
            .insert_header(name, value)
            .expect("fixture forwarding field must be valid");
    }

    proxy
        .apply_upstream_request_policy(&mut request)
        .expect("static trusted forwarding policy must be valid");

    assert_eq!(
        request
            .headers
            .get("forwarded")
            .expect("trusted Forwarded field must be present")
            .to_str()
            .expect("Forwarded field must be text"),
        "proto=http"
    );
    for removed in [
        "x-forwarded-for",
        "x-forwarded-host",
        "x-forwarded-proto",
        "x-real-ip",
    ] {
        assert!(request.headers.get(removed).is_none());
    }
}

#[test]
fn migration_runtime_observation_is_low_cardinality_and_payload_free() {
    let ok = RequestObservation::from_parts(200, false, 4096);
    assert_eq!(ok.status(), 200);
    assert_eq!(ok.outcome(), RequestOutcome::Ok);
    assert_eq!(ok.request_body_bytes(), 4096);

    let failed = RequestObservation::from_parts(502, true, 17);
    assert_eq!(failed.status(), 502);
    assert_eq!(failed.outcome(), RequestOutcome::Error);
    assert_eq!(failed.request_body_bytes(), 17);
}
