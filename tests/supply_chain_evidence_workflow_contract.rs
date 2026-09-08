//! Regression contract for preserving the primary Supply Chain failure in GitHub Actions.
//!
//! Exact candidate binding and the promotion-shaped artifact are success-path gates. A failed
//! audit/build/SBOM/scan path may publish separately named diagnostics, but must not manufacture
//! or partially publish the exact candidate-evidence artifact used by successful acceptance.

use serde_yaml::Value;
use std::fs;

const WORKFLOW: &str = ".github/workflows/supply-chain.yml";
const JOB: &str = "candidate-evidence";
const BIND_STEP: &str = "Bind candidate evidence to exact source";
const EXACT_UPLOAD_STEP: &str = "Upload exact candidate evidence";
const FAILURE_STAGE_STEP: &str = "Stage candidate failure diagnostics";
const FAILURE_UPLOAD_STEP: &str = "Upload candidate failure diagnostics";

fn candidate_steps() -> Vec<Value> {
    let source =
        fs::read_to_string(WORKFLOW).expect("Supply Chain workflow should be readable UTF-8");
    let document: Value = serde_yaml::from_str(&source)
        .expect("Supply Chain workflow YAML should parse before validation");
    document
        .get("jobs")
        .and_then(|jobs| jobs.get(JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .expect("candidate-evidence job should define steps")
        .clone()
}

fn named_step<'a>(steps: &'a [Value], name: &str) -> (usize, &'a Value) {
    steps
        .iter()
        .enumerate()
        .find(|(_, step)| step.get("name").and_then(Value::as_str) == Some(name))
        .unwrap_or_else(|| panic!("candidate-evidence should define step {name:?}"))
}

#[test]
fn exact_candidate_artifact_is_success_only_and_fully_bound() {
    let steps = candidate_steps();
    let (bind_index, bind) = named_step(&steps, BIND_STEP);
    let (upload_index, upload) = named_step(&steps, EXACT_UPLOAD_STEP);

    assert!(
        bind_index < upload_index,
        "exact binding must precede exact upload"
    );
    assert!(
        bind.get("if").is_none() && upload.get("if").is_none(),
        "binding and the promotion-shaped candidate artifact must remain success-only"
    );
    assert_eq!(
        upload
            .get("with")
            .and_then(|with| with.get("if-no-files-found"))
            .and_then(Value::as_str),
        Some("error"),
        "a successful exact candidate path must fail if its bound evidence disappears"
    );

    let command = bind
        .get("run")
        .and_then(Value::as_str)
        .expect("exact evidence binding should be a shell step");
    for evidence in [
        "Cargo.lock",
        "deny.toml",
        "candidate.spdx.json",
        "trivy-image.json",
        "trivy-pg-erd-image.json",
        "candidate-evidence.txt",
    ] {
        assert!(
            command.contains(evidence),
            "exact binding should account for {evidence}"
        );
    }
}

#[test]
fn failed_supply_chain_path_uses_distinct_best_effort_diagnostics() {
    let steps = candidate_steps();
    let (stage_index, stage) = named_step(&steps, FAILURE_STAGE_STEP);
    let (upload_index, upload) = named_step(&steps, FAILURE_UPLOAD_STEP);

    assert!(
        stage_index < upload_index,
        "failure diagnostics must be staged before upload"
    );
    assert_eq!(
        stage.get("if").and_then(Value::as_str),
        Some("${{ failure() }}"),
        "failure diagnostics should not run on a successful candidate path"
    );
    let stage_command = stage
        .get("run")
        .and_then(Value::as_str)
        .expect("failure diagnostic staging should be a shell step");
    assert!(
        stage_command.contains("candidate-failure-context.txt")
            && stage_command.contains("source_sha="),
        "failure diagnostics must bind the expected source SHA without requiring success artifacts"
    );

    assert_eq!(
        upload.get("if").and_then(Value::as_str),
        Some("${{ failure() }}"),
        "failure diagnostics must be distinct from the exact success artifact"
    );
    assert_eq!(
        upload
            .get("with")
            .and_then(|with| with.get("if-no-files-found"))
            .and_then(Value::as_str),
        Some("ignore"),
        "diagnostic upload must not replace the primary failure if staging itself cannot produce files"
    );
}
