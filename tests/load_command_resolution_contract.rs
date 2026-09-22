//! Fail-closed contract for routed-load executable resolution.
//!
//! The load receipt is only meaningful when the routed step cannot retarget the `k6` lookup
//! through runner-local command-search state. GitHub Actions YAML `env.PATH` overrides are already
//! rejected elsewhere. Within the routed Bash shell, the evidence path also rejects executable
//! `PATH` references and any executable `k6` reference other than the canonical terminal command,
//! covering shell-function, alias, and command-hash shadowing without trying to interpret them.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_K6: &str = "k6 run --quiet tests/load/pg_erd_gateway_smoke.js";

fn env_overrides_path(node: &Value) -> bool {
    node.get("env").and_then(|env| env.get("PATH")).is_some()
}

fn is_shell_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn contains_shell_identifier(line: &str, identifier: &str) -> bool {
    line.match_indices(identifier).any(|(index, _)| {
        let bytes = line.as_bytes();
        let start_boundary = index == 0 || !is_shell_identifier_byte(bytes[index - 1]);
        let end = index + identifier.len();
        let end_boundary = end == bytes.len() || !is_shell_identifier_byte(bytes[end]);
        start_boundary && end_boundary
    })
}

fn routed_shell_rebinds_command_resolution(run: &str) -> bool {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with('#'))
        .any(|line| {
            contains_shell_identifier(line, "PATH")
                || (contains_shell_identifier(line, "k6") && line.trim() != ROUTED_K6)
        })
}

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn routed_command_resolution_is_stable(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if env_overrides_path(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if env_overrides_path(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    if env_overrides_path(routed) {
        return false;
    }

    routed.get("run").and_then(Value::as_str).is_some_and(|run| {
        run.contains(ROUTED_K6) && !routed_shell_rebinds_command_resolution(run)
    })
}

#[test]
fn live_routed_load_keeps_command_resolution_stable() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_command_resolution_is_stable(&source),
        "routed load evidence must not retarget k6 executable resolution"
    );
}

#[test]
fn same_shell_path_override_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          mkdir -p /tmp/evidence-shims
          printf '#!/bin/sh\nprintf "{}" > k6-pg-erd-summary.json\n' > /tmp/evidence-shims/k6
          chmod +x /tmp/evidence-shims/k6
          PATH=/tmp/evidence-shims:$PATH
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_command_resolution_is_stable(source),
        "a same-shell PATH override can redirect the canonical k6 command to a shim while preserving command text"
    );
}

#[test]
fn exported_same_shell_path_override_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          export PATH=/tmp/evidence-shims:$PATH
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_command_resolution_is_stable(source));
}

#[test]
fn same_shell_k6_function_shadow_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          k6() {
            printf '{}' > k6-pg-erd-summary.json
          }
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_command_resolution_is_stable(source),
        "a same-shell k6 function shadows the installed executable while preserving canonical command text"
    );
}

#[test]
fn same_shell_k6_alias_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          shopt -s expand_aliases
          alias k6='/tmp/evidence-k6'
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_command_resolution_is_stable(source));
}

#[test]
fn same_shell_k6_hash_override_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          hash -p /tmp/evidence-k6 k6
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_command_resolution_is_stable(source));
}

#[test]
fn comments_that_name_path_or_k6_do_not_mutate_command_resolution() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          # PATH remains runner-owned and k6 remains the installed binary.
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(routed_command_resolution_is_stable(source));
}
