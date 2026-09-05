//! Fail-closed acceptance for compiler authority hidden in shell command substitution.

#[path = "support/toolchain_command_substitution.rs"]
mod toolchain_command_substitution_support;

use serde_yaml::Value;
use std::fs;
use toolchain_command_substitution_support::assert_no_hidden_compiler_authority as assert_no_hidden_dollar_compiler_authority;

/// Returns every active legacy backquote command-substitution body outside single quotes.
///
/// Bash documents the first unescaped backquote as the terminator of the legacy form. Backquotes
/// remain active inside double quotes, while single quotes and an outer backslash keep them literal.
fn legacy_command_substitution_bodies(shell: &str) -> Vec<String> {
    let mut bodies = Vec::new();
    let mut current: Option<String> = None;
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;

    for character in shell.chars() {
        if let Some(body) = current.as_mut() {
            if escaped {
                body.push(character);
                escaped = false;
                continue;
            }
            if character == '\\' {
                body.push(character);
                escaped = true;
                continue;
            }
            if character == '`' {
                bodies.push(current.take().expect("legacy command body must exist"));
                continue;
            }
            body.push(character);
            continue;
        }

        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && !single_quoted {
            escaped = true;
            continue;
        }
        if character == '\'' && !double_quoted {
            single_quoted = !single_quoted;
            continue;
        }
        if character == '"' && !single_quoted {
            double_quoted = !double_quoted;
            continue;
        }
        if character == '`' && !single_quoted {
            current = Some(String::new());
        }
    }

    assert!(
        current.is_none(),
        "compiler-authority shell contract requires terminated legacy command substitution"
    );
    bodies
}

/// Applies the same compiler-authority analysis to POSIX `$()` and Bash's legacy backquote form.
fn assert_no_hidden_compiler_authority(context: &str, shell: &str) {
    assert_no_hidden_dollar_compiler_authority(context, shell);

    for body in legacy_command_substitution_bodies(shell) {
        let normalized = format!("$({body})");
        assert_no_hidden_dollar_compiler_authority(context, &normalized);
    }
}

/// Returns every shell script attached to a direct GitHub Actions step.
fn workflow_run_scripts(path: &str) -> Vec<String> {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"));
    let document: Value = serde_yaml::from_str(&source)
        .unwrap_or_else(|error| panic!("workflow {path} must parse as YAML: {error}"));
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("workflow {path} must define a top-level jobs mapping"));

    let mut scripts = Vec::new();
    for (job_id, job) in jobs {
        let job_id = job_id
            .as_str()
            .unwrap_or_else(|| panic!("workflow {path} job identity must be a string"));
        let steps = job
            .get("steps")
            .and_then(Value::as_sequence)
            .unwrap_or_else(|| panic!("workflow {path} job {job_id} must define steps"));
        for step in steps {
            if let Some(run) = step.get("run").and_then(Value::as_str) {
                scripts.push(run.to_owned());
            }
        }
    }
    scripts
}

/// Returns shell-form Docker `RUN` instructions with line continuations joined.
fn docker_run_commands(source: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current: Option<String> = None;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if let Some(command) = current.as_mut() {
            let continued = trimmed.ends_with('\\');
            let fragment = trimmed.strip_suffix('\\').unwrap_or(trimmed).trim_end();
            if !command.is_empty() && !fragment.is_empty() {
                command.push(' ');
            }
            command.push_str(fragment);
            if !continued {
                commands.push(current.take().expect("continued RUN command must exist"));
            }
            continue;
        }

        let Some(after_run) = trimmed.strip_prefix("RUN") else {
            continue;
        };
        if !after_run.chars().next().is_some_and(char::is_whitespace) {
            continue;
        }
        let rest = after_run.trim_start();
        let continued = rest.ends_with('\\');
        let fragment = rest.strip_suffix('\\').unwrap_or(rest).trim_end();
        if continued {
            current = Some(fragment.to_owned());
        } else {
            commands.push(fragment.to_owned());
        }
    }

    assert!(
        current.is_none(),
        "Dockerfile RUN continuation must terminate before EOF"
    );
    commands
}

/// Production release workflows and the OCI build must not hide compiler selection inside shell substitution.
#[test]
fn release_paths_reject_hidden_compiler_authority_in_command_substitution() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/supply-chain.yml",
    ] {
        for script in workflow_run_scripts(path) {
            assert_no_hidden_compiler_authority(path, &script);
        }
    }

    let dockerfile = fs::read_to_string("Dockerfile")
        .expect("required repository evidence Dockerfile is missing");
    for command in docker_run_commands(&dockerfile) {
        assert_no_hidden_compiler_authority("Dockerfile RUN", &command);
    }
}

/// A verified default compiler must not be bypassable from executable command substitution.
#[test]
fn command_substitution_guard_rejects_alternate_compiler_authority() {
    for shell in [
        "echo $(RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked)",
        "printf '%s\\n' \"$(RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked)\"",
        "echo $(CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked)",
        "echo $(cargo +1.98.0 build --release --locked)",
        "echo $(rustup run 1.98.0 cargo build --release --locked)",
        "echo `rustup default 1.98.0`",
        "echo `cargo +1.98.0 build --release --locked`",
        "printf '%s\\n' \"prefix '`rustup default 1.98.0`' suffix\"",
        "printf '%s\\n' \"prefix '`cargo +1.98.0 build --release --locked`' suffix\"",
        "value=$(case x in x) RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked;; esac)",
        "version=$(git rev-parse HEAD); CARGO=cargo; RUSTUP_TOOLCHAIN=1.98.0 \"$CARGO\" build --release --locked",
        "version=$(git rev-parse HEAD); CARGO=cargo; RUSTUP_TOOLCHAIN=1.98.0 \"${CARGO:?}\" build --release --locked",
        "version=$(git rev-parse HEAD); CARGO=cargo; RUSTUP_TOOLCHAIN=1.98.0 \"${CARGO:-cargo}\" build --release --locked",
        "version=$(git rev-parse HEAD); RUSTUP_TOOLCHAIN=1.98.0 \"${CARGO=cargo}\" build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", shell);
        });
        assert!(
            result.is_err(),
            "command substitution must not hide alternate compiler authority: {shell}"
        );
    }
}

/// Ordinary command substitution remains available when it does not select Cargo compiler authority.
#[test]
fn command_substitution_guard_allows_non_compiler_subshells() {
    for shell in [
        "version=$(git rev-parse HEAD)",
        "artifact=$(cargo metadata --no-deps --format-version 1)",
        "printf '%s\\n' \"$(uname -m)\"",
        "literal=$(printf '%s' \"(not syntax)\")",
        "nested=$(printf '%s' \"$(uname -m)\")",
        "legacy=`git rev-parse HEAD`",
        "printf '%s\\n' '`rustup default 1.98.0`'",
        "printf '%s\\n' \\`rustup default 1.98.0\\`",
        "printf '%s\\n' '$(RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked)'",
        "printf '%s\\n' \\$(RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked)",
        "version=$(git rev-parse HEAD); CARGO=cargo; RUSTUP_TOOLCHAIN=1.98.0 printf '%s\\n' '${CARGO:?}'",
        "version=$(git rev-parse HEAD); CARGO=cargo; RUSTUP_TOOLCHAIN=1.98.0 printf '%s\\n' \\${CARGO:?}",
    ] {
        assert_no_hidden_compiler_authority("synthetic shell", shell);
    }
}
