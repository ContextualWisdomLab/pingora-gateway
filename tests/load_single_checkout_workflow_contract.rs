//! Exact-head checkout uniqueness contract for routed load evidence.
//!
//! The routed receipt is attributable to one checked-out source tree only when the evidence path
//! has exactly one checkout before the summary gate. The canonical checkout/identity contract
//! already pins the expected revision; this companion contract prevents a later checkout action
//! from replacing that verified worktree before build or measurement.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const CHECKOUT_STEP: &str = "Checkout exact revision";
const SUMMARY_STEP: &str = "Require routed pg-erd latency summary";
const CHECKOUT_ACTION: &str = "actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8";
const CHECKOUT_ACTION_PREFIX: &str = "actions/checkout@";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<(usize, &'a Value)> {
    let mut matches = steps.iter().enumerate().filter(|(_, step)| {
        step.get("name").and_then(Value::as_str) == Some(name)
    });
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn routed_evidence_uses_one_checkout(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some((checkout_index, checkout)) = unique_named_step(steps, CHECKOUT_STEP) else {
        return false;
    };
    let Some((summary_index, _)) = unique_named_step(steps, SUMMARY_STEP) else {
        return false;
    };
    if checkout_index >= summary_index
        || checkout.get("uses").and_then(Value::as_str) != Some(CHECKOUT_ACTION)
    {
        return false;
    }

    steps
        .iter()
        .take(summary_index + 1)
        .filter(|step| {
            step.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|uses| uses.starts_with(CHECKOUT_ACTION_PREFIX))
        })
        .count()
        == 1
}

#[test]
fn live_routed_load_uses_one_checkout_before_evidence() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_evidence_uses_one_checkout(&source),
        "routed evidence must remain on the one canonical checked-out revision"
    );
}

#[test]
fn checkout_after_identity_verification_must_not_claim_exact_head_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        uses: {CHECKOUT_ACTION}
      - name: Verify checkout identity
        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"
      - name: Checkout decoy revision
        uses: {CHECKOUT_ACTION}
        with:
          ref: refs/heads/main
          persist-credentials: false
      - name: Run routed pg-erd loopback traffic
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !routed_evidence_uses_one_checkout(&source),
        "a second checkout can replace the verified worktree before routed measurement"
    );
}

#[test]
fn checkout_at_another_action_version_must_not_claim_exact_head_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        uses: {CHECKOUT_ACTION}
      - name: Verify checkout identity
        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"
      - name: Checkout another tree
        uses: actions/checkout@v4
        with:
          ref: refs/heads/main
      - name: Run routed pg-erd loopback traffic
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(!routed_evidence_uses_one_checkout(&source));
}
