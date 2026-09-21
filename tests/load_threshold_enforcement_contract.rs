//! Fail-closed contract that keeps routed k6 thresholds executable.
//!
//! The routed load receipt is only meaningful when k6 evaluates the thresholds declared by the
//! checked-in script. Grafana k6 exposes `K6_NO_THRESHOLDS` / `--no-thresholds` specifically to
//! disable threshold execution. GNU Bash also sources `BASH_ENV` for non-interactive shells, so a
//! startup hook could silently export the k6 disable switch before the routed command. The evidence
//! lane therefore rejects both controls at workflow, job, step, and routed shell-command scope.
//! Earlier persisted-environment mutation is separately forbidden by the exact-head contract.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_K6: &str = "k6 run --quiet tests/load/pg_erd_gateway_smoke.js";
const NO_THRESHOLDS_ENV: &str = "K6_NO_THRESHOLDS";
const NO_THRESHOLDS_FLAG: &str = "--no-thresholds";
const BASH_ENV: &str = "BASH_ENV";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn env_key_present(node: &Value, key: &str) -> bool {
    node.get("env").and_then(|env| env.get(key)).is_some()
}

fn evidence_environment_is_clean(node: &Value) -> bool {
    !env_key_present(node, NO_THRESHOLDS_ENV) && !env_key_present(node, BASH_ENV)
}

fn routed_thresholds_are_enforced(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    if !evidence_environment_is_clean(&document) {
        return false;
    }

    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    if !evidence_environment_is_clean(job) {
        return false;
    }

    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };
    if !evidence_environment_is_clean(routed) {
        return false;
    }

    routed.get("run").and_then(Value::as_str).is_some_and(|run| {
        run.contains(ROUTED_K6)
            && !run.contains(NO_THRESHOLDS_ENV)
            && !run.contains(NO_THRESHOLDS_FLAG)
            && !run.contains(BASH_ENV)
    })
}

#[test]
fn live_routed_load_keeps_threshold_execution_enabled() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_thresholds_are_enforced(&source),
        "routed release evidence must execute the checked-in k6 thresholds without shell startup injection"
    );
}

#[test]
fn workflow_level_no_thresholds_must_not_claim_routed_release_evidence() {
    let source = r#"
env:
  K6_NO_THRESHOLDS: true
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_thresholds_are_enforced(source));
}

#[test]
fn job_level_no_thresholds_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    env:
      K6_NO_THRESHOLDS: true
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_thresholds_are_enforced(source),
        "K6_NO_THRESHOLDS disables threshold evaluation even when the canonical k6 command text is unchanged"
    );
}

#[test]
fn step_level_no_thresholds_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        env:
          K6_NO_THRESHOLDS: true
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_thresholds_are_enforced(source));
}

#[test]
fn shell_exported_no_thresholds_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          export K6_NO_THRESHOLDS=true
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_thresholds_are_enforced(source));
}

#[test]
fn cli_no_thresholds_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --no-thresholds --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_thresholds_are_enforced(source));
}

#[test]
fn bash_env_startup_hook_must_not_claim_routed_release_evidence() {
    let source = r#"
env:
  BASH_ENV: /tmp/evidence-startup.sh
jobs:
  load-contract:
    steps:
      - name: Prepare startup hook
        run: printf 'export K6_NO_THRESHOLDS=true\n' > /tmp/evidence-startup.sh
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_thresholds_are_enforced(source),
        "non-interactive bash sources BASH_ENV before the routed command, so startup hooks can disable thresholds without changing routed command text"
    );
}

#[test]
fn job_level_bash_env_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    env:
      BASH_ENV: /tmp/evidence-startup.sh
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_thresholds_are_enforced(source));
}

#[test]
fn routed_step_bash_env_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        env:
          BASH_ENV: /tmp/evidence-startup.sh
        run: |
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_thresholds_are_enforced(source));
}
