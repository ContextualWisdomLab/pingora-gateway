//! Guard routed k6 provenance against privileged mutation of its trust boundary.
//!
//! The point-of-use archive checksum and byte-identity comparison are only meaningful when earlier
//! workflow steps cannot replace `/usr/bin/sha256sum`, `/usr/bin/tar`, `/usr/bin/cmp`, or the
//! installed `/usr/local/bin/k6`. On the canonical GitHub-hosted runner those paths require root to
//! mutate. The load job therefore admits only the two reviewed `sudo` lines needed for native
//! dependencies and the pinned k6 install; any additional privileged command is evidence drift.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const NATIVE_DEPENDENCIES_STEP: &str = "Install native dependencies";
const K6_INSTALL_STEP: &str = "Install checksum-pinned k6 2.2.0";
const NATIVE_DEPENDENCIES_SUDO: &str = "sudo apt-get update && sudo apt-get install -y --no-install-recommends ca-certificates cmake curl libssl-dev pkg-config";
const K6_INSTALL_SUDO: &str =
    "sudo install -m 0755 /tmp/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6";

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

fn privileged_lines(step: &Value) -> impl Iterator<Item = &str> {
    step.get("run")
        .and_then(Value::as_str)
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter(|line| contains_shell_identifier(line, "sudo"))
}

fn load_privilege_surface_is_canonical(source: &str) -> bool {
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

    let observed = steps
        .iter()
        .flat_map(|step| {
            let name = step.get("name").and_then(Value::as_str).unwrap_or("");
            privileged_lines(step).map(move |line| (name, line))
        })
        .collect::<Vec<_>>();

    observed
        == [
            (NATIVE_DEPENDENCIES_STEP, NATIVE_DEPENDENCIES_SUDO),
            (K6_INSTALL_STEP, K6_INSTALL_SUDO),
        ]
}

#[test]
fn live_routed_load_privilege_surface_is_canonical() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_privilege_surface_is_canonical(&source),
        "load-contract must expose only the reviewed native-dependency and pinned-k6 privileged mutations"
    );
}

#[test]
fn intervening_privileged_cmp_replacement_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    runs-on: ubuntu-24.04
    steps:
      - name: Install native dependencies
        run: sudo apt-get update && sudo apt-get install -y --no-install-recommends ca-certificates cmake curl libssl-dev pkg-config
      - name: Install checksum-pinned k6 2.2.0
        run: |
          sudo install -m 0755 /tmp/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
      - name: Replace provenance comparator
        run: sudo install -m 0755 /bin/true /usr/bin/cmp
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          rm -rf /tmp/cwl-k6-routed
          mkdir -p /tmp/cwl-k6-routed
          echo "b5a8003c86f35f5cd5ceef1490312c48e587696c94d998cefc6d7b3b4cb1597d  /tmp/k6-v2.2.0-linux-amd64.tar.gz" | sha256sum --check --strict
          tar -xzf /tmp/k6-v2.2.0-linux-amd64.tar.gz -C /tmp/cwl-k6-routed
          cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !load_privilege_surface_is_canonical(source),
        "an extra sudo mutation can replace provenance utilities while leaving the routed evidence text unchanged"
    );
}

#[test]
fn privileged_mutation_hidden_inside_k6_install_step_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install native dependencies
        run: sudo apt-get update && sudo apt-get install -y --no-install-recommends ca-certificates cmake curl libssl-dev pkg-config
      - name: Install checksum-pinned k6 2.2.0
        run: |
          sudo install -m 0755 /tmp/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
          sudo install -m 0755 /bin/true /usr/bin/sha256sum
"#;

    assert!(!load_privilege_surface_is_canonical(source));
}

#[test]
fn canonical_privilege_surface_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install native dependencies
        run: sudo apt-get update && sudo apt-get install -y --no-install-recommends ca-certificates cmake curl libssl-dev pkg-config
      - name: Install checksum-pinned k6 2.2.0
        run: |
          set -euo pipefail
          sudo install -m 0755 /tmp/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
      - name: Run routed pg-erd loopback traffic
        run: echo measured
"#;

    assert!(load_privilege_surface_is_canonical(source));
}
