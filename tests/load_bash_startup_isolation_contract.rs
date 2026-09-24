//! Fail-closed contract for pre-script Bash startup isolation in routed load evidence.
//!
//! `shell: bash` maps to `bash --noprofile --norc -eo pipefail {0}`, but those flags do not by
//! themselves neutralize startup environment authority. GNU Bash can process `BASH_ENV`, import
//! exported functions, honor inherited `SHELLOPTS`/`BASHOPTS`, and enter POSIX mode when
//! `POSIXLY_CORRECT` is present before the first reviewed run line executes. The routed evidence
//! shell therefore removes `POSIXLY_CORRECT` before Bash starts and uses Bash privileged mode,
//! which ignores `BASH_ENV`/`ENV`, imported shell functions, `SHELLOPTS`, and `BASHOPTS`.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SHELL: &str =
    "/usr/bin/env -u POSIXLY_CORRECT /bin/bash --noprofile --norc -p -eo pipefail {0}";

fn routed_shell(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let shell = matches.next()?.get("shell")?.as_str()?.to_owned();
    matches.next().is_none().then_some(shell)
}

fn routed_bash_startup_is_isolated(source: &str) -> bool {
    routed_shell(source).as_deref() == Some(CANONICAL_SHELL)
}

#[test]
fn live_routed_step_isolates_bash_startup_before_the_script_runs() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_bash_startup_is_isolated(&source),
        "routed evidence must remove POSIXLY_CORRECT before Bash starts and use privileged Bash startup isolation"
    );
}

#[test]
fn plain_bash_does_not_close_pre_script_startup_authority() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: echo measured
"#;
    assert!(!routed_bash_startup_is_isolated(source));
}

#[test]
fn no_profile_and_no_rc_without_privileged_mode_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        shell: /usr/bin/env -u POSIXLY_CORRECT /bin/bash --noprofile --norc -eo pipefail {{0}}\n        run: echo measured\n"
    );
    assert!(!routed_bash_startup_is_isolated(&source));
}

#[test]
fn privileged_bash_without_pre_start_posix_removal_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        shell: /bin/bash --noprofile --norc -p -eo pipefail {{0}}\n        run: echo measured\n"
    );
    assert!(!routed_bash_startup_is_isolated(&source));
}

#[test]
fn exact_pre_start_isolation_shell_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        shell: {CANONICAL_SHELL}\n        run: echo measured\n"
    );
    assert!(routed_bash_startup_is_isolated(&source));
}
