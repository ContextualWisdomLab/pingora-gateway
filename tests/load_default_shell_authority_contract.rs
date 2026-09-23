//! Fail-closed contract for GitHub Actions default-shell authority in `load-contract`.
//!
//! Step-local `shell:` review is insufficient when `defaults.run.shell` can rewrite every `run`
//! invocation before the reviewed script begins. The load evidence contract therefore rejects both
//! workflow-wide and load-job default shell authority; explicit reviewed step shells remain governed
//! by `load_shell_trust_surface_contract`.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn run_defaults_define_shell(defaults: Option<&Value>) -> bool {
    defaults
        .and_then(|defaults| defaults.get("run"))
        .and_then(|run| run.get("shell"))
        .is_some()
}

fn default_shell_authority_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };

    let workflow_shell = run_defaults_define_shell(document.get("defaults"));
    let load_job_shell = document
        .get("jobs")
        .and_then(|jobs| jobs.get(LOAD_JOB))
        .is_some_and(|job| run_defaults_define_shell(job.get("defaults")));

    !workflow_shell && !load_job_shell
}

#[test]
fn live_workflow_has_no_default_shell_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(default_shell_authority_is_canonical(&source));
}

#[test]
fn workflow_default_shell_must_not_claim_release_evidence() {
    let source = r#"
defaults:
  run:
    shell: bash -c 'true' -- {0}
jobs:
  load-contract:
    steps: []
"#;

    assert!(!default_shell_authority_is_canonical(source));
}

#[test]
fn job_default_shell_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    defaults:
      run:
        shell: bash -c 'true' -- {0}
    steps: []
"#;

    assert!(
        !default_shell_authority_is_canonical(source),
        "jobs.<job_id>.defaults.run.shell can bypass exact-source and summary run steps before their script body executes"
    );
}

#[test]
fn unrelated_job_default_shell_does_not_change_load_authority() {
    let source = r#"
jobs:
  docs:
    defaults:
      run:
        shell: python
    steps: []
  load-contract:
    steps: []
"#;

    assert!(default_shell_authority_is_canonical(source));
}

#[test]
fn absent_default_shell_authority_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps: []
"#;
    assert!(default_shell_authority_is_canonical(source));
}
