//! Fail-closed contract for routed-load executable resolution.
//!
//! The routed receipt is attributable to the checksum-pinned k6 install only when the evidence
//! step invokes that installed path directly. A bare `k6` name is subject to Bash function, alias,
//! hash-table, and `PATH` resolution inside the same multi-line `run` shell. Bash also permits a
//! function name containing `/`, so even an absolute command token can be shadowed when that same
//! path is bound as a shell function first. Dynamic `eval` can reconstruct the same binding without
//! repeating the literal path in source. The evidence lane therefore requires one active
//! `/usr/local/bin/k6` measurement invocation, permits one exact byte-identity comparison against
//! the freshly rederived supplier binary, rejects executable `PATH` references, and rejects dynamic
//! evaluation in the routed shell.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const K6_PATH: &str = "/usr/local/bin/k6";
const ROUTED_K6: &str = "/usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js";
const K6_PROVENANCE_CMP: &str =
    "cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6";

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

fn active_shell_lines(run: &str) -> impl Iterator<Item = &str> {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with('#'))
}

fn routed_shell_references_path(run: &str) -> bool {
    active_shell_lines(run).any(|line| contains_shell_identifier(line, "PATH"))
}

fn routed_shell_uses_dynamic_evaluation(run: &str) -> bool {
    active_shell_lines(run).any(|line| contains_shell_identifier(line, "eval"))
}

fn routed_shell_has_single_k6_invocation_path(run: &str) -> bool {
    active_shell_lines(run)
        .filter(|line| *line != K6_PROVENANCE_CMP)
        .map(|line| line.match_indices(K6_PATH).count())
        .sum::<usize>()
        == 1
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
        run.contains(ROUTED_K6)
            && routed_shell_has_single_k6_invocation_path(run)
            && !routed_shell_references_path(run)
            && !routed_shell_uses_dynamic_evaluation(run)
    })
}

#[test]
fn live_routed_load_uses_the_pinned_k6_path() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_command_resolution_is_stable(&source),
        "routed load evidence must invoke the checksum-pinned k6 installation by one unshadowed absolute measurement path"
    );
}

#[test]
fn bare_k6_invocation_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_command_resolution_is_stable(source),
        "bare k6 is subject to same-shell function, alias, hash and PATH resolution"
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
          PATH=/tmp/evidence-shims:$PATH
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_command_resolution_is_stable(source),
        "same-shell PATH mutation can also retarget readiness and process-liveness commands"
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
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_command_resolution_is_stable(source));
}

#[test]
fn shell_function_named_k6_cannot_replace_absolute_measurement_path() {
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
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(routed_command_resolution_is_stable(source));
}

#[test]
fn canonical_provenance_comparison_does_not_count_as_a_second_invocation() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(routed_command_resolution_is_stable(source));
}

#[test]
fn noncanonical_second_k6_path_reference_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          test -x /usr/local/bin/k6
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_command_resolution_is_stable(source));
}

#[test]
fn slash_named_shell_function_must_not_replace_absolute_measurement_path() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          function /usr/local/bin/k6 {
            printf '{}' > k6-pg-erd-summary.json
          }
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_command_resolution_is_stable(source),
        "Bash permits a function name containing slashes, so an absolute command token alone does not prove external executable resolution"
    );
}

#[test]
fn eval_reconstructed_slash_named_function_must_not_replace_absolute_measurement_path() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          reconstructed="/usr/local/bin/k""6"
          eval "function ${reconstructed} { printf '{}' > k6-pg-erd-summary.json; }"
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_command_resolution_is_stable(source),
        "dynamic evaluation can reconstruct the absolute function name without a second literal k6 path token"
    );
}

#[test]
fn comments_that_name_path_or_k6_do_not_mutate_command_resolution() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          # PATH remains runner-owned and k6 is invoked through the checksum-pinned install path.
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(routed_command_resolution_is_stable(source));
}
