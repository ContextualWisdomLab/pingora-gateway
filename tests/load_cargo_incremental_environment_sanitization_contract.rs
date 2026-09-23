//! Fail-closed contract for inherited Cargo incremental-build authority in routed-load evidence.
//!
//! Cargo documents `CARGO_BUILD_INCREMENTAL` and `CARGO_INCREMENTAL` as environment authority over
//! incremental compilation, with `CARGO_INCREMENTAL` able to override every profile. The routed
//! benchmark must remove both inherited values before its clean release rebuild so measured output
//! cannot depend on runner-supplied incremental settings outside the repository profile contract.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SANITIZER: &str = "unset CARGO_BUILD_INCREMENTAL CARGO_INCREMENTAL";
const RELEASE_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";

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
    let run = routed.next()?.get("run")?.as_str()?.to_owned();
    (routed.next().is_none()).then_some(run)
}

fn inherited_incremental_authority_is_sanitized(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(sanitizer) = run.find(SANITIZER) else {
        return false;
    };
    let Some(build) = run.find(RELEASE_BUILD) else {
        return false;
    };
    sanitizer < build
}

#[test]
fn live_routed_rebuild_sanitizes_inherited_incremental_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        inherited_incremental_authority_is_sanitized(&source),
        "routed release evidence must remove inherited Cargo incremental-build authority before rebuilding"
    );
}

#[test]
fn missing_incremental_sanitizer_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!inherited_incremental_authority_is_sanitized(source));
}

#[test]
fn sanitizer_after_release_build_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
          unset CARGO_BUILD_INCREMENTAL CARGO_INCREMENTAL
"#;
    assert!(!inherited_incremental_authority_is_sanitized(source));
}

#[test]
fn sanitizer_before_release_build_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          unset CARGO_BUILD_INCREMENTAL CARGO_INCREMENTAL
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(inherited_incremental_authority_is_sanitized(source));
}
