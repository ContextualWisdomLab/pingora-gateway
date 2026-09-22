//! Test-first contract for `load-contract` shell selection.
//!
//! Run-line review is insufficient when a step can replace GitHub's normal shell with an arbitrary
//! command template such as `sudo bash {0}`. The initial predicate checks only the known explicit
//! Bash steps, intentionally exposing an additional custom-shell step as the counterexample.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const K6_INSTALL_STEP: &str = "Install checksum-pinned k6 2.2.0";
const LOOPBACK_STEP: &str = "Exercise concurrent loopback traffic contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

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

    [K6_INSTALL_STEP, LOOPBACK_STEP, ROUTED_STEP]
        .iter()
        .all(|name| {
            unique_named_step(steps, name)
                .and_then(|step| step.get("shell"))
                .and_then(Value::as_str)
                == Some("bash")
        })
}

#[test]
fn live_known_explicit_shells_are_bash() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(load_shell_surface_is_canonical(&source));
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
