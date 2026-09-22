//! Test-first contract for the `load-contract` action trust surface.
//!
//! Pinned known actions are not sufficient evidence when the job can also acquire an additional
//! `uses:` step. A newly introduced action executes repository-external code inside the same runner
//! and can invoke passwordless sudo before routed provenance is measured. The initial predicate
//! intentionally checks only that the known actions stay pinned, exposing the extra-action gap.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const CHECKOUT_STEP: &str = "Checkout exact revision";
const CHECKOUT_ACTION: &str = "actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8";
const LOOPBACK_UPLOAD_STEP: &str = "Upload loopback latency evidence";
const ROUTED_UPLOAD_STEP: &str = "Upload routed pg-erd exact-SHA load evidence";
const UPLOAD_ACTION: &str = "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn load_action_surface_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(steps) = document
        .get("jobs")
        .and_then(|jobs| jobs.get(LOAD_JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
    else {
        return false;
    };

    unique_named_step(steps, CHECKOUT_STEP)
        .and_then(|step| step.get("uses"))
        .and_then(Value::as_str)
        == Some(CHECKOUT_ACTION)
        && unique_named_step(steps, LOOPBACK_UPLOAD_STEP)
            .and_then(|step| step.get("uses"))
            .and_then(Value::as_str)
            == Some(UPLOAD_ACTION)
        && unique_named_step(steps, ROUTED_UPLOAD_STEP)
            .and_then(|step| step.get("uses"))
            .and_then(Value::as_str)
            == Some(UPLOAD_ACTION)
}

#[test]
fn live_load_job_keeps_required_actions_pinned() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(load_action_surface_is_canonical(&source));
}

#[test]
fn additional_unreviewed_action_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8
      - name: Replace provenance tools through an action
        uses: attacker/system-tool-replacer@0123456789012345678901234567890123456789
      - name: Upload loopback latency evidence
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
      - name: Upload routed pg-erd exact-SHA load evidence
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
"#;

    assert!(
        !load_action_surface_is_canonical(source),
        "an extra action can execute external code with runner privileges while all known actions remain pinned"
    );
}
