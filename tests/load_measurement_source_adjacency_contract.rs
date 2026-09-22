//! Measurement-time source binding for routed load evidence.
//!
//! The routed k6 script is interpreted only when k6 starts, so an earlier exact-HEAD check does
//! not bind the actual workload if the script can be rewritten later in the same shell. The final
//! source check therefore forms one exact suffix with supplier provenance and the k6 invocation:
//! no executable line may intervene between script verification and measurement.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const MEASUREMENT_TAIL: &str = r#"git diff --exit-code HEAD -- tests/load/pg_erd_gateway_smoke.js
rm -rf /tmp/cwl-k6-routed
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

fn routed_measurement_source_is_adjacent(source: &str) -> bool {
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

    routed
        .get("run")
        .and_then(Value::as_str)
        .is_some_and(|run| run.trim_end().ends_with(MEASUREMENT_TAIL))
}

#[test]
fn live_routed_workload_is_bound_immediately_before_measurement() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_measurement_source_is_adjacent(&source),
        "the interpreted routed k6 script must be revalidated against HEAD immediately before the provenance tail and k6 invocation"
    );
}

#[test]
fn mutation_after_an_earlier_source_check_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          git diff --exit-code HEAD -- tests/load/pg_erd_gateway_smoke.js
          printf 'export default function () {}' > tests/load/pg_erd_gateway_smoke.js
          rm -rf /tmp/cwl-k6-routed
          mkdir -p /tmp/cwl-k6-routed
          echo "b5a8003c86f35f5cd5ceef1490312c48e587696c94d998cefc6d7b3b4cb1597d  /tmp/k6-v2.2.0-linux-amd64.tar.gz" | sha256sum --check --strict
          tar -xzf /tmp/k6-v2.2.0-linux-amd64.tar.gz -C /tmp/cwl-k6-routed
          cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_measurement_source_is_adjacent(source),
        "a source check that precedes a later workload rewrite is stale measurement evidence"
    );
}

#[test]
fn canonical_measurement_tail_is_admitted() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          {tail}
"#,
        tail = MEASUREMENT_TAIL.replace('\n', "\n          ")
    );

    assert!(routed_measurement_source_is_adjacent(&source));
}
