//! Fail-closed PATH isolation for routed load evidence.
//!
//! GitHub's Ubuntu runner PATH includes user-home directories before system directories. An earlier
//! step can therefore place a same-name executable in an already-present writable directory without
//! mutating `PATH`, `GITHUB_PATH`, or the routed shell. The evidence shell must replace that inherited
//! search path with one rooted in the image-owned Rust proxy template and system directories, then
//! make PATH readonly before resolving any evidence command.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_PATH: &str =
    "readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn routed_path_is_readonly_system_path(source: &str) -> bool {
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
    let Some(run) = routed.get("run").and_then(Value::as_str) else {
        return false;
    };
    let lines = active_lines(run);
    let path_lines: Vec<_> = lines
        .iter()
        .copied()
        .filter(|line| line.contains("PATH"))
        .collect();

    lines.first().copied() == Some("set -euo pipefail")
        && lines.get(1).copied() == Some(CANONICAL_PATH)
        && path_lines == [CANONICAL_PATH]
}

#[test]
fn live_routed_evidence_replaces_inherited_user_writable_path() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_path_is_readonly_system_path(&source),
        "routed evidence must replace inherited user-home PATH entries with one readonly image/system-owned search path"
    );
}

#[test]
fn inherited_home_shim_without_path_mutation_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Plant an inherited-path shim
        run: |
          mkdir -p /home/runner/.local/bin
          printf '#!/bin/sh\nexit 0\n' > /home/runner/.local/bin/git
          chmod +x /home/runner/.local/bin/git
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_path_is_readonly_system_path(source),
        "a pre-existing writable PATH entry can shadow bare evidence commands even when PATH itself never changes"
    );
}

#[test]
fn writable_home_directory_must_not_be_retained_in_canonical_path() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly PATH=/home/runner/.local/bin:/etc/skel/.cargo/bin:/usr/bin:/bin
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_path_is_readonly_system_path(source));
}

#[test]
fn later_path_reassignment_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
          PATH=/tmp/evidence-shims:$PATH
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_path_is_readonly_system_path(source));
}