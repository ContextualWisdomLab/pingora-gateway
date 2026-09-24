//! Fail-closed contract for Bash compatibility-mode authority before routed evidence scripts run.
//!
//! GNU Bash imports `BASH_COMPAT` as a shell variable and uses its value to select a historical
//! compatibility level. Privileged mode closes startup-file/function/option inheritance, but the
//! manual does not include `BASH_COMPAT` among the variables ignored by `-p`. The routed shell
//! therefore removes both `POSIXLY_CORRECT` and `BASH_COMPAT` before Bash starts.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SHELL: &str = "/usr/bin/env -u POSIXLY_CORRECT -u BASH_COMPAT /bin/bash --noprofile --norc -p -eo pipefail {0}";

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

fn bash_compat_authority_is_removed_before_startup(source: &str) -> bool {
    routed_shell(source).as_deref() == Some(CANONICAL_SHELL)
}

#[test]
fn live_routed_shell_removes_bash_compat_before_startup() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        bash_compat_authority_is_removed_before_startup(&source),
        "routed evidence must remove BASH_COMPAT and POSIXLY_CORRECT before Bash startup"
    );
}

#[test]
fn privileged_mode_without_bash_compat_removal_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        shell: /usr/bin/env -u POSIXLY_CORRECT /bin/bash --noprofile --norc -p -eo pipefail {{0}}\n"
    );
    assert!(!bash_compat_authority_is_removed_before_startup(&source));
}

#[test]
fn in_script_unset_is_too_late() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        shell: /usr/bin/env -u POSIXLY_CORRECT /bin/bash --noprofile --norc -p -eo pipefail {{0}}\n        run: |\n          unset BASH_COMPAT\n          echo measured\n"
    );
    assert!(!bash_compat_authority_is_removed_before_startup(&source));
}

#[test]
fn exact_pre_start_removal_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        shell: {CANONICAL_SHELL}\n        run: echo measured\n"
    );
    assert!(bash_compat_authority_is_removed_before_startup(&source));
}
