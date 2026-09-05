//! Regression contract for shell control operators around compiler authority.

use serde_yaml::Value;
use std::fs;

const FORBIDDEN_COMPILER_AUTHORITIES: [&str; 3] = ["RUSTC", "CARGO_BUILD_RUSTC", "RUSTUP_TOOLCHAIN"];

/// Reads semantic `run` scripts from every workflow job.
fn workflow_run_scripts(workflow: &str) -> Vec<String> {
    let document: Value = serde_yaml::from_str(workflow)
        .unwrap_or_else(|error| panic!("workflow YAML must parse: {error}"));
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("workflow must contain jobs mapping");

    jobs.values()
        .flat_map(|job| {
            job.get("steps")
                .and_then(Value::as_sequence)
                .into_iter()
                .flatten()
        })
        .filter_map(|step| step.get("run").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// Returns the assignment variable name for a simple shell assignment word.
fn assignment_name(word: &str) -> Option<&str> {
    let (name, _) = word.split_once('=')?;
    Some(name.strip_suffix('+').unwrap_or(name))
}

/// RED parser: shell control operators attached without whitespace remain inside one token.
fn command_segments(script: &str) -> Vec<Vec<String>> {
    script
        .lines()
        .map(|line| line.split_whitespace().map(str::to_owned).collect())
        .collect()
}

/// Detects compiler-selection environment authority in assignment prefixes or env/export commands.
fn segment_has_forbidden_compiler_authority(segment: &[String]) -> bool {
    if segment.is_empty() {
        return false;
    }

    for word in segment {
        let Some(name) = assignment_name(word) else {
            break;
        };
        if FORBIDDEN_COMPILER_AUTHORITIES.contains(&name) {
            return true;
        }
    }

    let command = segment[0].rsplit('/').next().unwrap_or(&segment[0]);
    if matches!(command, "env" | "export") {
        return segment[1..].iter().any(|word| {
            assignment_name(word)
                .is_some_and(|name| FORBIDDEN_COMPILER_AUTHORITIES.contains(&name))
        });
    }

    false
}

/// Requires security-sensitive compiler authority to remain visible across shell command boundaries.
fn assert_no_hidden_compiler_authority(context: &str, script: &str) {
    assert!(
        !command_segments(script)
            .iter()
            .any(|segment| segment_has_forbidden_compiler_authority(segment)),
        "{context} must not hide compiler authority behind shell control operators"
    );
}

#[test]
fn repository_workflow_shell_control_contract() {
    for path in [".github/workflows/ci.yml", ".github/workflows/supply-chain.yml"] {
        let workflow = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("required workflow {path} is missing: {error}"));
        for script in workflow_run_scripts(&workflow) {
            assert_no_hidden_compiler_authority(path, &script);
        }
    }
}

#[test]
fn shell_control_operator_cannot_hide_compiler_assignment() {
    for script in [
        "true;RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "true&&CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "false||RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", script);
        });
        assert!(result.is_err(), "control operator must not hide compiler authority: {script}");
    }
}
