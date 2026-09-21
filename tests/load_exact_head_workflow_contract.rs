//! Exact-head source-binding contract for routed load evidence.
//!
//! A routed latency receipt is only attributable to the pull-request head when checkout and the
//! checkout-identity gate consume the workflow-owned `EXPECTED_SHA` without job-, step-, or
//! persisted runtime rebinding. Because shell code can reconstruct an `EXPECTED_SHA` assignment
//! without ever containing that literal token, the evidence-bearing path forbids `$GITHUB_ENV`
//! persistence before the routed summary gate instead of trying to parse arbitrary shell writes.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const EXPECTED_SHA_EXPR: &str = "${{ github.event.pull_request.head.sha || github.sha }}";
const CHECKOUT_STEP: &str = "Checkout exact revision";
const CHECKOUT_ACTION: &str = "actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8";
const CHECKOUT_REF: &str = "${{ env.EXPECTED_SHA }}";
const VERIFY_STEP: &str = "Verify checkout identity";
const VERIFY_RUN: &str = "test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SUMMARY_STEP: &str = "Require routed pg-erd latency summary";

fn failure_propagates(node: &Value) -> bool {
    matches!(
        node.get("continue-on-error"),
        None | Some(Value::Bool(false))
    )
}

fn expected_sha_overridden(node: &Value) -> bool {
    node.get("env")
        .and_then(|env| env.get("EXPECTED_SHA"))
        .is_some()
}

fn persists_runtime_environment(step: &Value) -> bool {
    step.get("run")
        .and_then(Value::as_str)
        .is_some_and(|run| run.contains("GITHUB_ENV"))
}

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<(usize, &'a Value)> {
    let mut matches = steps.iter().enumerate().filter(|(_, step)| {
        step.get("name").and_then(Value::as_str) == Some(name)
    });
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn checkout_contract(step: &Value) -> bool {
    if step.get("if").is_some()
        || !failure_propagates(step)
        || expected_sha_overridden(step)
    {
        return false;
    }
    if step.get("uses").and_then(Value::as_str) != Some(CHECKOUT_ACTION) {
        return false;
    }
    let Some(with) = step.get("with") else {
        return false;
    };
    with.get("ref").and_then(Value::as_str) == Some(CHECKOUT_REF)
        && with.get("persist-credentials") == Some(&Value::Bool(false))
}

fn verify_contract(step: &Value) -> bool {
    step.get("if").is_none()
        && failure_propagates(step)
        && !expected_sha_overridden(step)
        && step.get("run").and_then(Value::as_str) == Some(VERIFY_RUN)
}

fn load_evidence_claims_exact_head(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if document
        .get("env")
        .and_then(|env| env.get("EXPECTED_SHA"))
        .and_then(Value::as_str)
        != Some(EXPECTED_SHA_EXPR)
    {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if expected_sha_overridden(job) {
        return false;
    }
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };

    let Some((checkout_index, checkout)) = unique_named_step(steps, CHECKOUT_STEP) else {
        return false;
    };
    let Some((verify_index, verify)) = unique_named_step(steps, VERIFY_STEP) else {
        return false;
    };
    let Some((routed_index, routed)) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    let Some((summary_index, summary)) = unique_named_step(steps, SUMMARY_STEP) else {
        return false;
    };

    if steps
        .iter()
        .take(summary_index + 1)
        .any(persists_runtime_environment)
    {
        return false;
    }

    checkout_index < verify_index
        && verify_index < routed_index
        && routed_index < summary_index
        && checkout_contract(checkout)
        && verify_contract(verify)
        && !expected_sha_overridden(routed)
        && !expected_sha_overridden(summary)
}

#[test]
fn live_routed_load_evidence_is_bound_to_the_pull_request_head() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_evidence_claims_exact_head(&source),
        "routed load evidence must preserve workflow-owned source identity through measured traffic"
    );
}

#[test]
fn job_expected_sha_override_must_not_claim_exact_head_evidence() {
    let source = format!(
        r#"
env:
  EXPECTED_SHA: {EXPECTED_SHA_EXPR}
jobs:
  load-contract:
    env:
      EXPECTED_SHA: refs/heads/main
    steps:
      - name: Checkout exact revision
        uses: {CHECKOUT_ACTION}
        with:
          ref: ${{{{ env.EXPECTED_SHA }}}}
          persist-credentials: false
      - name: Verify checkout identity
        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"
      - name: Run routed pg-erd loopback traffic
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !load_evidence_claims_exact_head(&source),
        "a job-local EXPECTED_SHA override can make checkout and identity verification agree on the wrong revision"
    );
}

#[test]
fn paired_step_overrides_must_not_claim_exact_head_evidence() {
    let source = format!(
        r#"
env:
  EXPECTED_SHA: {EXPECTED_SHA_EXPR}
jobs:
  load-contract:
    steps:
      - name: Checkout exact revision
        env:
          EXPECTED_SHA: refs/heads/main
        uses: {CHECKOUT_ACTION}
        with:
          ref: ${{{{ env.EXPECTED_SHA }}}}
          persist-credentials: false
      - name: Verify checkout identity
        env:
          EXPECTED_SHA: refs/heads/main
        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"
      - name: Run routed pg-erd loopback traffic
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !load_evidence_claims_exact_head(&source),
        "step-local EXPECTED_SHA overrides must not make the wrong checkout self-consistent"
    );
}

#[test]
fn github_env_rebinding_must_not_claim_exact_head_evidence() {
    let source = format!(
        r#"
env:
  EXPECTED_SHA: {EXPECTED_SHA_EXPR}
jobs:
  load-contract:
    steps:
      - name: Rebind custom source identity
        run: echo 'EXPECTED_SHA=refs/heads/main' >> \"$GITHUB_ENV\"
      - name: Checkout exact revision
        uses: {CHECKOUT_ACTION}
        with:
          ref: ${{{{ env.EXPECTED_SHA }}}}
          persist-credentials: false
      - name: Verify checkout identity
        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"
      - name: Run routed pg-erd loopback traffic
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !load_evidence_claims_exact_head(&source),
        "persisted GITHUB_ENV rebinding must not retarget later checkout and verification"
    );
}

#[test]
fn indirect_github_env_rebinding_must_not_claim_exact_head_evidence() {
    let source = format!(
        r#"
env:
  EXPECTED_SHA: {EXPECTED_SHA_EXPR}
jobs:
  load-contract:
    steps:
      - name: Rebind custom source identity indirectly
        run: |
          suffix=SHA
          printf 'EXPECTED_%s=refs/heads/main\n' "$suffix" >> "$GITHUB_ENV"
      - name: Checkout exact revision
        uses: {CHECKOUT_ACTION}
        with:
          ref: ${{{{ env.EXPECTED_SHA }}}}
          persist-credentials: false
      - name: Verify checkout identity
        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"
      - name: Run routed pg-erd loopback traffic
        run: echo measured
      - name: Require routed pg-erd latency summary
        run: test -s k6-pg-erd-summary.json
"#
    );

    assert!(
        !load_evidence_claims_exact_head(&source),
        "indirect writes through GITHUB_ENV can reconstruct EXPECTED_SHA without the literal token"
    );
}
