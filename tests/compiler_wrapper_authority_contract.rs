//! Fail-closed checks for Cargo compiler-wrapper authority outside the verified Rust toolchain.

use serde_yaml::Value;
use std::{fs, path::Path};

const FORBIDDEN_COMPILER_ENV: [&str; 7] = [
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    "RUSTUP_TOOLCHAIN",
];

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

fn normalize_shell_word(word: &str) -> String {
    let mut normalized = String::with_capacity(word.len());
    let mut characters = word.chars();

    while let Some(character) = characters.next() {
        match character {
            '\'' | '"' => {}
            '\\' => {
                if let Some(escaped) = characters.next() {
                    normalized.push(escaped);
                } else {
                    normalized.push('\\');
                }
            }
            _ => normalized.push(character),
        }
    }

    normalized
}

fn assignment_name(word: &str) -> Option<&str> {
    let (name, _) = word.split_once('=')?;
    Some(name.strip_suffix('+').unwrap_or(name))
}

fn assert_environment_mapping_has_no_compiler_wrapper(
    context: &str,
    environment: Option<&Value>,
) {
    let Some(environment) = environment.and_then(Value::as_mapping) else {
        return;
    };

    for forbidden in FORBIDDEN_COMPILER_ENV {
        assert!(
            !environment
                .keys()
                .any(|key| key.as_str() == Some(forbidden)),
            "{context} must not set compiler authority {forbidden} through YAML env"
        );
    }
}

fn assert_run_script_has_no_compiler_wrapper(context: &str, run: &str) {
    let normalized = run.replace("\\\r\n", "").replace("\\\n", "");

    for raw_word in normalized.split_whitespace() {
        let word = normalize_shell_word(raw_word);
        let Some(name) = assignment_name(&word) else {
            continue;
        };

        assert!(
            !FORBIDDEN_COMPILER_ENV.contains(&name),
            "{context} must not set compiler authority {name} in shell execution"
        );
    }
}

fn assert_workflow_has_no_compiler_wrapper(path: &str, workflow: &str) {
    let document: Value = serde_yaml::from_str(workflow)
        .unwrap_or_else(|error| panic!("{path} must parse as workflow YAML: {error}"));

    assert_environment_mapping_has_no_compiler_wrapper(path, document.get("env"));

    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("{path} must contain a jobs mapping"));

    for (job_name, job) in jobs {
        let job_name = job_name
            .as_str()
            .unwrap_or_else(|| panic!("{path} job names must be strings"));
        let job_context = format!("{path} job {job_name}");
        assert_environment_mapping_has_no_compiler_wrapper(&job_context, job.get("env"));

        if let Some(steps) = job.get("steps").and_then(Value::as_sequence) {
            for (index, step) in steps.iter().enumerate() {
                let step_context = format!("{job_context} step {index}");
                assert_environment_mapping_has_no_compiler_wrapper(&step_context, step.get("env"));
                if let Some(run) = step.get("run").and_then(Value::as_str) {
                    assert_run_script_has_no_compiler_wrapper(&step_context, run);
                }
            }
        }
    }
}

#[test]
fn release_workflows_reject_cargo_compiler_wrapper_authority() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/supply-chain.yml",
    ] {
        let workflow = read_repository_file(path);
        assert_workflow_has_no_compiler_wrapper(path, &workflow);
    }
}

#[test]
fn compiler_wrapper_contract_covers_cargo_and_rustc_override_variables() {
    for forbidden in FORBIDDEN_COMPILER_ENV {
        let yaml = format!(
            "env:\n  {forbidden}: /tmp/alternate\njobs:\n  build:\n    steps:\n      - run: cargo build --release --locked\n"
        );
        let yaml_result = std::panic::catch_unwind(|| {
            assert_workflow_has_no_compiler_wrapper("synthetic.yml", &yaml);
        });
        assert!(
            yaml_result.is_err(),
            "workflow YAML env must reject compiler authority {forbidden}"
        );

        let shell = format!("{forbidden}=/tmp/alternate cargo build --release --locked");
        let shell_result = std::panic::catch_unwind(|| {
            assert_run_script_has_no_compiler_wrapper("synthetic run", &shell);
        });
        assert!(
            shell_result.is_err(),
            "shell assignment must reject compiler authority {forbidden}"
        );
    }
}

#[test]
fn repository_does_not_introduce_cargo_config_compiler_authority() {
    for path in [".cargo/config.toml", ".cargo/config"] {
        assert!(
            !Path::new(path).exists(),
            "{path} can override build.rustc or compiler wrappers; govern repository Cargo config before adding it"
        );
    }
}
