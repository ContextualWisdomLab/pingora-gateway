use std::fs;

fn has_odd_unescaped_backtick_count(line: &str) -> bool {
    let mut escaped = false;
    let mut count = 0usize;

    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '`' {
            count += 1;
        }
    }

    count % 2 == 1
}

fn active_lines(script: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut in_block_comment = false;
    let mut in_multiline_template = false;

    for raw_line in script.lines() {
        let line = raw_line.trim();

        if in_block_comment {
            if line.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if in_multiline_template {
            if has_odd_unescaped_backtick_count(raw_line) {
                in_multiline_template = false;
            }
            continue;
        }
        if let Some(comment_start) = line.find("/*") {
            if !line[comment_start + 2..].contains("*/") {
                in_block_comment = true;
            }
            continue;
        }
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        lines.push(line);
        if has_odd_unescaped_backtick_count(raw_line) {
            in_multiline_template = true;
        }
    }

    lines
}

fn options_block_matches_canonical_contract(script: &str) -> bool {
    const EXPECTED: [&str; 13] = [
        "export const options = {",
        "vus: 4,",
        "iterations: 400,",
        "thresholds: {",
        "checks: ['rate==1'],",
        "http_req_failed: ['rate==0'],",
        "http_req_duration: ['p(95)<20'],",
        "'http_req_duration{route:backend}': ['p(95)<20'],",
        "'http_req_duration{route:frontend}': ['p(95)<20'],",
        "'http_reqs{route:backend}': ['count>=198'],",
        "'http_reqs{route:frontend}': ['count>=198'],",
        "},",
        "};",
    ];

    let lines = active_lines(script);
    let Some(start) = lines
        .iter()
        .position(|line| *line == "export const options = {")
    else {
        return false;
    };
    let Some(candidate) = lines.get(start..start + EXPECTED.len()) else {
        return false;
    };

    candidate == EXPECTED
}

fn contains_active_binding_line(script: &str, expected: &str) -> bool {
    active_lines(script).into_iter().any(|line| line == expected)
}

fn default_function_matches_canonical_body(script: &str) -> bool {
    const EXPECTED: [&str; 10] = [
        "export default function () {",
        "const backendRoute = (__VU + __ITER) % 2 === 0;",
        "const route = backendRoute ? 'backend' : 'frontend';",
        "const path = backendRoute ? '/api/load-contract' : '/load-contract';",
        "const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';",
        "const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });",
        "check(response, {",
        "'pg-erd gateway returns 200': (result) => result.status === 200,",
        "'pg-erd gateway preserves characterized route body': (result) => result.body === expectedBody,",
        "});",
    ];

    let lines = active_lines(script);
    let Some(start) = lines
        .iter()
        .position(|line| *line == "export default function () {")
    else {
        return false;
    };
    let Some(summary_start) = lines
        .iter()
        .skip(start + 1)
        .position(|line| *line == "export function handleSummary(data) {")
        .map(|offset| start + 1 + offset)
    else {
        return false;
    };

    let function_lines = &lines[start..summary_start];
    function_lines.len() == EXPECTED.len() + 1
        && function_lines[..EXPECTED.len()] == EXPECTED
        && function_lines[EXPECTED.len()] == "}"
}

fn routed_evidence_contract_accepts(script: &str) -> bool {
    options_block_matches_canonical_contract(script) && default_function_matches_canonical_body(script)
}

#[test]
fn routed_latency_is_gated_for_each_characterized_route() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");

    assert!(
        routed_evidence_contract_accepts(&script),
        "routed load contract must bind workload shape, failure gates, latency thresholds, route sample floors, and the exact executable request/check body"
    );
}

#[test]
fn reduced_workload_shape_must_not_retain_commercial_latency_evidence() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");
    let reduced = script
        .replace("vus: 4,", "vus: 1,")
        .replace("iterations: 400,", "iterations: 396,");

    assert!(
        !routed_evidence_contract_accepts(&reduced),
        "a one-VU / 396-iteration run must not satisfy the declared 4-VU / 400-iteration evidence contract"
    );
}

#[test]
fn nested_workload_shape_bait_must_not_retain_commercial_latency_evidence() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");
    let baited = script
        .replace(
            "  vus: 4,",
            "  vus: 1,\n  archived_scenario: {\n    vus: 4,\n    iterations: 400,\n  },",
        )
        .replace("  iterations: 400,\n", "");

    assert_ne!(baited, script, "nested workload-shape mutation must apply");
    assert!(
        !routed_evidence_contract_accepts(&baited),
        "nested object entries must not manufacture top-level 4-VU / 400-iteration evidence"
    );
}

