//! Fail-closed contract for Bash startup authority in routed load evidence.
//!
//! GitHub Actions executes an explicit `shell: bash` step as non-interactive Bash. GNU Bash reads
//! `BASH_ENV` before the step script, imports options from `BASHOPTS` and `SHELLOPTS` when present in
//! its startup environment, and enters POSIX mode when `POSIXLY_CORRECT` is inherited. Those inputs
//! can change parsing and command resolution before any reviewed run line executes, so routed
//! evidence rejects them at workflow/job/step scope and in active scripts that could persist them.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const BASH_STARTUP_AUTHORITY: [&str; 4] = [
    "BASH_ENV",
    "BASHOPTS",
    "SHELLOPTS",
    "POSIXLY_CORRECT",
];

fn env_sets_bash_startup_authority(node: &Value) -> bool {
    node.get("env")
        .and_then(Value::as_mapping)
        .is_some_and(|env| {
            env.keys().filter_map(Value::as_str).any(|key| {
                BASH_STARTUP_AUTHORITY
                    .iter()
                    .any(|forbidden| key == *forbidden)
            })
        })
}

fn active_run_mentions_bash_startup_authority(run: &str) -> bool {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with('#'))
        .any(|line| {
            BASH_STARTUP_AUTHORITY
                .iter()
                .any(|forbidden| line.contains(forbidden))
        })
}

fn load_job_has_closed_bash_startup_authority(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if env_sets_bash_startup_authority(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if env_sets_bash_startup_authority(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };

    !steps.iter().any(|step| {
        env_sets_bash_startup_authority(step)
            || step
                .get("run")
                .and_then(Value::as_str)
                .is_some_and(active_run_mentions_bash_startup_authority)
    })
}

#[test]
fn live_load_job_has_no_bash_startup_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_job_has_closed_bash_startup_authority(&source),
        "load-contract must not allow environment-controlled Bash startup code or option authority"
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
    assert!(!load_job_has_closed_bash_startup_authority(source));
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
    assert!(!load_job_has_closed_bash_startup_authority(source));
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
    assert!(!load_job_has_closed_bash_startup_authority(source));
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
    assert!(!load_job_has_closed_bash_startup_authority(source));
}

#[test]
fn inherited_bashopts_must_not_claim_release_evidence() {
    let source = r#"
env:
  BASHOPTS: expand_aliases
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: echo measured
"#;
    assert!(
        !load_job_has_closed_bash_startup_authority(source),
        "BASHOPTS in Bash's startup environment can enable shell options before the evidence script runs"
    );
}

#[test]
fn inherited_shellopts_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    env:
      SHELLOPTS: posix
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: echo measured
"#;
    assert!(!load_job_has_closed_bash_startup_authority(source));
}

#[test]
fn inherited_posix_mode_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        env:
          POSIXLY_CORRECT: "1"
        run: echo measured
"#;
    assert!(!load_job_has_closed_bash_startup_authority(source));
}

#[test]
fn github_env_persistence_of_shell_options_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Poison later Bash options
        run: echo "BASHOPTS=expand_aliases" >> "$GITHUB_ENV"
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: echo measured
"#;
    assert!(!load_job_has_closed_bash_startup_authority(source));
}

#[test]
fn comment_only_startup_authority_reference_is_not_execution() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          # BASH_ENV, BASHOPTS, SHELLOPTS and POSIXLY_CORRECT are forbidden in executable evidence.
          echo measured
"#;
    assert!(load_job_has_closed_bash_startup_authority(source));
}

#[test]
fn inherited_exported_function_must_not_claim_release_evidence() {
    let source = r#"
env:
  BASH_FUNC_cargo%%: "() { :; }"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(
        !load_job_has_closed_bash_startup_authority(source),
        "Bash imports exported functions from its startup environment before the reviewed routed script executes"
    );
}
