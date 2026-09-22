//! Fail-closed contract for `load-contract` shell selection.
//!
//! Run-line privilege review is insufficient when a step can replace GitHub's normal shell with an
//! arbitrary command template such as `sudo bash {0}`. The load job therefore permits explicit
//! `shell:` only on the three reviewed traffic/tool steps, and each must be exactly `bash`; all
//! other steps stay on the GitHub-hosted runner's default shell contract.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const K6_INSTALL_STEP: &str = "Install checksum-pinned k6 2.2.0";
const LOOPBACK_STEP: &str = "Exercise concurrent loopback traffic contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

fn load_shell_surface_is_canonical(source: &str) -> bool {
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
        .filter_map(|step| {
            let shell = step.get("shell").and_then(Value::as_str)?;
            let name = step.get("name").and_then(Value::as_str).unwrap_or("");
            Some((name, shell))
        })
        .collect::<Vec<_>>();

    observed
        == [
            (K6_INSTALL_STEP, "bash"),
            (LOOPBACK_STEP, "bash"),
            (ROUTED_STEP, "bash"),
        ]
}

#[test]
fn live_load_job_shell_surface_is_canonical() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_shell_surface_is_canonical(&source),
        "load-contract must use explicit shell selection only for the three reviewed Bash steps"
    );
}

#[test]
fn additional_privileged_shell_template_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
        run: echo install
      - name: Exercise concurrent loopback traffic contract
        shell: bash
        run: echo loopback
      - name: Replace provenance comparator without sudo in run text
        shell: sudo bash {0}
        run: install -m 0755 /bin/true /usr/bin/cmp
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: echo routed
"#;

    assert!(
        !load_shell_surface_is_canonical(source),
        "a custom shell template can acquire root before run-line privilege checks see any sudo token"
    );
}

#[test]
fn changed_routed_shell_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
      - name: Exercise concurrent loopback traffic contract
        shell: bash
      - name: Run routed pg-erd loopback traffic
        shell: bash --noprofile --norc {0}
"#;

    assert!(!load_shell_surface_is_canonical(source));
}

#[test]
fn canonical_shell_surface_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
      - name: Exercise concurrent loopback traffic contract
        shell: bash
      - name: Run routed pg-erd loopback traffic
        shell: bash
"#;

    assert!(load_shell_surface_is_canonical(source));
}
