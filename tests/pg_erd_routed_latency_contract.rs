use std::fs;

#[test]
fn routed_latency_is_gated_for_each_characterized_route() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");

    for threshold in [
        "'http_req_duration{route:backend}': ['p(95)<20']",
        "'http_req_duration{route:frontend}': ['p(95)<20']",
    ] {
        assert!(
            script.contains(threshold),
            "routed load contract must gate {threshold} independently"
        );
    }

    assert!(
        script.contains("const route = backendRoute ? 'backend' : 'frontend';"),
        "each measured request must carry an explicit characterized route identity"
    );
    assert!(
        script.contains("http.get(`${gatewayUrl}${path}`, { tags: { route } })"),
        "k6 request metrics must be tagged so per-route thresholds are enforceable"
    );
}
