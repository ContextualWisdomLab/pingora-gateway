//! Fail-closed contract for the routed-load job execution substrate.
//!
//! Exact-head evidence is attributable to the reviewed GitHub-hosted runner only when the
//! `load-contract` job stays on the canonical runner class. Job containers execute ordinary
//! `run` steps inside the selected image and can therefore replace the filesystem, toolchain,
//! command namespace, and standard paths underneath otherwise unchanged evidence scripts.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const CANONICAL_RUNNER: &str = "ubuntu-24.04";

fn load_runner_environment_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };

    job.get("runs-on").and_then(Value::as_str) == Some(CANONICAL_RUNNER)
}

#[test]
fn live_routed_load_uses_the_canonical_runner_environment() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_runner_environment_is_canonical(&source),
        "routed evidence must execute on the reviewed GitHub-hosted runner environment"
    );
}

#[test]
fn self_hosted_runner_must_not_claim_canonical_routed_evidence() {
    let source = r#"
jobs:
  load-contract:
    runs-on: self-hosted
    steps: []
"#;

    assert!(
        !load_runner_environment_is_canonical(source),
        "self-hosted runner state is outside this evidence contract"
    );
}

#[test]
fn job_container_must_not_claim_canonical_routed_evidence() {
    let source = r#"
jobs:
  load-contract:
    runs-on: ubuntu-24.04
    container:
      image: attacker.example/evidence-environment:latest
    steps: []
"#;

    assert!(
        !load_runner_environment_is_canonical(source),
        "a job container can replace the command and filesystem authority beneath routed evidence"
    );
}
