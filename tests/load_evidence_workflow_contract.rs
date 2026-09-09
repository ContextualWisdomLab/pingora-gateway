//! Regression contract for preserving causal load-test evidence in GitHub Actions.
//!
//! A failure before k6 creates `k6-summary.json` must not be replaced by a secondary
//! artifact-upload failure. Conversely, a successful measured path must still require
//! the summary before the always-run evidence upload is admitted.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const LOAD_STEP: &str = "Exercise concurrent loopback traffic contract";
const SUMMARY_STEP: &str = "Require loopback latency summary";
const UPLOAD_STEP: &str = "Upload loopback latency evidence";

/// Parses the repository CI workflow and returns the direct load-contract steps.
fn load_steps() -> Vec<Value> {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    let document: Value =
        serde_yaml::from_str(&source).expect("CI workflow YAML should parse before validation");
    document
        .get("jobs")
        .and_then(|jobs| jobs.get(LOAD_JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .expect("load-contract job should define steps")
        .clone()
}

/// Returns the index and semantic mapping for one named load-contract step.
fn named_step<'a>(steps: &'a [Value], name: &str) -> (usize, &'a Value) {
    steps
        .iter()
        .enumerate()
        .find(|(_, step)| step.get("name").and_then(Value::as_str) == Some(name))
        .unwrap_or_else(|| panic!("load-contract should define step {name:?}"))
}

#[test]
fn successful_load_requires_nonempty_summary_before_always_upload() {
    let steps = load_steps();
    let (load_index, _) = named_step(&steps, LOAD_STEP);
    let (summary_index, summary) = named_step(&steps, SUMMARY_STEP);
    let (upload_index, _) = named_step(&steps, UPLOAD_STEP);

    assert!(
        load_index < summary_index && summary_index < upload_index,
        "summary validation must run after measured traffic and before evidence upload"
    );
    assert_eq!(
        summary.get("run").and_then(Value::as_str),
        Some("test -s k6-summary.json"),
        "a successful measured path must fail causally when its k6 summary is missing or empty"
    );
    assert!(
        summary.get("if").is_none(),
        "summary validation must retain the default success-only step condition"
    );
}

#[test]
fn pre_k6_failure_cannot_be_replaced_by_missing_summary_upload_failure() {
    let steps = load_steps();
    let (_, upload) = named_step(&steps, UPLOAD_STEP);

    assert_eq!(
        upload.get("if").and_then(Value::as_str),
        Some("${{ always() }}"),
        "load evidence upload should remain available after an earlier failure"
    );
    let missing_file_policy = upload
        .get("with")
        .and_then(|with| with.get("if-no-files-found"))
        .and_then(Value::as_str);
    assert_eq!(
        missing_file_policy,
        Some("ignore"),
        "pre-k6 failures must preserve their primary failure instead of failing again because the summary was never created"
    );
}
