//! Regression contract for shell control operators around compiler authority.

use serde_yaml::Value;
use std::fs;

const FIXED_TOOLCHAIN: &str = "1.98.1";
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

/// Reads executable shell-form Docker `RUN` bodies after continuation normalization.
fn docker_run_scripts(dockerfile: &str) -> Vec<String> {
    let normalized = dockerfile.replace("\\\r\n", "").replace("\\\n", "");
    normalized
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let boundary = trimmed.find(char::is_whitespace)?;
            if !trimmed[..boundary].eq_ignore_ascii_case("RUN") {
                return None;
            }
            Some(trimmed[boundary..].trim_start().to_owned())
        })
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
        if !single_quoted && character == '`' {
            panic!("shell authority contract does not admit legacy backtick command substitution");
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

/// Resolves an executable after the shell `command` builtin without mistaking its options for the command.
fn unwrap_command_builtin<'a>(
    segment: &'a [String],
    mut index: usize,
    command: &'a str,
) -> Option<(&'a str, usize)> {
    if command != "command" {
        return Some((command, index));
    }

    loop {
        match segment.get(index).map(String::as_str) {
            Some("-p") => index += 1,
            Some("--") => {
                index += 1;
                break;
            }
            Some("-v" | "-V") => return None,
            Some(option) if option.starts_with('-') => return None,
            _ => break,
        }
    }

    let command = command_basename(segment.get(index)?);
    Some((command, index + 1))
}

/// Checks only GNU `env`'s own option/assignment prefix and stops at the child command boundary.
fn env_prefix_violates_compiler_contract(arguments: &[String]) -> bool {
    let mut index = 0;
    let mut options_active = true;

    while let Some(argument) = arguments.get(index).map(String::as_str) {
        if options_active {
            if argument == "--" {
                options_active = false;
                index += 1;
                continue;
            }
            if argument == "-" {
                options_active = false;
                index += 1;
                continue;
            }

            if let Some(long_option) = argument.strip_prefix("--") {
                let (name, has_attached_argument) = long_option
                    .split_once('=')
                    .map_or((long_option, false), |(name, _)| (name, true));

                match name {
                    "split-string" => return true,
                    "null"
                    | "ignore-environment"
                    | "default-signal"
                    | "ignore-signal"
                    | "block-signal"
                    | "list-signal-handling"
                    | "debug"
                    | "help"
                    | "version" => {
                        index += 1;
                        continue;
                    }
                    "argv0" | "unset" | "chdir" => {
                        if !has_attached_argument {
                            index += 1;
                            if index >= arguments.len() {
                                return true;
                            }
                        }
                        index += 1;
                        continue;
                    }
                    _ => return true,
                }
            }

            if let Some(short_options) = argument.strip_prefix('-') {
                let mut consumes_next_argument = false;
                for (offset, option) in short_options.char_indices() {
                    match option {
                        'S' => return true,
                        '0' | 'i' | 'v' => {}
                        'a' | 'u' | 'C' => {
                            consumes_next_argument = offset + option.len_utf8() == short_options.len();
                            break;
                        }
                        _ => return true,
                    }
                }

                if consumes_next_argument {
                    index += 1;
                    if index >= arguments.len() {
                        return true;
                    }
                }
                index += 1;
                continue;
            }

            options_active = false;
        }

        if let Some(name) = assignment_name(argument) {
            if FORBIDDEN_COMPILER_AUTHORITIES.contains(&name) {
                return true;
            }
            index += 1;
            continue;
        }

        return false;
    }

    false
}

