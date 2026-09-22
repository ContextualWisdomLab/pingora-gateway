//! Fail-closed contract for Bash startup-file injection into routed load evidence.
//!
//! GitHub Actions executes an explicit `shell: bash` step as non-interactive Bash. GNU Bash reads
//! the file named by `BASH_ENV` before the step script, even with `--noprofile --norc`. A routed
//! evidence job must therefore reject `BASH_ENV` from workflow/job/step environments and from run
//! scripts that could persist it through `GITHUB_ENV` or export it before a later Bash step.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn env_sets_bash_env(node: &Value) -> bool {
    node.get("env")
        .and_then(Value::as_mapping)
        .is_some_and(|env| env.keys().filter_map(Value::as_str).any(|key| key == "BASH_ENV"))
}

fn active_run_mentions_bash_env(run: &str) -> bool {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with('#'))
        .any(|line| line.contains("BASH_ENV"))
}

fn load_job_has_closed_bash_startup_env(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if env_sets_bash_env(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if env_sets_bash_env(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };

    !steps.iter().any(|step| {
        env_sets_bash_env(step)
            || step
                .get("run")
                .and_then(Value::as_str)
                .is_some_and(active_run_mentions_bash_env)
    })
}

#[test]
fn live_load_job_has_no_bash_startup_hook() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_job_has_closed_bash_startup_env(&source),
        "load-contract must not allow BASH_ENV to run unreviewed startup code before evidence scripts"
    );
}

#[test]
fn workflow_level_bash_env_must_not_claim_release_evidence() {
    let source = r#"
env:
  BASH_ENV: /tmp/provenance-startup.sh
jobs:
  load-contract:
    steps:
      - run: echo measured
"#;
    assert!(!load_job_has_closed_bash_startup_env(source));
}

#[test]
fn job_level_bash_env_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    env:
      BASH_ENV: /tmp/provenance-startup.sh
    steps:
      - run: echo measured
"#;
    assert!(!load_job_has_closed_bash_startup_env(source));
}

#[test]
fn step_level_bash_env_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        env:
          BASH_ENV: /tmp/provenance-startup.sh
        run: echo measured
"#;
    assert!(!load_job_has_closed_bash_startup_env(source));
}

#[test]
fn github_env_persistence_of_bash_env_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Poison later Bash startup
        run: echo "BASH_ENV=/tmp/provenance-startup.sh" >> "$GITHUB_ENV"
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: echo measured
"#;
    assert!(!load_job_has_closed_bash_startup_env(source));
}

#[test]
fn comment_only_bash_env_reference_is_not_execution() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          # BASH_ENV is forbidden in executable load evidence.
          echo measured
"#;
    assert!(load_job_has_closed_bash_startup_env(source));
}
