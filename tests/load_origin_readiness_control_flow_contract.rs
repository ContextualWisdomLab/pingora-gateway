//! Fail-closed control-flow contract for measured-origin readiness evidence.
//!
//! The existing workflow contract binds evidence to one unconditional `load-contract` step and
//! filters comments, quoted data, heredocs, and multiline string payloads. This companion contract
//! closes a different shell-semantic gap: canonical command text must not earn readiness credit
//! when it appears only inside an outer branch or other non-executed control-flow container.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn load_job_scripts(source: &str) -> Option<Vec<String>> {
    let document: Value = serde_yaml::from_str(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;

    Some(
        steps
            .iter()
            .filter(|step| step.get("if").is_none())
            .filter_map(|step| step.get("run").and_then(Value::as_str))
            .map(str::to_string)
            .collect(),
    )
}

fn visible_lines(script: &str) -> Vec<String> {
    script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn script_proves_readiness(source: &str) -> bool {
    let lines = visible_lines(source);
    let position = |predicate: &dyn Fn(&str) -> bool| lines.iter().position(|line| predicate(line));

    let Some(origin_start) = position(&|line| {
        line == "/tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &"
    }) else {
        return false;
    };
    let Some(origin_ready) = position(&|line| {
        line.starts_with("if curl ")
            && line.contains("http://127.0.0.1:18081/fixture-ready")
            && line.ends_with("; then")
    }) else {
        return false;
    };
    let Some(origin_liveness) = position(&|line| {
        line == "if ! kill -0 \"$upstream_pid\" 2>/dev/null; then"
    }) else {
        return false;
    };
    let Some(gateway_start) = position(&|line| {
        line.starts_with("target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml")
    }) else {
        return false;
    };
    let Some(measured_traffic) = position(&|line| {
        line.starts_with("GATEWAY_URL=http://127.0.0.1:18080 k6 run")
    }) else {
        return false;
    };

    origin_start < origin_ready
        && origin_ready < origin_liveness
        && origin_liveness < gateway_start
        && gateway_start < measured_traffic
}

fn readiness_contract_accepts(source: &str) -> bool {
    load_job_scripts(source)
        .is_some_and(|scripts| scripts.iter().any(|script| script_proves_readiness(script)))
}

#[test]
fn live_workflow_has_control_flow_safe_origin_readiness_evidence() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        readiness_contract_accepts(&source),
        "load-contract must retain executable origin readiness evidence"
    );
}

#[test]
fn outer_false_branch_must_not_manufacture_readiness_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          if false; then
            /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
            for _ in $(seq 1 80); do
              if curl --fail http://127.0.0.1:18081/fixture-ready; then
                break
              fi
              if ! kill -0 "$upstream_pid" 2>/dev/null; then
                exit 1
              fi
            done
            target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
            GATEWAY_URL=http://127.0.0.1:18080 k6 run
          fi
"#;

    assert!(
        !readiness_contract_accepts(source),
        "canonical readiness text inside an outer false branch is not executable evidence"
    );
}

#[test]
fn uncalled_function_must_not_manufacture_readiness_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          archived_readiness() {
            /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
            for _ in $(seq 1 80); do
              if curl --fail http://127.0.0.1:18081/fixture-ready; then
                break
              fi
              if ! kill -0 "$upstream_pid" 2>/dev/null; then
                exit 1
              fi
            done
            target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
            GATEWAY_URL=http://127.0.0.1:18080 k6 run
          }
          echo archived-only
"#;

    assert!(
        !readiness_contract_accepts(source),
        "an uncalled shell function is data-like archived behavior, not executed readiness evidence"
    );
}
