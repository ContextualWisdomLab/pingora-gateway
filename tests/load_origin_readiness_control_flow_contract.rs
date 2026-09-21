//! Fail-closed control-flow contract for measured-origin readiness evidence.
//!
//! The existing workflow contract binds evidence to one unconditional `load-contract` step and
//! filters comments, quoted data, heredocs, and multiline string payloads. This companion contract
//! closes a different shell-semantic gap: canonical command text must not earn readiness credit
//! when it appears only inside an outer branch or an uncalled function.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

#[derive(Debug)]
struct ShellLine {
    text: String,
    if_depth: usize,
    loop_depth: usize,
    function_depth: usize,
    case_depth: usize,
}

fn load_job_scripts(source: &str) -> Option<Vec<String>> {
    let document: Value = serde_yaml::from_str(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;

    Some(
        steps
            .iter()
            .filter(|step| step.get("if").is_none())
            .filter_map(|step| step.get("run").and_then(Value::as_str))
            .map(str::to_string)
            .collect(),
    )
}

fn strip_shell_comments(line: &str) -> String {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    let mut visible = String::new();

    for character in line.chars() {
        if escaped {
            visible.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && !single_quoted {
            visible.push(character);
            escaped = true;
            continue;
        }
        if character == '\'' && !double_quoted {
            single_quoted = !single_quoted;
            visible.push(character);
            continue;
        }
        if character == '"' && !single_quoted {
            double_quoted = !double_quoted;
            visible.push(character);
            continue;
        }
        if character == '#' && !single_quoted && !double_quoted {
            break;
        }
        visible.push(character);
    }

    visible
}

fn quote_state_after(line: &str, mut single_quoted: bool, mut double_quoted: bool) -> (bool, bool) {
    let mut escaped = false;
    for character in line.chars() {
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
        if character == '#' && !single_quoted && !double_quoted {
            break;
        }
    }
    (single_quoted, double_quoted)
}

fn heredoc_terminator(line: &str) -> Option<String> {
    let marker = line.find("<<")?;
    let mut remainder = line[marker + 2..].trim_start();
    if let Some(stripped) = remainder.strip_prefix('-') {
        remainder = stripped.trim_start();
    }
    if remainder.is_empty() {
        return None;
    }

    let first = remainder.chars().next()?;
    if first == '\'' || first == '"' {
        let quoted = &remainder[first.len_utf8()..];
        let end = quoted.find(first)?;
        let terminator = &quoted[..end];
        return (!terminator.is_empty()).then(|| terminator.to_string());
    }

    let terminator = remainder
        .split(|character: char| character.is_whitespace() || character == ';')
        .next()?;
    (!terminator.is_empty()).then(|| terminator.to_string())
}

fn shell_lines(script: &str) -> Option<Vec<ShellLine>> {
    let mut lines = Vec::new();
    let mut heredoc_end: Option<String> = None;
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut if_depth = 0usize;
    let mut loop_depth = 0usize;
    let mut function_depth = 0usize;
    let mut case_depth = 0usize;

    for raw_line in script.lines() {
        let trimmed = raw_line.trim();
        if let Some(terminator) = heredoc_end.as_deref() {
            if trimmed == terminator {
                heredoc_end = None;
            }
            continue;
        }

        let started_inside_multiline_quote = single_quoted || double_quoted;
        (single_quoted, double_quoted) =
            quote_state_after(raw_line, single_quoted, double_quoted);
        if started_inside_multiline_quote {
            continue;
        }

        let visible = strip_shell_comments(raw_line);
        let visible = visible.trim();
        if visible.is_empty() {
            continue;
        }
        if let Some(terminator) = heredoc_terminator(visible) {
            heredoc_end = Some(terminator);
        }

        if visible == "fi" {
            if_depth = if_depth.checked_sub(1)?;
        } else if visible == "done" {
            loop_depth = loop_depth.checked_sub(1)?;
        } else if visible == "esac" {
            case_depth = case_depth.checked_sub(1)?;
        } else if visible == "}" && function_depth > 0 {
            function_depth -= 1;
        }

        lines.push(ShellLine {
            text: visible.to_string(),
            if_depth,
            loop_depth,
            function_depth,
            case_depth,
        });

        let function_opener = (visible.contains("()") || visible.starts_with("function "))
            && visible.ends_with('{');
        if function_opener {
            function_depth += 1;
        } else if visible.starts_with("if ") && visible.ends_with("; then") {
            if_depth += 1;
        } else if (visible.starts_with("for ")
            || visible.starts_with("while ")
            || visible.starts_with("until "))
            && visible.ends_with("; do")
        {
            loop_depth += 1;
        } else if visible.starts_with("case ") && visible.ends_with(" in") {
            case_depth += 1;
        }
    }

    (heredoc_end.is_none()
        && !single_quoted
        && !double_quoted
        && if_depth == 0
        && loop_depth == 0
        && function_depth == 0
        && case_depth == 0)
        .then_some(lines)
}

fn unguarded(line: &ShellLine) -> bool {
    line.if_depth == 0 && line.function_depth == 0 && line.case_depth == 0
}

fn script_proves_readiness(source: &str) -> bool {
    let Some(lines) = shell_lines(source) else {
        return false;
    };

    let position = |predicate: &dyn Fn(&ShellLine) -> bool| lines.iter().position(predicate);

    let Some(origin_start) = position(&|line| {
        unguarded(line)
            && line.loop_depth == 0
            && line.text == "/tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &"
    }) else {
        return false;
    };
    let Some(origin_ready) = position(&|line| {
        unguarded(line)
            && line.loop_depth == 1
            && line.text.starts_with("if curl ")
            && line
                .text
                .contains("http://127.0.0.1:18081/fixture-ready")
            && line.text.ends_with("; then")
    }) else {
        return false;
    };
    let Some(origin_liveness) = position(&|line| {
        unguarded(line)
            && line.loop_depth == 1
            && line.text == "if ! kill -0 \"$upstream_pid\" 2>/dev/null; then"
    }) else {
        return false;
    };
    let Some(gateway_start) = position(&|line| {
        unguarded(line)
            && line.loop_depth == 0
            && line
                .text
                .starts_with("target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml")
    }) else {
        return false;
    };
    let Some(measured_traffic) = position(&|line| {
        unguarded(line)
            && line.loop_depth == 0
            && line
                .text
                .starts_with("GATEWAY_URL=http://127.0.0.1:18080 k6 run")
    }) else {
        return false;
    };

    origin_start < origin_ready
        && origin_ready < origin_liveness
        && origin_liveness < gateway_start
        && gateway_start < measured_traffic
}

fn readiness_contract_accepts(source: &str) -> bool {
    load_job_scripts(source)
        .is_some_and(|scripts| scripts.iter().any(|script| script_proves_readiness(script)))
}

#[test]
fn live_workflow_has_control_flow_safe_origin_readiness_evidence() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        readiness_contract_accepts(&source),
        "load-contract must retain executable origin readiness evidence"
    );
}

