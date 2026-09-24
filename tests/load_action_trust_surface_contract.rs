//! Fail-closed contract for the `load-contract` action trust surface.
//!
//! A pinned set of known actions is insufficient if an additional `uses:` step can be inserted into
//! the same runner. Repository-external action code can invoke passwordless sudo and mutate the
//! provenance utilities before routed measurement. The load job therefore admits exactly the
//! reviewed checkout and two artifact-upload action invocations, each pinned to its canonical SHA.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const CHECKOUT_STEP: &str = "Checkout exact revision";
const CHECKOUT_ACTION: &str = "actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8";
const LOOPBACK_UPLOAD_STEP: &str = "Upload loopback latency evidence";
const ROUTED_UPLOAD_STEP: &str = "Upload routed pg-erd exact-SHA load evidence";
const UPLOAD_ACTION: &str = "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";

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

    let observed = steps
        .iter()
        .filter_map(|step| {
            let uses = step.get("uses").and_then(Value::as_str)?;
            let name = step.get("name").and_then(Value::as_str).unwrap_or("");
            Some((name, uses))
        })
        .collect::<Vec<_>>();

    observed
        == [
            (CHECKOUT_STEP, CHECKOUT_ACTION),
            (LOOPBACK_UPLOAD_STEP, UPLOAD_ACTION),
            (ROUTED_UPLOAD_STEP, UPLOAD_ACTION),
        ]
}

#[test]
fn live_load_job_action_surface_is_canonical() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_action_surface_is_canonical(&source),
        "load-contract must execute only the reviewed exact-SHA checkout and artifact actions"
    );
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

#[test]
fn changed_checkout_action_sha_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        uses: actions/checkout@1111111111111111111111111111111111111111
      - name: Upload loopback latency evidence
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
      - name: Upload routed pg-erd exact-SHA load evidence
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
"#;

    assert!(!load_action_surface_is_canonical(source));
}

#[test]
fn canonical_action_surface_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8
      - name: Upload loopback latency evidence
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
      - name: Upload routed pg-erd exact-SHA load evidence
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
"#;

    assert!(load_action_surface_is_canonical(source));
}