#[test]
fn disabled_check_failure_gate_must_not_retain_body_parity_evidence() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");
    let ungated = script.replace("checks: ['rate==1'],", "// checks: ['rate==1'],");

    assert!(
        !routed_evidence_contract_accepts(&ungated),
        "status/body check failures must remain exit-failing k6 evidence rather than advisory checks"
    );
}

#[test]
fn weakened_status_and_body_predicates_must_not_retain_parity_evidence() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");
    let weakened = script
        .replace(
            "'pg-erd gateway returns 200': (result) => result.status === 200,",
            "'pg-erd gateway returns 200': (_result) => true,",
        )
        .replace(
            "'pg-erd gateway preserves characterized route body': (result) => result.body === expectedBody,",
            "'pg-erd gateway preserves characterized route body': (_result) => true,",
        );

    assert!(
        !routed_evidence_contract_accepts(&weakened),
        "the checks threshold must not manufacture status/body parity when the runtime predicates are weakened"
    );
}

#[test]
fn dead_code_parity_block_must_not_retain_evidence() {
    let script = fs::read_to_string("tests/load/pg_erd_gateway_smoke.js")
        .expect("pg-erd routed load script must be readable");
    let canonical = r#"  check(response, {
    'pg-erd gateway returns 200': (result) => result.status === 200,
    'pg-erd gateway preserves characterized route body': (result) => result.body === expectedBody,
  });"#;
    let dead_coded = r#"  if (false) {
    check(response, {
      'pg-erd gateway returns 200': (result) => result.status === 200,
      'pg-erd gateway preserves characterized route body': (result) => result.body === expectedBody,
    });
  }
  check(response, {
    'pg-erd gateway returns 200': (_result) => true,
    'pg-erd gateway preserves characterized route body': (_result) => true,
  });"#;
    let weakened = script.replace(canonical, dead_coded);

    assert_ne!(weakened, script, "dead-code mutation must apply to the canonical check block");
    assert!(
        !routed_evidence_contract_accepts(&weakened),
        "dead-code exact predicates must not manufacture parity evidence while permissive runtime checks execute"
    );
}

#[test]
fn template_literal_bait_must_not_retain_executable_body_evidence() {
    let baited = r#"
const archived_contract = `
export default function () {
const backendRoute = (__VU + __ITER) % 2 === 0;
const route = backendRoute ? 'backend' : 'frontend';
const path = backendRoute ? '/api/load-contract' : '/load-contract';
const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';
const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });
check(response, {
'pg-erd gateway returns 200': (result) => result.status === 200,
'pg-erd gateway preserves characterized route body': (result) => result.body === expectedBody,
});
}
export function handleSummary(data) {
`;
export default function () {
  check(response, {
    'pg-erd gateway returns 200': (_result) => true,
    'pg-erd gateway preserves characterized route body': (_result) => true,
  });
}
export function handleSummary(data) {
}
"#;

    assert!(
        !default_function_matches_canonical_body(baited),
        "canonical text inside a non-executed multiline template literal must not manufacture executable request/body parity evidence"
    );
}

#[test]
fn inline_block_comment_bait_must_not_retain_executable_body_evidence() {
    let baited = r#"
const archive_marker = true; /*
export default function () {
const backendRoute = (__VU + __ITER) % 2 === 0;
const route = backendRoute ? 'backend' : 'frontend';
const path = backendRoute ? '/api/load-contract' : '/load-contract';
const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';
const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });
check(response, {
'pg-erd gateway returns 200': (result) => result.status === 200,
'pg-erd gateway preserves characterized route body': (result) => result.body === expectedBody,
});
}
export function handleSummary(data) {
*/
export default function () {
  check(response, {
    'pg-erd gateway returns 200': (_result) => true,
    'pg-erd gateway preserves characterized route body': (_result) => true,
  });
}
export function handleSummary(data) {
}
"#;

    assert!(
        !default_function_matches_canonical_body(baited),
        "canonical text inside a block comment opened after executable code must not manufacture request/body parity evidence"
    );
}

#[test]
fn commented_threshold_bait_does_not_satisfy_the_contract() {
    let commented_bait = r#"
export const options = {
  vus: 4,
  iterations: 400,
  thresholds: {
    // 'http_req_duration{route:backend}': ['p(95)<20'],
  },
};
"#;

    assert!(
        !options_block_matches_canonical_contract(commented_bait),
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
