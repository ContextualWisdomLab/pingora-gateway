//! Guard the routed k6 provenance lane against privileged post-install mutation.
//!
//! Point-of-use checksum and byte-identity checks only mean what they claim when the commands
//! performing those checks have not been replaced after the pinned k6 install. This contract is
//! test-first: the initial predicate only inspects the routed step, so an intervening privileged
//! step that replaces `/usr/bin/cmp` remains an intentional counterexample until the causal repair.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

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

fn active_run_uses_sudo(step: &Value) -> bool {
    step.get("run")
        .and_then(Value::as_str)
        .is_some_and(|run| {
            run.lines()
                .map(str::trim_start)
                .filter(|line| !line.starts_with('#'))
                .any(|line| contains_shell_identifier(line, "sudo"))
        })
}

fn post_install_privilege_boundary_is_stable(source: &str) -> bool {
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

    let mut routed = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let Some(routed) = routed.next() else {
        return false;
    };
    if routed.next().is_some() {
        return false;
    }

    !active_run_uses_sudo(routed)
}

#[test]
fn live_routed_lane_does_not_use_privileged_mutation_in_measurement_step() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(post_install_privilege_boundary_is_stable(&source));
}

#[test]
fn intervening_privileged_cmp_replacement_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    runs-on: ubuntu-24.04
    steps:
      - name: Install checksum-pinned k6 2.2.0
        run: sudo install -m 0755 /tmp/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
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
        !post_install_privilege_boundary_is_stable(source),
        "an earlier sudo mutation can replace the provenance utilities while leaving the routed evidence text unchanged"
    );
}
