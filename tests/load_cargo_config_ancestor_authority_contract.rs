//! Fail-closed contract for Cargo configuration discovery in routed exact-head evidence.
//!
//! Cargo discovers `.cargo/config` and `.cargo/config.toml` from the current directory and every
//! ancestor directory, independently of `CARGO_HOME`. Those files can change the compiler,
//! linker, rustflags, dependency patches, profiles, and environment seen by build scripts without
//! changing the checked-in manifest or lockfile. Routed evidence therefore rejects every ambient
//! ancestor Cargo config before the measured release rebuild.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SCAN_START: &str = "cargo_config_dir=\"$PWD\"";
const LEGACY_CONFIG_GUARD: &str = "test ! -e \"$cargo_config_dir/.cargo/config\"";
const TOML_CONFIG_GUARD: &str = "test ! -e \"$cargo_config_dir/.cargo/config.toml\"";
const ROOT_BREAK: &str = "[ \"$cargo_config_dir\" = \"/\" ] && break";
const ASCEND: &str = "cargo_config_dir=\"$(dirname \"$cargo_config_dir\")\"";
const CARGO_CLEAN: &str = "cargo clean --release";

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

fn cargo_ancestor_config_is_rejected_before_build(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    let positions = [
        SCAN_START,
        "while :; do",
        LEGACY_CONFIG_GUARD,
        TOML_CONFIG_GUARD,
        ROOT_BREAK,
        ASCEND,
        "done",
    ]
    .map(|needle| lines.iter().position(|line| *line == needle));
    let Some(clean_index) = lines.iter().position(|line| *line == CARGO_CLEAN) else {
        return false;
    };

    if positions.iter().any(Option::is_none) {
        return false;
    }
    let positions = positions.map(Option::unwrap);
    positions.windows(2).all(|pair| pair[0] < pair[1]) && positions[6] < clean_index
}

#[test]
fn live_routed_evidence_rejects_ambient_cargo_config_ancestors() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        cargo_ancestor_config_is_rejected_before_build(&source),
        "routed evidence must reject Cargo config files in the workspace and every ancestor before the release rebuild"
    );
}

#[test]
fn cargo_home_only_guard_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          [[ ! -e \"${{CARGO_HOME}}/config\" && ! -e \"${{CARGO_HOME}}/config.toml\" ]]\n          {CARGO_CLEAN}\n"
    );
    assert!(!cargo_ancestor_config_is_rejected_before_build(&source));
}

#[test]
fn scan_after_release_rebuild_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {SCAN_START}\n          while :; do\n            {LEGACY_CONFIG_GUARD}\n            {TOML_CONFIG_GUARD}\n            {ROOT_BREAK}\n            {ASCEND}\n          done\n"
    );
    assert!(!cargo_ancestor_config_is_rejected_before_build(&source));
}

#[test]
fn complete_ancestor_scan_before_release_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {SCAN_START}\n          while :; do\n            {LEGACY_CONFIG_GUARD}\n            {TOML_CONFIG_GUARD}\n            {ROOT_BREAK}\n            {ASCEND}\n          done\n          {CARGO_CLEAN}\n"
    );
    assert!(cargo_ancestor_config_is_rejected_before_build(&source));
}
