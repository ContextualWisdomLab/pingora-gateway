//! Fail-closed contract for Cargo release-profile authority in routed-load evidence.
//!
//! The routed benchmark is meaningful only when `--release` means the repository's canonical
//! release profile. Cargo permits profile settings to be overridden from environment variables,
//! which can change optimizer/LTO/codegen semantics independently of `Cargo.toml`. This contract
//! rejects workflow, job, and routed-step release-profile overrides so performance evidence cannot
//! be produced by a differently optimized candidate than the one described by source control.

use serde_yaml::{Mapping, Value};
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const RELEASE_PROFILE_ENV_PREFIX: &str = "CARGO_PROFILE_RELEASE_";

fn mapping_has_release_profile_override(mapping: Option<&Mapping>) -> bool {
    mapping.is_some_and(|mapping| {
        mapping.keys().any(|key| {
            key.as_str()
                .is_some_and(|key| key.starts_with(RELEASE_PROFILE_ENV_PREFIX))
        })
    })
}

fn node_has_release_profile_override(node: &Value) -> bool {
    mapping_has_release_profile_override(node.get("env").and_then(Value::as_mapping))
}

fn routed_release_profile_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if node_has_release_profile_override(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if node_has_release_profile_override(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let mut routed_steps = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let Some(routed) = routed_steps.next() else {
        return false;
    };
    if routed_steps.next().is_some() || node_has_release_profile_override(routed) {
        return false;
    }

    true
}

#[test]
fn live_routed_candidate_uses_repository_release_profile() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_release_profile_is_canonical(&source),
        "routed evidence must not override the repository release profile"
    );
}

#[test]
fn release_opt_level_override_is_rejected() {
    let source = r#"
env:
  CARGO_PROFILE_RELEASE_OPT_LEVEL: "3"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_release_profile_is_canonical(source));
}

#[test]
fn release_lto_override_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    env:
      CARGO_PROFILE_RELEASE_LTO: "fat"
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_release_profile_is_canonical(source));
}

#[test]
fn release_codegen_units_override_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        env:
          CARGO_PROFILE_RELEASE_CODEGEN_UNITS: "1"
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_release_profile_is_canonical(source));
}

#[test]
fn unrelated_cargo_environment_is_not_a_profile_override() {
    let source = r#"
env:
  CARGO_TERM_COLOR: always
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(routed_release_profile_is_canonical(source));
}