/// Detects alternate compiler authority in assignment prefixes or explicit selector commands.
fn segment_has_alternate_compiler_authority(segment: &[String]) -> bool {
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
    let command = command_basename(command);
    index += 1;
    let Some((command, index)) = unwrap_command_builtin(segment, index, command) else {
        return false;
    };

    if command == "env" && env_prefix_violates_compiler_contract(&segment[index..]) {
        return true;
    }

    if command == "export"
        && segment[index..].iter().any(|word| {
            assignment_name(word)
                .is_some_and(|name| FORBIDDEN_COMPILER_AUTHORITIES.contains(&name))
        })
    {
        return true;
    }

    if command == "cargo" {
        return segment
            .get(index)
            .is_some_and(|argument| argument.starts_with('+') && argument.len() > 1);
    }

    if command == "rustup" {
        return match segment.get(index).map(String::as_str) {
            Some("default") => {
                segment.get(index + 1).map(String::as_str) != Some(FIXED_TOOLCHAIN)
            }
            Some("toolchain") if segment.get(index + 1).map(String::as_str) == Some("install") => {
                segment.get(index + 2).map(String::as_str) != Some(FIXED_TOOLCHAIN)
            }
            Some("override" | "run") => true,
            _ => false,
        };
    }

    false
}

/// Requires security-sensitive compiler authority to remain visible across shell command boundaries.
fn assert_no_hidden_compiler_authority(context: &str, script: &str) {
    assert!(
        !command_segments(script)
            .iter()
            .any(|segment| segment_has_alternate_compiler_authority(segment)),
        "{context} must not hide alternate compiler authority behind shell control operators"
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
    for script in docker_run_scripts(&dockerfile) {
        assert_no_hidden_compiler_authority("Dockerfile RUN", &script);
    }
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
        "command -p env RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "command -p env CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "command -p env RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked",
        "echo `RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked`",
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
        "command cargo +nightly build --release --locked",
        "command rustup run 1.98.0 cargo build --release --locked",
        "command -p cargo +1.98.0 build --release --locked",
        "command -p rustup default 1.98.0",
        "command -p -- cargo +nightly build --release --locked",
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
fn env_split_string_cannot_hide_compiler_authority() {
    for script in [
        "env --split-string='RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked'",
        "command -p env --split-string='RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked'",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", script);
        });
        assert!(
            result.is_err(),
            "env split-string must not hide compiler authority: {script}"
        );
    }
}

#[test]
fn bundled_env_split_string_cannot_hide_compiler_authority() {
    for script in [
        "env -iS 'RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked'",
        "env -iS 'CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked'",
        "env -iS 'RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked'",
        "command -p env -iS 'RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked'",
        "command -p env -iS 'CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked'",
        "command -p env -iS 'RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked'",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", script);
        });
        assert!(
            result.is_err(),
            "bundled env split-string must not hide compiler authority: {script}"
        );
    }
}

#[test]
fn env_end_of_options_cannot_hide_compiler_assignment_prefix() {
    for script in [
        "env -- RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked",
        "command -p env -- RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
        "env -- CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_hidden_compiler_authority("synthetic shell", script);
        });
        assert!(
            result.is_err(),
            "env -- must end option parsing without hiding assignment operands: {script}"
        );
    }
}

#[test]
fn fixed_toolchain_commands_quoted_text_and_comments_remain_allowed() {
    for script in [
        "rustup toolchain install 1.98.1 --profile minimal; rustup default 1.98.1",
        "command -p rustup default 1.98.1",
        "command -v rustup",
        "command -V cargo",
        "env -i PATH=/usr/bin /usr/bin/printf ok",
        "env -i /usr/bin/printf -S",
        "command -p env -i /usr/bin/printf -S",
        "env -i /usr/bin/printf RUSTUP_TOOLCHAIN=1.98.0",
        "command -p env -i /usr/bin/printf RUSTC=/tmp/rustc-1.98.0",
        "env -- /usr/bin/printf RUSTUP_TOOLCHAIN=1.98.0",
        "echo 'RUSTC=/tmp/rustc-1.98.0'",
        "echo '`literal`'",
        "printf '%s\\n' \"CARGO_BUILD_RUSTC=/tmp/rustc-1.98.0\"",
        "# RUSTUP_TOOLCHAIN=1.98.0 cargo build --release --locked\necho ok",
    ] {
        assert_no_hidden_compiler_authority("synthetic shell", script);
    }
}
