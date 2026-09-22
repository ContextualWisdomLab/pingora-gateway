//! Buyer-visible routed latency must include client-side connection and blocking time.
//!
//! k6 `http_req_duration` excludes initial connection/DNS time, so it cannot by itself prove the
//! requested end-to-end p95 budget. The routed script therefore records wall-clock time around the
//! synchronous HTTP call in a custom Trend and applies the same p95 budget globally and per route.

use std::fs;

const ROUTED_SCRIPT: &str = "tests/load/pg_erd_gateway_smoke.js";
const TREND_IMPORT: &str = "import { Trend } from 'k6/metrics';";
const TREND_DECLARATION: &str =
    "const endToEndDuration = new Trend('pg_erd_end_to_end_duration', true);";
const AGGREGATE_THRESHOLD: &str = "pg_erd_end_to_end_duration: ['p(95)<20']";
const BACKEND_THRESHOLD: &str =
    "'pg_erd_end_to_end_duration{route:backend}': ['p(95)<20']";
const FRONTEND_THRESHOLD: &str =
    "'pg_erd_end_to_end_duration{route:frontend}': ['p(95)<20']";
const START_TIMER: &str = "const startedAt = Date.now();";
const HTTP_CALL: &str = "const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });";
const RECORD_TIMER: &str = "endToEndDuration.add(Date.now() - startedAt, { route });";

fn exactly_once(source: &str, needle: &str) -> Option<usize> {
    let mut matches = source.match_indices(needle);
    let first = matches.next()?.0;
    matches.next().is_none().then_some(first)
}

fn routed_wall_clock_latency_is_gated(source: &str) -> bool {
    let Some(import) = exactly_once(source, TREND_IMPORT) else {
        return false;
    };
    let Some(declaration) = exactly_once(source, TREND_DECLARATION) else {
        return false;
    };
    let Some(aggregate) = exactly_once(source, AGGREGATE_THRESHOLD) else {
        return false;
    };
    let Some(backend) = exactly_once(source, BACKEND_THRESHOLD) else {
        return false;
    };
    let Some(frontend) = exactly_once(source, FRONTEND_THRESHOLD) else {
        return false;
    };
    let Some(start) = exactly_once(source, START_TIMER) else {
        return false;
    };
    let Some(request) = exactly_once(source, HTTP_CALL) else {
        return false;
    };
    let Some(record) = exactly_once(source, RECORD_TIMER) else {
        return false;
    };

    import < declaration
        && declaration < aggregate
        && aggregate < backend
        && backend < frontend
        && frontend < start
        && start < request
        && request < record
}

#[test]
fn live_routed_script_gates_wall_clock_latency_globally_and_per_route() {
    let source = fs::read_to_string(ROUTED_SCRIPT).expect("routed k6 script should be readable");
    assert!(
        routed_wall_clock_latency_is_gated(&source),
        "routed buyer evidence must gate wall-clock latency around the HTTP call, not only http_req_duration"
    );
}

#[test]
fn http_req_duration_only_is_not_end_to_end_latency_evidence() {
    let source = r#"
import http from 'k6/http';
export const options = {
  thresholds: {
    http_req_duration: ['p(95)<20'],
    'http_req_duration{route:backend}': ['p(95)<20'],
    'http_req_duration{route:frontend}': ['p(95)<20'],
  },
};
export default function () {
  const route = 'backend';
  const gatewayUrl = 'http://127.0.0.1:18180';
  const path = '/api/load-contract';
  const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });
}
"#;
    assert!(!routed_wall_clock_latency_is_gated(source));
}

#[test]
fn aggregate_only_wall_clock_gate_is_not_route_complete() {
    let source = r#"
import http from 'k6/http';
import { Trend } from 'k6/metrics';
const endToEndDuration = new Trend('pg_erd_end_to_end_duration', true);
export const options = {
  thresholds: {
    pg_erd_end_to_end_duration: ['p(95)<20'],
  },
};
export default function () {
  const route = 'backend';
  const gatewayUrl = 'http://127.0.0.1:18180';
  const path = '/api/load-contract';
  const startedAt = Date.now();
  const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });
  endToEndDuration.add(Date.now() - startedAt, { route });
}
"#;
    assert!(!routed_wall_clock_latency_is_gated(source));
}
