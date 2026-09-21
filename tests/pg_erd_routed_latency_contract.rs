use std::fs;

fn threshold_section_contains_exact_entry(script: &str, expected: &str) -> bool {
    let mut in_options = false;
    let mut in_thresholds = false;
    let mut in_block_comment = false;

    for raw_line in script.lines() {
        let line = raw_line.trim();

        if in_block_comment {
            if line.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if line.starts_with("/*") {
            if !line.contains("*/") {
                in_block_comment = true;
            }
            continue;
        }
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if !in_options {
            if line == "export const options = {" {
                in_options = true;
            }
            continue;
        }

        if !in_thresholds {
            if line == "thresholds: {" {
                in_thresholds = true;
                continue;
            }
            if line == "};" {
                return false;
            }
            continue;
        }

        if line == "}," {
            return false;
        }
        if line == expected {
            return true;
        }
    }

    false
}

#[test]
fn routed_latency_is_gated_for_each_characterized_route() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");

    for threshold in [
        "'http_req_duration{route:backend}': ['p(95)<20'],",
        "'http_req_duration{route:frontend}': ['p(95)<20'],",
        "'http_reqs{route:backend}': ['count>=198'],",
        "'http_reqs{route:frontend}': ['count>=198'],",
    ] {
        assert!(
            threshold_section_contains_exact_entry(&script, threshold),
            "routed load contract must gate {threshold} independently inside active k6 thresholds"
        );
    }

    assert!(
        script.contains("const backendRoute = (__VU + __ITER) % 2 === 0;"),
        "the fixed shared-iteration contract must alternate route identity per VU"
    );
    assert!(
        script.contains("const route = backendRoute ? 'backend' : 'frontend';"),
        "each measured request must carry an explicit characterized route identity"
    );
    assert!(
        script.contains("const path = backendRoute ? '/api/load-contract' : '/load-contract';"),
        "route identity must remain bound to the characterized backend/fallback path"
    );
    assert!(
        script.contains("const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';"),
        "route identity must remain bound to its characterized origin body"
    );
    assert!(
        script.contains("http.get(`${gatewayUrl}${path}`, { tags: { route } })"),
        "k6 request metrics must be tagged so per-route thresholds are enforceable"
    );
}

#[test]
fn commented_threshold_bait_does_not_satisfy_the_contract() {
    let commented_bait = r#"
export const options = {
  thresholds: {
    // 'http_req_duration{route:backend}': ['p(95)<20'],
  },
};
"#;

    assert!(
        !threshold_section_contains_exact_entry(
            commented_bait,
            "'http_req_duration{route:backend}': ['p(95)<20'],"
        ),
        "commented threshold text must not manufacture routed-latency evidence"
    );
}
