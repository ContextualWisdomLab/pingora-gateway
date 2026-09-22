//! Fail-closed contract for sourced shell-code injection in routed load evidence.
//!
//! The routed evidence shell already rejects direct `export`, `declare`, `env`, `eval`, PATH
//! mutation, Bash startup hooks, and shell-function shadowing. Bash `source`/`.` can still execute
//! a generated helper in the current shell, letting dynamically reconstructed environment names or
//! slash-named functions mutate k6/candidate resolution while the canonical measurement lines stay
//! unchanged. The routed evidence step therefore admits no active `source`/`.` invocation, including
//! `builtin`, `command`, and backslash-qualified forms.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn active_lines(run: &str) -> impl Iterator<Item = &str> {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

fn invokes_source_builtin(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    match tokens.next() {
        Some("source" | "." | "\\source" | "\\.") => true,
        Some("builtin" | "command") => matches!(tokens.next(), Some("source" | ".")),
        _ => false,
    }
}

fn routed_shell_does_not_source_external_code(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(steps) = document
        .get("jobs")
        .and_then(|jobs| jobs.get(LOAD_JOB))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
    else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    routed
        .get("run")
        .and_then(Value::as_str)
        .is_some_and(|run| active_lines(run).all(|line| !invokes_source_builtin(line)))
}

#[test]
fn live_routed_evidence_does_not_source_external_shell_code() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_shell_does_not_source_external_code(&source),
        "routed evidence must not execute sourced helper code in the measurement shell"
    );
}

#[test]
fn source_can_reconstruct_a_threshold_bypass_without_literal_k6_option() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          prefix=K6
          name="${prefix}_NO_THRESHOLDS"
          helper=/tmp/routed-env
          printf 'export %s=true\n' "$name" > "$helper"
          source "$helper"
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_does_not_source_external_code(source));
}

#[test]
fn dot_builtin_can_load_the_same_mutation() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          helper=/tmp/routed-env
          . "$helper"
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_does_not_source_external_code(source));
}

#[test]
fn builtin_and_command_qualified_source_are_rejected() {
    for invocation in [
        "builtin source /tmp/routed-env",
        "builtin . /tmp/routed-env",
        "command source /tmp/routed-env",
        "command . /tmp/routed-env",
        "\\source /tmp/routed-env",
        "\\. /tmp/routed-env",
    ] {
        let source = format!(
            "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        shell: bash\n        run: |\n          {invocation}\n"
        );
        assert!(
            !routed_shell_does_not_source_external_code(&source),
            "qualified source form must remain outside routed evidence: {invocation}"
        );
    }
}

#[test]
fn comments_and_relative_executables_are_not_source_invocations() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          # source and . are forbidden as shell-code loaders here.
          ./target/release/cwl-pingora-pg-erd-migration --help
"#;
    assert!(routed_shell_does_not_source_external_code(source));
}
