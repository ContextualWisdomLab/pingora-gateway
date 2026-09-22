//! Fail-closed trust contract for Cargo configuration during routed-load candidate rebuilds.
//!
//! Cargo merges `.cargo/config.toml` (and legacy `.cargo/config`) from the current directory and
//! every ancestor, then `$CARGO_HOME/config.toml`. A rooted `cargo` executable therefore does not
//! by itself prove that the measured candidate was built under repository-controlled settings.
//! The routed evidence step pins the image-owned Cargo home and rejects every discovered config
//! location before the point-of-use `cargo clean`/`cargo build` sequence.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_CARGO_HOME: &str = "declare -rx CARGO_HOME=/etc/skel/.cargo";
const CARGO_CLEAN: &str = "cargo clean -p cwl-pingora-gateway --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";

const CONFIG_GUARD: [&str; 9] = [
    "cargo_config_dir=\"$PWD\"",
    "while :; do",
    "[[ ! -e \"${cargo_config_dir}/.cargo/config\" ]]",
    "[[ ! -e \"${cargo_config_dir}/.cargo/config.toml\" ]]",
    "[[ \"${cargo_config_dir}\" = \"/\" ]] && break",
    "cargo_config_dir=\"${cargo_config_dir%/*}\"",
    "[[ -n \"${cargo_config_dir}\" ]] || cargo_config_dir=\"/\"",
    "done",
    "[[ ! -e \"${CARGO_HOME}/config\" && ! -e \"${CARGO_HOME}/config.toml\" ]]",
];

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn env_overrides_cargo_home(node: &Value) -> bool {
    node.get("env")
        .and_then(|env| env.get("CARGO_HOME"))
        .is_some()
}

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn contains_ordered_sequence(lines: &[&str], sequence: &[&str]) -> Option<usize> {
    lines
        .windows(sequence.len())
        .position(|window| window == sequence)
}

fn routed_cargo_configuration_is_trusted(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if env_overrides_cargo_home(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if env_overrides_cargo_home(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    if env_overrides_cargo_home(routed) {
        return false;
    }

    let Some(run) = routed.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);

    let cargo_home_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == CANONICAL_CARGO_HOME).then_some(index))
        .collect();
    if cargo_home_positions.len() != 1 {
        return false;
    }

    let Some(guard_start) = contains_ordered_sequence(&lines, &CONFIG_GUARD) else {
        return false;
    };
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = lines.iter().position(|line| *line == CARGO_BUILD) else {
        return false;
    };

    cargo_home_positions[0] < guard_start
        && guard_start + CONFIG_GUARD.len() <= clean_index
        && clean_index < build_index
}

#[test]
fn live_routed_candidate_rebuild_has_no_mutable_cargo_configuration_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_cargo_configuration_is_trusted(&source),
        "routed candidate rebuild must pin image-owned CARGO_HOME and reject Cargo configs from the workspace ancestry and Cargo home before rebuilding"
    );
}

#[test]
fn rustup_only_trust_root_does_not_cover_cargo_configuration() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          declare -rx RUSTUP_HOME=/etc/skel/.rustup
          declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
          cargo clean -p cwl-pingora-gateway --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_cargo_configuration_is_trusted(source));
}

#[test]
fn step_level_cargo_home_override_is_not_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        env:
          CARGO_HOME: /tmp/user-cargo
        run: |
          {CANONICAL_CARGO_HOME}
          {}
          {CARGO_CLEAN}
          {CARGO_BUILD}
"#,
        CONFIG_GUARD.join("\n          ")
    );
    assert!(!routed_cargo_configuration_is_trusted(&source));
}

#[test]
fn canonical_cargo_trust_root_and_ancestor_guard_are_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          {CANONICAL_CARGO_HOME}
          {}
          {CARGO_CLEAN}
          {CARGO_BUILD}
"#,
        CONFIG_GUARD.join("\n          ")
    );
    assert!(routed_cargo_configuration_is_trusted(&source));
}
