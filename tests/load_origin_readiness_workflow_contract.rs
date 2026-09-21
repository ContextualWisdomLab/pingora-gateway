//! Regression contract for measured-origin readiness in the load workflow.
//!
//! The gateway health endpoint proves only the proxy process is accepting traffic. It does not
//! prove the measured upstream fixture has bound its socket. The load harness must therefore prove
//! origin readiness directly before starting the gateway candidate or k6 measurement.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn load_job_scripts(source: &str) -> Option<String> {
    let document: Value = serde_yaml::from_str(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;

    Some(
        steps
            .iter()
            .filter_map(|step| step.get("run").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn readiness_contract_accepts(source: &str) -> bool {
    let Some(source) = load_job_scripts(source) else {
        return false;
    };
    let Some(origin_start) = source.find("/tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &")
    else {
        return false;
    };
    let Some(origin_ready) = source.find("http://127.0.0.1:18081/fixture-ready") else {
        return false;
    };
    let Some(origin_liveness) = source.find("kill -0 \"$upstream_pid\"") else {
        return false;
    };
    let Some(gateway_start) =
        source.find("target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml")
    else {
        return false;
    };
    let Some(measured_traffic) = source.find("GATEWAY_URL=http://127.0.0.1:18080 k6 run") else {
        return false;
    };

    origin_start < origin_ready
        && origin_ready < origin_liveness
        && origin_liveness < gateway_start
        && gateway_start < measured_traffic
}

#[test]
fn load_contract_proves_origin_readiness_before_gateway_measurement() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");

    assert!(
        readiness_contract_accepts(&source),
        "origin readiness and liveness must be established inside the measured load job before gateway startup and measured traffic"
    );
}

#[test]
fn unrelated_job_decoy_must_not_manufacture_readiness_order_evidence() {
    let source = r#"
jobs:
  unrelated:
    steps:
      - run: |
          /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
          curl http://127.0.0.1:18081/fixture-ready
          kill -0 "$upstream_pid"
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
  load-contract:
    steps:
      - run: |
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
"#;

    assert!(
        !readiness_contract_accepts(source),
        "readiness strings in another job must not let the measured load job omit its own origin-readiness proof"
    );
}
