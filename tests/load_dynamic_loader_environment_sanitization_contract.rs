//! Fail-closed contract for inherited GNU dynamic-loader authority in routed-load evidence.
//!
//! The routed rebuild executes Cargo, rustc, GCC, make, Perl and the measured candidate as
//! dynamically linked processes. `LD_PRELOAD`, `LD_AUDIT` and `LD_LIBRARY_PATH` can inject or
//! redirect shared objects before source-controlled build inputs are evaluated, while
//! `GLIBC_TUNABLES` can alter GNU C Library runtime behavior. The critical loader inputs must be
//! neutralized before the routed step shell starts and then removed from child-process state.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const FIRST_TOOLCHAIN_ASSERTION: &str = "[[ \"$(rustc --version)\" == rustc\\ 1.98.1\\ * ]]";
const CANONICAL_SANITIZE: &str = "unset LD_PRELOAD LD_LIBRARY_PATH LD_AUDIT GLIBC_TUNABLES";
const CRITICAL_ENVIRONMENT: [&str; 4] = [
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "LD_AUDIT",
    "GLIBC_TUNABLES",
];

fn routed_step(source: &str) -> Option<&Value> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn dynamic_loader_authority_is_neutralized(source: &str) -> bool {
    let Some(step) = routed_step(source) else {
        return false;
    };
    let Some(environment) = step.get("env").and_then(Value::as_mapping) else {
        return false;
    };
    if CRITICAL_ENVIRONMENT.iter().any(|name| {
        environment
            .get(Value::String((*name).to_owned()))
            .and_then(Value::as_str)
            != Some("")
    }) {
        return false;
    }

    let Some(run) = step.get("run").and_then(Value::as_str) else {
        return false;
    };
    let Some(sanitize_index) = run.find(CANONICAL_SANITIZE) else {
        return false;
    };
    if run[sanitize_index + CANONICAL_SANITIZE.len()..].contains(CANONICAL_SANITIZE) {
        return false;
    }
    let Some(assertion_index) = run.find(FIRST_TOOLCHAIN_ASSERTION) else {
        return false;
    };
    sanitize_index < assertion_index
}

#[test]
fn live_routed_step_neutralizes_dynamic_loader_authority_before_toolchain_execution() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        dynamic_loader_authority_is_neutralized(&source),
        "routed evidence must clear inherited GNU loader injection/search/tunable authority before the shell and toolchain execute"
    );
}

#[test]
fn missing_step_environment_neutralization_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {FIRST_TOOLCHAIN_ASSERTION}\n"
    );
    assert!(!dynamic_loader_authority_is_neutralized(&source));
}

#[test]
fn missing_preload_neutralization_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        env:\n          LD_LIBRARY_PATH: \"\"\n          LD_AUDIT: \"\"\n          GLIBC_TUNABLES: \"\"\n        run: |\n          {CANONICAL_SANITIZE}\n          {FIRST_TOOLCHAIN_ASSERTION}\n"
    );
    assert!(!dynamic_loader_authority_is_neutralized(&source));
}

#[test]
fn child_process_sanitization_after_toolchain_execution_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        env:\n          LD_PRELOAD: \"\"\n          LD_LIBRARY_PATH: \"\"\n          LD_AUDIT: \"\"\n          GLIBC_TUNABLES: \"\"\n        run: |\n          {FIRST_TOOLCHAIN_ASSERTION}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!dynamic_loader_authority_is_neutralized(&source));
}

#[test]
fn canonical_pre_shell_and_child_process_neutralization_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        env:\n          LD_PRELOAD: \"\"\n          LD_LIBRARY_PATH: \"\"\n          LD_AUDIT: \"\"\n          GLIBC_TUNABLES: \"\"\n        run: |\n          {CANONICAL_SANITIZE}\n          {FIRST_TOOLCHAIN_ASSERTION}\n"
    );
    assert!(dynamic_loader_authority_is_neutralized(&source));
}
