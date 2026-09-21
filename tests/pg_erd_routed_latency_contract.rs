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

fn contains_active_binding_line(script: &str, expected: &str) -> bool {
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
        if line == expected {
            return true;
        }
    }

    false
}

fn current_oracle_accepts_without_workload_shape(script: &str) -> bool {
    [
        "'http_req_duration{route:backend}': ['p(95)<20'],",
        "'http_req_duration{route:frontend}': ['p(95)<20'],",
        "'http_reqs{route:backend}': ['count>=198'],",
        "'http_reqs{route:frontend}': ['count>=198'],",
    ]
    .iter()
    .all(|threshold| threshold_section_contains_exact_entry(script, threshold))
        && [
            "const backendRoute = (__VU + __ITER) % 2 === 0;",
            "const route = backendRoute ? 'backend' : 'frontend';",
            "const path = backendRoute ? '/api/load-contract' : '/load-contract';",
            "const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';",
            "const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });",
        ]
        .iter()
        .all(|binding| contains_active_binding_line(script, binding))
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

    for binding in [
        "const backendRoute = (__VU + __ITER) % 2 === 0;",
        "const route = backendRoute ? 'backend' : 'frontend';",
        "const path = backendRoute ? '/api/load-contract' : '/load-contract';",
        "const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';",
        "const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });",
    ] {
        assert!(
            contains_active_binding_line(&script, binding),
            "routed load contract must keep active route/path/body/tag binding {binding}"
        );
    }
}

#[test]
fn reduced_workload_shape_must_not_retain_commercial_latency_evidence() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");
    let reduced = script
        .replace("vus: 4,", "vus: 1,")
        .replace("iterations: 400,", "iterations: 396,");

    assert!(
        !current_oracle_accepts_without_workload_shape(&reduced),
        "a one-VU / 396-iteration run must not satisfy the declared 4-VU / 400-iteration evidence contract"
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

#[test]
fn commented_route_binding_bait_does_not_satisfy_the_contract() {
    let commented_bait = r#"
// const path = backendRoute ? '/api/load-contract' : '/load-contract';
// const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';
"#;

    assert!(
        !contains_active_binding_line(
            commented_bait,
            "const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';"
        ),
        "commented route/body binding text must not manufacture routed parity evidence"
    );
}
