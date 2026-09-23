//! Fail-closed contract for the exact Rust toolchain used by routed load evidence.
//!
//! `RUSTUP_TOOLCHAIN=stable-*` is a channel selector, not an immutable compiler identity. The
//! hosted image can advance that channel independently of this repository, so routed evidence must
//! prove the exact rustc/cargo point release before the candidate rebuild. CI setup must install the
//! same point release so test, lint, coverage, fixture compilation, and measured rebuilds do not
//! silently diverge.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const PINNED_VERSION: &str = "1.98.1";
const INSTALL_STEP: &str = "Install Rust 1.98.1";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const RUSTUP_DECLARATION: &str =
    "declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu";
const RUSTC_VERSION_ASSERTION: &str = r#"[[ "$(rustc --version)" == rustc\ 1.98.1\ * ]]"#;
const CARGO_VERSION_ASSERTION: &str = r#"[[ "$(cargo --version)" == cargo\ 1.98.1\ * ]]"#;

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn install_step_pins(step: &Value, required_component_suffix: &str) -> bool {
    let Some(run) = step.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);
    let install = format!(
        "rustup toolchain install {PINNED_VERSION} --profile minimal --component {required_component_suffix}"
    );
    let default = format!("rustup default {PINNED_VERSION}");
    lines.iter().any(|line| *line == install)
        && lines.iter().any(|line| *line == default)
}

fn workflow_pins_exact_toolchain(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(jobs) = document.get("jobs") else {
        return false;
    };

    let Some(test_steps) = jobs
        .get("test")
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
    else {
        return false;
    };
    let Some(test_install) = unique_named_step(test_steps, INSTALL_STEP) else {
        return false;
    };
    if !install_step_pins(test_install, "clippy,rustfmt") {
        return false;
    }

    let Some(load_steps) = jobs
        .get("load-contract")
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
    else {
        return false;
    };
    let Some(load_install) = unique_named_step(load_steps, INSTALL_STEP) else {
        return false;
    };
    if !install_step_pins(load_install, "rustfmt") {
        return false;
    }

    let Some(routed) = unique_named_step(load_steps, ROUTED_STEP) else {
        return false;
    };
    let Some(run) = routed.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);
    let Some(toolchain_index) = lines.iter().position(|line| *line == RUSTUP_DECLARATION) else {
        return false;
    };

    lines.get(toolchain_index + 1).copied() == Some(RUSTC_VERSION_ASSERTION)
        && lines.get(toolchain_index + 2).copied() == Some(CARGO_VERSION_ASSERTION)
}

#[test]
fn live_workflow_proves_the_exact_rust_point_release() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        workflow_pins_exact_toolchain(&source),
        "CI and routed evidence must use and verify Rust 1.98.1 rather than trusting a mutable stable channel"
    );
}

#[test]
fn mutable_stable_without_point_release_assertions_is_rejected() {
    let source = r#"
jobs:
  test:
    steps:
      - name: Install Rust 1.98.1
        run: |
          rustup toolchain install 1.98.1 --profile minimal --component clippy,rustfmt
          rustup default 1.98.1
  load-contract:
    steps:
      - name: Install Rust 1.98.1
        run: |
          rustup toolchain install 1.98.1 --profile minimal --component rustfmt
          rustup default 1.98.1
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!workflow_pins_exact_toolchain(source));
}

#[test]
fn stale_point_release_is_rejected() {
    let source = r#"
jobs:
  test:
    steps:
      - name: Install Rust 1.98.0
        run: |
          rustup toolchain install 1.98.0 --profile minimal --component clippy,rustfmt
          rustup default 1.98.0
  load-contract:
    steps:
      - name: Install Rust 1.98.0
        run: |
          rustup toolchain install 1.98.0 --profile minimal --component rustfmt
          rustup default 1.98.0
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
          [[ "$(rustc --version)" == rustc\ 1.98.0\ * ]]
          [[ "$(cargo --version)" == cargo\ 1.98.0\ * ]]
"#;
    assert!(!workflow_pins_exact_toolchain(source));
}
