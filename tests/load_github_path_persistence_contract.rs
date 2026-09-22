//! Fail-closed contract for persisted PATH mutation in routed-load evidence.
//!
//! GitHub Actions applies entries written to `GITHUB_PATH` to the `PATH` of subsequent steps.
//! The routed receipt resolves provenance utilities such as `sha256sum`, `tar`, and `cmp` by bare
//! command name, so an earlier load-job step must not be able to prepend an unreviewed shim
//! directory before the measured step.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn env_overrides_path(node: &Value) -> bool {
    node.get("env")
        .and_then(Value::as_mapping)
        .is_some_and(|env| env.keys().filter_map(Value::as_str).any(|key| key == "PATH"))
}

fn load_job_has_closed_persisted_path(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if env_overrides_path(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if env_overrides_path(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };

    !steps.iter().any(env_overrides_path)
}

#[test]
fn live_load_job_does_not_persist_an_unreviewed_path() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_job_has_closed_persisted_path(&source),
        "load-contract must not allow PATH mutation before routed evidence"
    );
}

#[test]
fn github_path_persistence_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Poison later command resolution
        run: echo "/tmp/evidence-shims" >> "$GITHUB_PATH"
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !load_job_has_closed_persisted_path(source),
        "GITHUB_PATH can retarget bare provenance utilities in later steps without a PATH key or same-shell PATH assignment"
    );
}

#[test]
fn workflow_level_path_override_must_not_claim_release_evidence() {
    let source = r#"
env:
  PATH: /tmp/evidence-shims:/usr/bin
jobs:
  load-contract:
    steps:
      - run: echo measured
"#;
    assert!(!load_job_has_closed_persisted_path(source));
}

#[test]
fn step_level_path_override_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - env:
          PATH: /tmp/evidence-shims:/usr/bin
        run: echo measured
"#;
    assert!(!load_job_has_closed_persisted_path(source));
}
