//! Fail-closed contract for routed-load job dependency authority in GitHub Actions.
//!
//! The routed load job is release evidence only when GitHub schedules it directly. A newly added
//! prerequisite can cause the evidence job to be skipped before any runner-side source, process,
//! or latency contract executes.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const LOAD_JOB_IF: &str = "github.event_name != 'pull_request' || github.event.pull_request.draft == false";

fn load_job_is_directly_scheduled(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };

    job.get("if").and_then(Value::as_str) == Some(LOAD_JOB_IF) && job.get("needs").is_none()
}

#[test]
fn live_workflow_schedules_routed_load_job_directly() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_job_is_directly_scheduled(&source),
        "load-contract must remain directly schedulable from the workflow event"
    );
}

#[test]
fn prerequisite_dependency_must_not_be_able_to_skip_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  prerequisite:
    if: ${{{{ false }}}}
    runs-on: ubuntu-24.04
    steps:
      - run: true
  load-contract:
    if: {LOAD_JOB_IF}
    needs: prerequisite
    runs-on: ubuntu-24.04
    steps:
      - run: true
"#
    );

    assert!(
        !load_job_is_directly_scheduled(&source),
        "jobs.<job_id>.needs can skip load-contract before any routed evidence executes"
    );
}

#[test]
fn canonical_job_without_dependencies_is_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    if: {LOAD_JOB_IF}
    runs-on: ubuntu-24.04
    steps:
      - run: true
"#
    );

    assert!(load_job_is_directly_scheduled(&source));
}
