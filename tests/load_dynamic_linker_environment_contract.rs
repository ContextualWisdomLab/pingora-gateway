//! Fail-closed contract for dynamic-loader environment authority in routed load evidence.
//!
//! Exact command text and pinned executable paths are not sufficient if the workflow, job, or
//! evidence-bearing step can inject dynamic-loader controls before those processes start. The
//! glibc loader consumes `LD_*` variables such as `LD_PRELOAD`, `LD_LIBRARY_PATH`, and `LD_AUDIT`,
//! while `GLIBC_TUNABLES` can alter runtime behavior relevant to a latency receipt. These controls
//! must not become an unreviewed authority over exact-head source checks, candidate construction,
//! or the measured process.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const CHECKOUT_STEP: &str = "Checkout exact revision";
const VERIFY_STEP: &str = "Verify checkout identity";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SUMMARY_STEP: &str = "Require routed pg-erd latency summary";

fn env_sets_dynamic_loader_authority(node: &Value) -> bool {
    node.get("env")
        .and_then(Value::as_mapping)
        .is_some_and(|env| {
            env.keys().filter_map(Value::as_str).any(|key| {
                key.starts_with("LD_") || key == "GLIBC_TUNABLES"
            })
        })
}

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn load_evidence_has_closed_dynamic_loader_environment(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if env_sets_dynamic_loader_authority(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if env_sets_dynamic_loader_authority(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };

    for name in [CHECKOUT_STEP, VERIFY_STEP, ROUTED_STEP, SUMMARY_STEP] {
        let Some(step) = unique_named_step(steps, name) else {
            return false;
        };
        if env_sets_dynamic_loader_authority(step) {
            return false;
        }
    }

    true
}

#[test]
fn live_load_evidence_has_no_explicit_dynamic_loader_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_evidence_has_closed_dynamic_loader_environment(&source),
        "exact-head load evidence must not admit explicit dynamic-loader environment authority"
    );
}

#[test]
fn workflow_level_ld_preload_must_not_claim_release_evidence() {
    let source = r#"
env:
  LD_PRELOAD: /tmp/interpose.so
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        run: echo checkout
      - name: Verify checkout identity
        run: echo verify
      - name: Run routed pg-erd loopback traffic
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: echo summary
"#;

    assert!(
        !load_evidence_has_closed_dynamic_loader_environment(source),
        "LD_PRELOAD can interpose code before exact source/build/measurement commands execute"
    );
}

#[test]
fn routed_ld_library_path_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        run: echo checkout
      - name: Verify checkout identity
        run: echo verify
      - name: Run routed pg-erd loopback traffic
        env:
          LD_LIBRARY_PATH: /tmp/alternate-libs
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: echo summary
"#;

    assert!(!load_evidence_has_closed_dynamic_loader_environment(source));
}

#[test]
fn routed_ld_audit_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        run: echo checkout
      - name: Verify checkout identity
        run: echo verify
      - name: Run routed pg-erd loopback traffic
        env:
          LD_AUDIT: /tmp/audit.so
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: echo summary
"#;

    assert!(!load_evidence_has_closed_dynamic_loader_environment(source));
}

#[test]
fn routed_glibc_tunables_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        run: echo checkout
      - name: Verify checkout identity
        run: echo verify
      - name: Run routed pg-erd loopback traffic
        env:
          GLIBC_TUNABLES: glibc.malloc.tcache_count=0
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: echo summary
"#;

    assert!(!load_evidence_has_closed_dynamic_loader_environment(source));
}
