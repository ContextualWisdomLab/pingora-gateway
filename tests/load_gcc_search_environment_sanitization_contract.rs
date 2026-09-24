//! Fail-closed contract for inherited GCC driver/search-path authority in routed-load evidence.
//!
//! The measured native build executes GCC through `cc::Build`. GCC independently consumes
//! `GCC_EXEC_PREFIX` and `COMPILER_PATH` for subprogram lookup, `LIBRARY_PATH` for linker
//! startfiles/libraries, and `CPATH` plus language-specific include variables for header search.
//! Pinning the `cc` executable and clearing `CC`/`CFLAGS` therefore does not neutralize these
//! driver-level build inputs.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CARGO_CLEAN: &str = "cargo clean --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";
const CANONICAL_SANITIZE: &str =
    "unset GCC_EXEC_PREFIX COMPILER_PATH LIBRARY_PATH CPATH C_INCLUDE_PATH CPLUS_INCLUDE_PATH OBJC_INCLUDE_PATH";

fn routed_run(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let run = matches.next()?.get("run")?.as_str()?.to_owned();
    matches.next().is_none().then_some(run)
}

fn gcc_search_environment_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(sanitize_index) = run.find(CANONICAL_SANITIZE) else {
        return false;
    };
    if run[sanitize_index + CANONICAL_SANITIZE.len()..].contains(CANONICAL_SANITIZE) {
        return false;
    }
    let Some(clean_index) = run.find(CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = run.find(CARGO_BUILD) else {
        return false;
    };
    sanitize_index < clean_index && clean_index < build_index
}

#[test]
fn live_routed_rebuild_neutralizes_inherited_gcc_search_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        gcc_search_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited GCC subprogram, library, and header-search authority before rebuilding the measured candidate"
    );
}

#[test]
fn clean_rebuild_without_gcc_search_sanitization_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!gcc_search_environment_is_sanitized_before_rebuild(source));
}

#[test]
fn gcc_exec_prefix_authority_must_not_survive() {
    let incomplete = CANONICAL_SANITIZE.replace("GCC_EXEC_PREFIX ", "");
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {incomplete}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!gcc_search_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn header_and_library_search_authority_must_not_survive() {
    let incomplete = CANONICAL_SANITIZE.replace(
        " LIBRARY_PATH CPATH C_INCLUDE_PATH CPLUS_INCLUDE_PATH OBJC_INCLUDE_PATH",
        "",
    );
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {incomplete}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(!gcc_search_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn gcc_search_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {CANONICAL_SANITIZE}\n"
    );
    assert!(!gcc_search_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_gcc_search_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(gcc_search_environment_is_sanitized_before_rebuild(&source));
}
