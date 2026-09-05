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

/// Pushes one completed shell word into its current command segment.
fn flush_word(word: &mut String, segment: &mut Vec<String>) {
    if !word.is_empty() {
        segment.push(std::mem::take(word));
    }
}

/// Pushes one completed command segment into the parsed command stream.
fn flush_segment(segment: &mut Vec<String>, segments: &mut Vec<Vec<String>>) {
    if !segment.is_empty() {
        segments.push(std::mem::take(segment));
    }
}

/// Splits shell commands on unquoted control operators, including operators without surrounding spaces.
fn command_segments(script: &str) -> Vec<Vec<String>> {
    let normalized = script.replace("\\\r\n", "").replace("\\\n", "");
    let mut segments = Vec::new();
    let mut segment = Vec::new();
    let mut word = String::new();
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    let mut comment = false;

    for character in normalized.chars() {
        if comment {
            if character == '\n' {
                comment = false;
                flush_word(&mut word, &mut segment);
                flush_segment(&mut segment, &mut segments);
            }
            continue;
        }

        if escaped {
            word.push(character);
            escaped = false;
            continue;
        }

        if !single_quoted && character == '\\' {
            escaped = true;
            continue;
        }
        if !double_quoted && character == '\'' {
            single_quoted = !single_quoted;
            continue;
        }
        if !single_quoted && character == '"' {
            double_quoted = !double_quoted;
            continue;
        }

        if !single_quoted && !double_quoted {
            if character == '#' && word.is_empty() {
                comment = true;
                continue;
            }
            if character.is_whitespace() {
                flush_word(&mut word, &mut segment);
                if character == '\n' {
                    flush_segment(&mut segment, &mut segments);
                }
                continue;
            }
            if matches!(character, ';' | '|' | '&' | '(' | ')') {
                flush_word(&mut word, &mut segment);
                flush_segment(&mut segment, &mut segments);
                continue;
            }
        }

        word.push(character);
    }

    assert!(
        !single_quoted && !double_quoted && !escaped,
        "shell authority contract requires balanced quoting and escaping"
    );
    flush_word(&mut word, &mut segment);
    flush_segment(&mut segment, &mut segments);
    segments
}

/// Returns the executable basename used for security-sensitive shell command matching.
fn command_basename(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// Detects compiler-selection environment authority in assignment prefixes or env/export commands.
fn segment_has_forbidden_compiler_authority(segment: &[String]) -> bool {
    let mut index = 0;
    while let Some(word) = segment.get(index) {
        let Some(name) = assignment_name(word) else {
            break;
        };
        if FORBIDDEN_COMPILER_AUTHORITIES.contains(&name) {
            return true;
        }
        index += 1;
    }

    let Some(command) = segment.get(index) else {
        return false;
    };
    let mut command = command_basename(command);
    index += 1;

    if command == "command" {
        let Some(next) = segment.get(index) else {
            return false;
        };
        command = command_basename(next);
        index += 1;
    }

    if matches!(command, "env" | "export") {
        return segment[index..].iter().any(|word| {
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
fn repository_release_shell_control_contract() {
    for path in [".github/workflows/ci.yml", ".github/workflows/supply-chain.yml"] {
        let workflow = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("required workflow {path} is missing: {error}"));
        for script in workflow_run_scripts(&workflow) {
            assert_no_hidden_compiler_authority(path, &script);
        }
    }

    let dockerfile = fs::read_to_string("Dockerfile")
        .unwrap_or_else(|error| panic!("required Dockerfile is missing: {error}"));
    assert_no_hidden_compiler_authority("Dockerfile", &dockerfile);
}

#[test]
fn shell_control_operator_cannot_hide_compiler_assignment() {
    for script in [
        "true;RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "true&&CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "false||RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked",
        "printf ok|RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "command env RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked",
        "export CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0; cargo build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", script);
        });
        assert!(
            result.is_err(),
            "control operator must not hide compiler authority: {script}"
        );
    }
}

#[test]
fn shell_control_operator_cannot_hide_toolchain_command_authority() {
    for script in [
        "true;cargo +1.98.0 build --release --locked",
        "true&&rustup default 1.98.0",
        "false||rustup override set 1.98.0",
        "printf ok|rustup toolchain install 1.98.0 --profile minimal",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", script);
        });
        assert!(
            result.is_err(),
            "control operator must not hide toolchain command authority: {script}"
        );
    }
}

#[test]
fn quoted_text_and_comments_do_not_create_compiler_authority() {
    for script in [
        "echo 'RUSTC=/tmp/rustc-1.98.0'",
        "printf '%s\\n' \"CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0\"",
        "# RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked\necho ok",
    ] {
        assert_no_hidden_compiler_authority("synthetic shell", script);
    }
}
