//! Regression coverage for shell-word forms that can select an alternate Cargo toolchain.

use serde_yaml::Value;
use std::fs;

/// Mirrors the shell's removal of an unquoted backslash-newline pair before tokenization.
fn normalize_shell_continuations(script: &str) -> String {
    script.replace("\\\r\n", "").replace("\\\n", "")
}

/// Detects Cargo's explicit `+<toolchain>` selector across ordinary shell whitespace.
fn contains_explicit_cargo_toolchain_selector(script: &str) -> bool {
    let normalized = normalize_shell_continuations(script);

    normalized.lines().any(|line| {
        let tokens: Vec<_> = line.split_whitespace().collect();
        tokens.windows(2).any(|window| {
            let command = window[0].trim_matches(|character| character == '\'' || character == '"');
            let command = command.rsplit('/').next().unwrap_or(command);
            command == "cargo" && window[1].starts_with('+') && window[1].len() > 1
        })
    })
}

/// Extracts every semantic YAML `run` script from the workflows that produce release evidence.
fn workflow_run_scripts(path: &str) -> Vec<String> {
    let workflow = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required workflow {path} is missing: {error}"));
    let document: Value = serde_yaml::from_str(&workflow)
        .unwrap_or_else(|error| panic!("workflow YAML {path} must parse: {error}"));
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("workflow must contain a jobs mapping");

    jobs.values()
        .flat_map(|job| {
            job.get("steps")
                .and_then(Value::as_sequence)
                .into_iter()
                .flatten()
                .filter_map(|step| step.get("run").and_then(Value::as_str))
                .map(str::to_owned)
        })
        .collect()
}

/// Current release workflows must not select an alternate Cargo toolchain through shell quoting.
#[test]
fn release_workflows_reject_explicit_cargo_toolchain_selectors() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/supply-chain.yml",
    ] {
        for script in workflow_run_scripts(path) {
            assert!(
                !contains_explicit_cargo_toolchain_selector(&script),
                "{path} must not select a compiler with cargo +<toolchain>"
            );
        }
    }
}

/// Proves quoted and escaped shell words cannot hide Cargo's rustup override shorthand.
#[test]
fn quoted_cargo_toolchain_selectors_are_detected() {
    for command in [
        "cargo '+1.98.0' build",
        "cargo \"+1.98.0\" build",
        "cargo \\+1.98.0 build",
        "cargo \"+\"1.98.0 build",
        "\"/home/runner/.cargo/bin/cargo\" '+1.98.0' build",
    ] {
        assert!(
            contains_explicit_cargo_toolchain_selector(command),
            "selector form must be detected: {command}"
        );
    }
}
