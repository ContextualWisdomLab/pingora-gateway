//! Hosted-runner regression for the routed candidate's Cargo cache/home.
//!
//! A full `cargo clean --release` removes dependency artifacts that can hide an unusable Cargo
//! home. Git dependencies then need writable `$CARGO_HOME/git` state. The measurement step keeps
//! executable/rustup authority under the image-owned `/etc/skel` roots but recreates a dedicated
//! runner-owned Cargo home before configuration guards and the full locked rebuild.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CARGO_HOME: &str = "declare -rx CARGO_HOME=/tmp/cwl-routed-cargo-home";
const RESET: [&str; 3] = [
    "rm -rf -- \"$CARGO_HOME\"",
    "install -d -m 0700 -- \"$CARGO_HOME\"",
    "[[ -d \"$CARGO_HOME\" && ! -L \"$CARGO_HOME\" && -O \"$CARGO_HOME\" && -w \"$CARGO_HOME\" ]]",
];
const HOME_CONFIG_GUARD: &str =
    "[[ ! -e \"${CARGO_HOME}/config\" && ! -e \"${CARGO_HOME}/config.toml\" ]]";
const CLEAN: &str = "cargo clean --release";
const BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn position(lines: &[&str], needle: &str) -> Option<usize> {
    let matches: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == needle).then_some(index))
        .collect();
    (matches.len() == 1).then_some(matches[0])
}

fn routed_rebuild_has_isolated_writable_cargo_home(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(step) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    let Some(run) = step.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);

    let Some(home) = position(&lines, CARGO_HOME) else {
        return false;
    };
    let Some(reset) = lines.windows(RESET.len()).position(|window| window == RESET) else {
        return false;
    };
    let Some(config_guard) = position(&lines, HOME_CONFIG_GUARD) else {
        return false;
    };
    let Some(clean) = position(&lines, CLEAN) else {
        return false;
    };
    let Some(build) = position(&lines, BUILD) else {
        return false;
    };

    home < reset
        && reset + RESET.len() <= config_guard
        && config_guard < clean
        && clean < build
        && !run.contains("declare -rx CARGO_HOME=/etc/skel/.cargo")
        && !run.contains("declare -rx CARGO_HOME=/home/runner/.cargo")
}

#[test]
fn live_routed_full_rebuild_uses_fresh_writable_cargo_home() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_rebuild_has_isolated_writable_cargo_home(&source),
        "full point-of-use rebuild must use a freshly recreated writable Cargo cache/home while keeping config authority fail closed"
    );
}

#[test]
fn read_only_image_template_cargo_home_reproduces_the_hosted_red_shape() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          declare -rx CARGO_HOME=/etc/skel/.cargo
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_rebuild_has_isolated_writable_cargo_home(source));
}

#[test]
fn mutable_runner_default_cargo_home_is_not_an_admitted_substitute() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          declare -rx CARGO_HOME=/home/runner/.cargo
          rm -rf -- "$CARGO_HOME"
          install -d -m 0700 -- "$CARGO_HOME"
          [[ -d "$CARGO_HOME" && ! -L "$CARGO_HOME" && -O "$CARGO_HOME" && -w "$CARGO_HOME" ]]
          [[ ! -e "${CARGO_HOME}/config" && ! -e "${CARGO_HOME}/config.toml" ]]
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!routed_rebuild_has_isolated_writable_cargo_home(source));
}
