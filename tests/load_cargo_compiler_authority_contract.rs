//! Fail-closed contract for Cargo compiler-argument authority in routed-load evidence.
//!
//! `RUSTFLAGS` and `CARGO_ENCODED_RUSTFLAGS` are not the only declarative paths that can alter the
//! point-of-use release build. Cargo also maps `build.rustflags` and target-specific linker/rustflags
//! configuration from environment variables. The routed benchmark rejects those authorities so
//! performance evidence cannot be produced by a differently compiled or linked binary than the
//! source-controlled build contract describes.

use serde_yaml::{Mapping, Value};
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

fn is_cargo_compiler_override(key: &str) -> bool {
    key == "CARGO_BUILD_RUSTFLAGS"
        || key.strip_prefix("CARGO_TARGET_").is_some_and(|target_key| {
            target_key.ends_with("_RUSTFLAGS") || target_key.ends_with("_LINKER")
        })
}

fn mapping_has_compiler_override(mapping: Option<&Mapping>) -> bool {
    mapping.is_some_and(|mapping| {
        mapping
            .keys()
            .filter_map(Value::as_str)
            .any(is_cargo_compiler_override)
    })
}

fn node_has_compiler_override(node: &Value) -> bool {
    mapping_has_compiler_override(node.get("env").and_then(Value::as_mapping))
}

fn routed_candidate_compiler_authority_is_canonical(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if node_has_compiler_override(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if node_has_compiler_override(job) {
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
    if routed_steps.next().is_some() || node_has_compiler_override(routed) {
        return false;
    }

    true
}

#[test]
fn live_routed_candidate_has_no_declarative_cargo_compiler_override() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_candidate_compiler_authority_is_canonical(&source),
        "routed evidence must not admit Cargo compiler/linker overrides outside source control"
    );
}

#[test]
fn cargo_build_rustflags_override_is_rejected() {
    let source = r#"
env:
  CARGO_BUILD_RUSTFLAGS: "-C target-cpu=native"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_candidate_compiler_authority_is_canonical(source));
}

#[test]
fn target_specific_rustflags_override_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    env:
      CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS: "-C target-cpu=native"
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_candidate_compiler_authority_is_canonical(source));
}

#[test]
fn target_specific_linker_override_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        env:
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER: /tmp/wrapper-linker
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_candidate_compiler_authority_is_canonical(source));
}

#[test]
fn target_runner_is_not_a_build_compiler_override() {
    let source = r#"
env:
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER: /usr/bin/env
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(routed_candidate_compiler_authority_is_canonical(source));
}

#[test]
fn unrelated_cargo_environment_is_admitted() {
    let source = r#"
env:
  CARGO_TERM_COLOR: always
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(routed_candidate_compiler_authority_is_canonical(source));
}
