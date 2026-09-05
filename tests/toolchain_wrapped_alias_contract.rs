//! Fail-closed acceptance for Cargo `+toolchain` selection hidden behind persistent shell aliases.

#[path = "support/toolchain_wrapped_alias.rs"]
mod toolchain_wrapped_alias_support;

use serde_yaml::Value;
use std::fs;
use toolchain_wrapped_alias_support::assert_no_wrapped_cargo_toolchain_selector;

fn workflow_run_scripts(path: &str) -> Vec<String> {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"));
    let document: Value = serde_yaml::from_str(&source)
        .unwrap_or_else(|error| panic!("workflow {path} must parse as YAML: {error}"));
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("workflow {path} must define a top-level jobs mapping"));

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

fn docker_run_commands(source: &str) -> Vec<String> {
    let normalized = source.replace("\\\r\n", "").replace("\\\n", "");
    normalized
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let boundary = trimmed.find(char::is_whitespace)?;
            trimmed[..boundary]
                .eq_ignore_ascii_case("RUN")
                .then(|| trimmed[boundary..].trim_start().to_owned())
        })
        .collect()
}

#[test]
fn release_paths_reject_wrapped_cargo_toolchain_aliases() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/supply-chain.yml",
    ] {
        for script in workflow_run_scripts(path) {
            assert_no_wrapped_cargo_toolchain_selector(path, &script);
        }
    }

    let dockerfile = fs::read_to_string("Dockerfile")
        .expect("required repository evidence Dockerfile is missing");
    for command in docker_run_commands(&dockerfile) {
        assert_no_wrapped_cargo_toolchain_selector("Dockerfile RUN", &command);
    }
}

#[test]
fn persistent_cargo_alias_cannot_select_toolchain_through_command_or_env() {
    for shell in [
        "CARGO=cargo; command \"$CARGO\" +1.98.0 build --release --locked",
        "CARGO=cargo; command -p -- \"${CARGO}\" +nightly build --release --locked",
        "CARGO=/usr/local/bin/cargo; env \"$CARGO\" +1.98.0 build --release --locked",
        "CARGO=cargo; env -- \"${CARGO}\" +nightly build --release --locked",
        "export CARGO=cargo; command \"$CARGO\" +1.98.0 build --release --locked",
        "readonly CARGO=cargo; env -i \"$CARGO\" +nightly build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_wrapped_cargo_toolchain_selector("synthetic shell", shell);
        });
        assert!(
            result.is_err(),
            "command/env wrapper must not hide Cargo +toolchain authority: {shell}"
        );
    }
}

#[test]
fn ordinary_alias_use_without_toolchain_selector_remains_admitted() {
    for shell in [
        "CARGO=cargo; \"$CARGO\" build --release --locked",
        "CARGO=cargo; command \"$CARGO\" build --release --locked",
        "CARGO=/usr/local/bin/cargo; env -- \"${CARGO}\" build --release --locked",
        "CARGO=cargo; unset CARGO; command \"$CARGO\" +nightly build --release --locked",
    ] {
        assert_no_wrapped_cargo_toolchain_selector("synthetic shell", shell);
    }
}