#[test]
fn outer_false_branch_must_not_manufacture_readiness_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          if false; then
            /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
            for _ in $(seq 1 80); do
              if curl --fail http://127.0.0.1:18081/fixture-ready; then
                break
              fi
              if ! kill -0 "$upstream_pid" 2>/dev/null; then
                exit 1
              fi
            done
            target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
            GATEWAY_URL=http://127.0.0.1:18080 k6 run
          fi
"#;

    assert!(
        !readiness_contract_accepts(source),
        "canonical readiness text inside an outer false branch is not executable evidence"
    );
}

#[test]
fn uncalled_function_must_not_manufacture_readiness_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          archived_readiness() {
            /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
            for _ in $(seq 1 80); do
              if curl --fail http://127.0.0.1:18081/fixture-ready; then
                break
              fi
              if ! kill -0 "$upstream_pid" 2>/dev/null; then
                exit 1
              fi
            done
            target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
            GATEWAY_URL=http://127.0.0.1:18080 k6 run
          }
          echo archived-only
"#;

    assert!(
        !readiness_contract_accepts(source),
        "an uncalled shell function is archived behavior, not executed readiness evidence"
    );
}

#[test]
fn zero_iteration_loop_must_not_hide_the_process_sequence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          for _ in; do
            /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
            if curl --fail http://127.0.0.1:18081/fixture-ready; then
              break
            fi
            if ! kill -0 "$upstream_pid" 2>/dev/null; then
              exit 1
            fi
            target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
            GATEWAY_URL=http://127.0.0.1:18080 k6 run
          done
"#;

    assert!(
        !readiness_contract_accepts(source),
        "origin start, gateway start and measured traffic must not be credited from a zero-iteration loop"
    );
}
