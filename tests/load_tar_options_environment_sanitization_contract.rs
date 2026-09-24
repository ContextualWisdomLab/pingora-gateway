//! Fail-closed contract for GNU tar option authority in k6 provenance extraction.
//!
//! GNU tar prepends `TAR_OPTIONS` to explicit command-line options. A runner-inherited value can
//! therefore alter extraction semantics, including path handling and checkpoint actions, even when
//! the supplier archive itself is checksum-pinned. Both the initial k6 install and the routed
//! point-of-use re-extraction must clear this authority before invoking `tar`.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const INSTALL_STEP: &str = "Install checksum-pinned k6 2.2.0";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SANITIZER: &str = "unset TAR_OPTIONS";
const TAR_COMMAND_PREFIX: &str = "tar -xzf ";

fn unique_named_run(source: &str, step_name: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(step_name));
    let run = matches.next()?.get("run")?.as_str()?.to_owned();
    matches.next().is_none().then_some(run)
}

fn tar_options_are_sanitized(run: &str) -> bool {
    let lines = run.lines().map(str::trim).collect::<Vec<_>>();
    let sanitizer_positions = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == SANITIZER).then_some(index))
        .collect::<Vec<_>>();
    let tar_positions = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.starts_with(TAR_COMMAND_PREFIX).then_some(index))
        .collect::<Vec<_>>();

    sanitizer_positions.len() == 1
        && tar_positions.len() == 1
        && sanitizer_positions[0] < tar_positions[0]
        && !lines.iter().any(|line| {
            !line.starts_with('#')
                && *line != SANITIZER
                && (line.starts_with("TAR_OPTIONS=")
                    || line.starts_with("export TAR_OPTIONS=")
                    || line.starts_with("declare -x TAR_OPTIONS="))
        })
}

fn workflow_sanitizes_tar_options(source: &str) -> bool {
    [INSTALL_STEP, ROUTED_STEP].into_iter().all(|step_name| {
        unique_named_run(source, step_name)
            .as_deref()
            .is_some_and(tar_options_are_sanitized)
    })
}

#[test]
fn live_k6_extractions_clear_inherited_tar_options() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        workflow_sanitizes_tar_options(&source),
        "checksum-pinned k6 extraction must clear inherited GNU tar default options before every provenance extraction"
    );
}

#[test]
fn inherited_tar_options_without_cleanup_are_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {INSTALL_STEP}\n        run: |\n          tar -xzf /tmp/k6.tgz -C /tmp\n      - name: {ROUTED_STEP}\n        run: |\n          {SANITIZER}\n          tar -xzf /tmp/k6.tgz -C /tmp/routed\n"
    );
    assert!(!workflow_sanitizes_tar_options(&source));
}

#[test]
fn cleanup_after_extraction_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {INSTALL_STEP}\n        run: |\n          tar -xzf /tmp/k6.tgz -C /tmp\n          {SANITIZER}\n      - name: {ROUTED_STEP}\n        run: |\n          {SANITIZER}\n          tar -xzf /tmp/k6.tgz -C /tmp/routed\n"
    );
    assert!(!workflow_sanitizes_tar_options(&source));
}

#[test]
fn reintroduced_tar_options_are_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {INSTALL_STEP}\n        run: |\n          {SANITIZER}\n          export TAR_OPTIONS=--checkpoint=1\n          tar -xzf /tmp/k6.tgz -C /tmp\n      - name: {ROUTED_STEP}\n        run: |\n          {SANITIZER}\n          tar -xzf /tmp/k6.tgz -C /tmp/routed\n"
    );
    assert!(!workflow_sanitizes_tar_options(&source));
}

#[test]
fn canonical_tar_option_sanitization_is_admitted() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {INSTALL_STEP}\n        run: |\n          {SANITIZER}\n          tar -xzf /tmp/k6.tgz -C /tmp\n      - name: {ROUTED_STEP}\n        run: |\n          {SANITIZER}\n          tar -xzf /tmp/k6.tgz -C /tmp/routed\n"
    );
    assert!(workflow_sanitizes_tar_options(&source));
}
