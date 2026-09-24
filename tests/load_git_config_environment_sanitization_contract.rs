//! Fail-closed contract for inherited Git protected-configuration authority in routed evidence.
//!
//! Git reads system/global configuration before repository-local configuration and also accepts
//! command-scope configuration through `GIT_CONFIG_COUNT` plus numbered key/value pairs. The routed
//! step uses Git as a provenance oracle, so inherited runner configuration must not be able to
//! redirect those protected configuration scopes. System/global files are pinned to `/dev/null`,
//! and command-scope/system-suppression environment authority is removed before the first Git proof.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const GLOBAL_CONFIG: &str = "declare -rx GIT_CONFIG_GLOBAL=/dev/null";
const SYSTEM_CONFIG: &str = "declare -rx GIT_CONFIG_SYSTEM=/dev/null";
const CONFIG_SANITIZER: &str = "unset GIT_CONFIG_NOSYSTEM GIT_CONFIG_COUNT";
const FIRST_GIT_PROOF: &str =
    "test \"$(git --no-replace-objects rev-parse HEAD)\" = \"$EXPECTED_SHA\"";

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

fn git_config_environment_is_canonical(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines: Vec<_> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let Some(proof_index) = lines.iter().position(|line| *line == FIRST_GIT_PROOF) else {
        return false;
    };

    [GLOBAL_CONFIG, SYSTEM_CONFIG, CONFIG_SANITIZER]
        .iter()
        .all(|required| {
            let matches: Vec<_> = lines
                .iter()
                .enumerate()
                .filter_map(|(index, line)| (*line == *required).then_some(index))
                .collect();
            matches.len() == 1 && matches[0] < proof_index
        })
}

#[test]
fn live_routed_evidence_neutralizes_inherited_git_config_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        git_config_environment_is_canonical(&source),
        "routed exact-source provenance must neutralize inherited Git system/global/command-scope configuration before Git is used as an oracle"
    );
}

#[test]
fn missing_command_scope_sanitizer_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {GLOBAL_CONFIG}\n          {SYSTEM_CONFIG}\n          {FIRST_GIT_PROOF}\n"
    );
    assert!(!git_config_environment_is_canonical(&source));
}

#[test]
fn redirected_global_config_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          declare -rx GIT_CONFIG_GLOBAL=/tmp/runner.gitconfig\n          {SYSTEM_CONFIG}\n          {CONFIG_SANITIZER}\n          {FIRST_GIT_PROOF}\n"
    );
    assert!(!git_config_environment_is_canonical(&source));
}

#[test]
fn sanitizer_after_git_proof_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {FIRST_GIT_PROOF}\n          {GLOBAL_CONFIG}\n          {SYSTEM_CONFIG}\n          {CONFIG_SANITIZER}\n"
    );
    assert!(!git_config_environment_is_canonical(&source));
}

#[test]
fn null_protected_config_before_git_proof_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {GLOBAL_CONFIG}\n          {SYSTEM_CONFIG}\n          {CONFIG_SANITIZER}\n          {FIRST_GIT_PROOF}\n"
    );
    assert!(git_config_environment_is_canonical(&source));
}
