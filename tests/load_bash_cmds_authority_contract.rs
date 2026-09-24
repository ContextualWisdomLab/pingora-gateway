//! Fail-closed contract for Bash's special command-hash array in routed-load evidence.
//!
//! `BASH_CMDS` mirrors Bash's internal command hash table. Assigning an element such as
//! `BASH_CMDS[cargo]=/bin/true` binds that command name without changing `PATH` or invoking the
//! `hash` builtin. The routed evidence shell therefore locks the special array before any external
//! command can run, while Bash remains free to populate the internal table during normal command
//! lookup.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SET: &str = "set -euo pipefail";
const CANONICAL_EXPECTED_SHA: &str = "readonly EXPECTED_SHA";
const CANONICAL_BASH_CMDS_LOCK: &str = "readonly BASH_CMDS";

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn routed_run(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut routed = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let step = routed.next()?;
    if routed.next().is_some() {
        return None;
    }
    step.get("run")?.as_str().map(str::to_owned)
}

fn routed_shell_locks_bash_command_hash_authority(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines = active_lines(&run);
    if lines.len() < 3
        || lines[0] != CANONICAL_SET
        || lines[1] != CANONICAL_EXPECTED_SHA
        || lines[2] != CANONICAL_BASH_CMDS_LOCK
    {
        return false;
    }

    !lines[..2]
        .iter()
        .any(|line| line.contains("BASH_CMDS[") || line.contains("BASH_CMDS="))
}

#[test]
fn live_routed_shell_locks_bash_command_hash_authority_before_external_commands() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_shell_locks_bash_command_hash_authority(&source),
        "routed evidence must make BASH_CMDS readonly before any external command can alter or consume command-hash authority"
    );
}

#[test]
fn process_local_bash_cmds_rebinding_before_lock_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly EXPECTED_SHA
          BASH_CMDS[cargo]=/bin/true
          readonly BASH_CMDS
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_shell_locks_bash_command_hash_authority(source));
}

#[test]
fn canonical_early_bash_cmds_lock_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly EXPECTED_SHA
          readonly BASH_CMDS
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(routed_shell_locks_bash_command_hash_authority(source));
}
