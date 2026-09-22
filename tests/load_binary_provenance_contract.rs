//! Fail-closed provenance contract for the routed k6 executable.
//!
//! Verifying the supplier archive only at install time is insufficient for release evidence: a
//! later workflow step can replace `/usr/local/bin/k6` while preserving the canonical absolute
//! invocation and every workload/threshold check. Routed evidence therefore revalidates the
//! pinned supplier archive immediately before measurement, extracts a fresh private copy, proves
//! the installed executable is byte-identical to that derivation, and then invokes it without an
//! intervening command. The provenance utilities themselves must also retain their normal shell
//! resolution; function/alias/source mutations, imported Bash functions, or dynamic-loader
//! injection can otherwise turn the textual proof into a no-op.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const PROVENANCE_COMMANDS: [&str; 5] = ["rm", "mkdir", "sha256sum", "tar", "cmp"];
const ROUTED_PROVENANCE_TAIL: &str = r#"rm -rf /tmp/cwl-k6-routed
mkdir -p /tmp/cwl-k6-routed
echo "b5a8003c86f35f5cd5ceef1490312c48e587696c94d998cefc6d7b3b4cb1597d  /tmp/k6-v2.2.0-linux-amd64.tar.gz" | sha256sum --check --strict
tar -xzf /tmp/k6-v2.2.0-linux-amd64.tar.gz -C /tmp/cwl-k6-routed
cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js"#;

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn env_imports_bash_function(node: &Value) -> bool {
    node.get("env")
        .and_then(Value::as_mapping)
        .is_some_and(|env| {
            env.keys()
                .filter_map(Value::as_str)
                .any(|key| key.starts_with("BASH_FUNC_") && key.ends_with("%%"))
        })
}

fn env_mutates_dynamic_loader(node: &Value) -> bool {
    node.get("env")
        .and_then(Value::as_mapping)
        .is_some_and(|env| {
            env.keys()
                .filter_map(Value::as_str)
                .any(|key| key.starts_with("LD_"))
        })
}

fn shell_function_defines(line: &str, command: &str) -> bool {
    let line = line.trim_start();
    line.starts_with(&format!("{command}()"))
        || line.starts_with(&format!("{command} ()"))
        || line.strip_prefix("function ").is_some_and(|rest| {
            rest == command
                || rest.starts_with(&format!("{command} "))
                || rest.starts_with(&format!("{command}("))
        })
}

fn shell_namespace_can_subvert_provenance(run: &str) -> bool {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with('#'))
        .any(|line| {
            line.contains("LD_")
                || line.starts_with("source ")
                || line.starts_with(". ")
                || line.starts_with("alias ")
                || line.starts_with("shopt ")
                || PROVENANCE_COMMANDS
                    .iter()
                    .any(|command| shell_function_defines(line, command))
        })
}

fn routed_binary_provenance_is_fresh(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if env_imports_bash_function(&document) || env_mutates_dynamic_loader(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if env_imports_bash_function(job) || env_mutates_dynamic_loader(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    if env_imports_bash_function(routed) || env_mutates_dynamic_loader(routed) {
        return false;
    }

    routed.get("run").and_then(Value::as_str).is_some_and(|run| {
        run.trim_end().ends_with(ROUTED_PROVENANCE_TAIL)
            && !shell_namespace_can_subvert_provenance(run)
    })
}

#[test]
fn live_routed_load_revalidates_k6_provenance_at_measurement_time() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_binary_provenance_is_fresh(&source),
        "routed release evidence must revalidate the pinned k6 archive and prove the invoked binary is byte-identical immediately before measurement"
    );
}

#[test]
fn mutable_global_install_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        run: |
          echo "b5a8003c86f35f5cd5ceef1490312c48e587696c94d998cefc6d7b3b4cb1597d  /tmp/k6-v2.2.0-linux-amd64.tar.gz" | sha256sum --check --strict
          sudo install -m 0755 /tmp/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
      - name: Replace admitted binary after verification
        run: sudo install -m 0755 /tmp/fake-k6 /usr/local/bin/k6
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_binary_provenance_is_fresh(source),
        "an absolute path does not preserve supplier provenance when the installed binary remains mutable after archive verification"
    );
}

#[test]
fn provenance_check_must_be_adjacent_to_measurement() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          rm -rf /tmp/cwl-k6-routed
          mkdir -p /tmp/cwl-k6-routed
          echo "b5a8003c86f35f5cd5ceef1490312c48e587696c94d998cefc6d7b3b4cb1597d  /tmp/k6-v2.2.0-linux-amd64.tar.gz" | sha256sum --check --strict
          tar -xzf /tmp/k6-v2.2.0-linux-amd64.tar.gz -C /tmp/cwl-k6-routed
          cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
          sudo install -m 0755 /tmp/fake-k6 /usr/local/bin/k6
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_binary_provenance_is_fresh(source),
        "a provenance check separated from measurement by an executable mutation is stale evidence"
    );
}

#[test]
fn shell_function_shadowing_cmp_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          cmp() {{ return 0; }}
          destination="/usr/local/bin/k""6"
          install -m 0755 /tmp/fake-k6 "$destination"
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(
        !routed_binary_provenance_is_fresh(&source),
        "a shell function can shadow the byte-comparison utility and falsely attest a replaced k6 binary"
    );
}

#[test]
fn sourced_shell_namespace_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          source /tmp/provenance-command-shims.sh
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(
        !routed_binary_provenance_is_fresh(&source),
        "sourcing mutable shell state before the provenance tail can replace the utilities that produce the evidence"
    );
}

#[test]
fn imported_bash_function_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        env:
          "BASH_FUNC_cmp%%": "() {{ return 0; }}"
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(
        !routed_binary_provenance_is_fresh(&source),
        "Bash can import functions from BASH_FUNC_*%% environment entries before the run script starts"
    );
}

#[test]
fn job_level_imported_bash_function_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    env:
      "BASH_FUNC_cmp%%": "() {{ return 0; }}"
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(!routed_binary_provenance_is_fresh(&source));
}

#[test]
fn dynamic_loader_injection_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
env:
  LD_PRELOAD: /tmp/evidence-preload.so
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(
        !routed_binary_provenance_is_fresh(&source),
        "dynamic-loader environment can alter provenance utilities before the routed evidence command executes"
    );
}

#[test]
fn job_level_dynamic_loader_injection_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    env:
      LD_LIBRARY_PATH: /tmp/evidence-libs
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(!routed_binary_provenance_is_fresh(&source));
}

#[test]
fn step_level_dynamic_loader_injection_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        env:
          LD_AUDIT: /tmp/evidence-audit.so
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(!routed_binary_provenance_is_fresh(&source));
}

#[test]
fn same_shell_dynamic_loader_injection_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          export LD_PRELOAD=/tmp/evidence-preload.so
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(!routed_binary_provenance_is_fresh(&source));
}

#[test]
fn custom_job_container_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    container: ghcr.io/example/untrusted-evidence-runtime:latest
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(
        !routed_binary_provenance_is_fresh(&source),
        "a job container can replace the process environment and provenance utilities while preserving canonical run text"
    );
}

#[test]
fn canonical_provenance_tail_is_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(routed_binary_provenance_is_fresh(&source));
}
