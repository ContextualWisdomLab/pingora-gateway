//! Regression contract for preserving the primary Supply Chain failure in GitHub Actions.
//!
//! Exact candidate binding is a success-path gate: it may require every SBOM/scan/image
//! receipt only after earlier audit/build/scan steps succeed. The always-run upload must not
//! replace an earlier causal failure merely because those success artifacts do not exist.

use serde_yaml::Value;
use std::fs;

const WORKFLOW: &str = ".github/workflows/supply-chain.yml";
const JOB: &str = "candidate-evidence";
const BIND_STEP: &str = "Bind candidate evidence to exact source";
const UPLOAD_STEP: &str = "Upload exact candidate evidence";

fn candidate_steps() -> Vec<Value> {
    let source = fs::read_to_string(WORKFLOW).expect("Supply Chain workflow should be readable UTF-8");
    let document: Value =
        serde_yaml::from_str(&source).expect("Supply Chain workflow YAML should parse before validation");
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
fn exact_binding_remains_a_success_path_gate_before_upload() {
    let steps = candidate_steps();
    let (bind_index, bind) = named_step(&steps, BIND_STEP);
    let (upload_index, _) = named_step(&steps, UPLOAD_STEP);

    assert!(
        bind_index < upload_index,
        "exact evidence binding must complete before the artifact upload"
    );
    assert!(
        bind.get("if").is_none(),
        "exact evidence binding must retain the default success-only condition so an earlier failure remains causal"
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
            "exact evidence binding should account for {evidence}"
        );
    }
}

#[test]
fn earlier_supply_chain_failure_is_not_replaced_by_missing_artifact_failure() {
    let steps = candidate_steps();
    let (_, upload) = named_step(&steps, UPLOAD_STEP);

    assert_eq!(
        upload.get("if").and_then(Value::as_str),
        Some("${{ always() }}"),
        "available candidate evidence should remain uploadable after an earlier failure"
    );
    assert_eq!(
        upload
            .get("with")
            .and_then(|with| with.get("if-no-files-found"))
            .and_then(Value::as_str),
        Some("ignore"),
        "a failure before evidence generation must not be replaced by a missing-artifact failure"
    );
}
