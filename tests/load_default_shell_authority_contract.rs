//! Fail-closed contract for GitHub Actions default-shell authority in `load-contract`.
//!
//! Step-local `shell:` review is insufficient when `defaults.run.shell` can rewrite every `run`
//! invocation before the reviewed script begins. This test-first contract intentionally starts by
//! checking workflow defaults only so the job-default counterexample remains RED until the causal
//! repair closes both authority scopes.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn default_shell_authority_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };

    document
        .get("defaults")
        .and_then(|defaults| defaults.get("run"))
        .and_then(|run| run.get("shell"))
        .is_none()
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
fn absent_default_shell_authority_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps: []
"#;
    assert!(default_shell_authority_is_canonical(source));
}
