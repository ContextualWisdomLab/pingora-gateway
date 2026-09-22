//! Fail-closed trust-root contract for Rust toolchain resolution in routed load evidence.
//!
//! A root-owned rustup proxy is not sufficient when `RUSTUP_HOME` still defaults to the runner
//! user's writable `~/.rustup`: the proxy delegates to the selected toolchain under that mutable
//! root. The routed evidence shell must bind Cargo and rustup to the image-owned `/etc/skel`
//! installation roots, export those roots and the image-owned stable x86_64 toolchain read-only,
//! and reject declarative Rust/Cargo compiler overrides that could redirect the point-of-use rebuild.

use serde_yaml::{Mapping, Value};
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_PATH: &str =
    "readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
const CANONICAL_CARGO_HOME: &str = "declare -rx CARGO_HOME=/etc/skel/.cargo";
const CANONICAL_RUSTUP_HOME: &str = "declare -rx RUSTUP_HOME=/etc/skel/.rustup";
const CANONICAL_RUSTUP_TOOLCHAIN: &str =
    "declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu";

const FORBIDDEN_ENV_KEYS: &[&str] = &[
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
];

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn mapping_has_forbidden_env(mapping: Option<&Mapping>) -> bool {
    mapping.is_some_and(|mapping| {
        FORBIDDEN_ENV_KEYS.iter().any(|key| {
            mapping.contains_key(Value::String((*key).to_owned()))
        })
    })
}

fn node_has_forbidden_env(node: &Value) -> bool {
    mapping_has_forbidden_env(node.get("env").and_then(Value::as_mapping))
}

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn routed_rust_toolchain_is_root_owned(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if node_has_forbidden_env(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if node_has_forbidden_env(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    if node_has_forbidden_env(routed) {
        return false;
    }

    let Some(run) = routed.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);
    let Some(path_index) = lines.iter().position(|line| *line == CANONICAL_PATH) else {
        return false;
    };

    lines.get(path_index + 1).copied() == Some(CANONICAL_CARGO_HOME)
        && lines.get(path_index + 2).copied() == Some(CANONICAL_RUSTUP_HOME)
        && lines.get(path_index + 3).copied() == Some(CANONICAL_RUSTUP_TOOLCHAIN)
        && lines
            .iter()
            .filter(|line| line.contains("CARGO_HOME"))
            .copied()
            .eq([CANONICAL_CARGO_HOME])
        && lines
            .iter()
            .filter(|line| line.contains("RUSTUP_HOME"))
            .copied()
            .eq([CANONICAL_RUSTUP_HOME])
        && lines
            .iter()
            .filter(|line| line.contains("RUSTUP_TOOLCHAIN"))
            .copied()
            .eq([CANONICAL_RUSTUP_TOOLCHAIN])
}

#[test]
fn live_routed_rebuild_uses_image_owned_rustup_root() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_rust_toolchain_is_root_owned(&source),
        "routed candidate rebuild must export read-only image-owned Cargo/rustup roots and toolchain before rustc/cargo resolution"
    );
}

#[test]
fn root_owned_proxy_with_default_user_rustup_home_is_not_sufficient() {
    let source = r#"
env:
  EXPECTED_SHA: deadbeef
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
          rustc --version
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_rust_toolchain_is_root_owned(source));
}

#[test]
fn unexported_readonly_rustup_bindings_are_not_sufficient() {
    let source = r#"
env:
  EXPECTED_SHA: deadbeef
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
          readonly RUSTUP_HOME=/etc/skel/.rustup
          readonly RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
          rustc --version
"#;
    assert!(!routed_rust_toolchain_is_root_owned(source));
}

#[test]
fn declarative_rustc_wrapper_override_is_rejected() {
    let source = r#"
env:
  EXPECTED_SHA: deadbeef
jobs:
  load-contract:
    env:
      RUSTC_WRAPPER: /tmp/fake-rustc-wrapper
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
          declare -rx CARGO_HOME=/etc/skel/.cargo
          declare -rx RUSTUP_HOME=/etc/skel/.rustup
          declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
"#;
    assert!(!routed_rust_toolchain_is_root_owned(source));
}

#[test]
fn declarative_workspace_rustc_wrapper_alias_is_rejected() {
    let source = r#"
env:
  EXPECTED_SHA: deadbeef
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        env:
          CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER: /tmp/fake-workspace-wrapper
        run: |
          set -euo pipefail
          readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
          declare -rx CARGO_HOME=/etc/skel/.cargo
          declare -rx RUSTUP_HOME=/etc/skel/.rustup
          declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
"#;
    assert!(!routed_rust_toolchain_is_root_owned(source));
}

#[test]
fn canonical_exported_root_owned_rustup_binding_is_admitted() {
    let source = r#"
env:
  EXPECTED_SHA: deadbeef
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
          declare -rx CARGO_HOME=/etc/skel/.cargo
          declare -rx RUSTUP_HOME=/etc/skel/.rustup
          declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
          rustc --version
"#;
    assert!(routed_rust_toolchain_is_root_owned(source));
}
