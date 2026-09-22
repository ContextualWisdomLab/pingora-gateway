//! RED contract for Bash command-hash poisoning on the routed provenance lane.
//!
//! Bash can bind a bare command name to an arbitrary executable with `hash -p`. The routed k6
//! provenance tail invokes `sha256sum`, `tar`, and `cmp` by bare name, so a prior hash-table
//! mutation can turn checksum/extraction/byte-identity evidence into a false GREEN without changing
//! the canonical tail. This test-first contract captures that counterexample before the causal
//! rejection is added.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
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

fn routed_provenance_hash_table_is_stable(source: &str) -> bool {
    let Ok(document) = serde_yaml::from_str::<Value>(source) else {
        return false;
    };
    let Some(job) = document.get("jobs").and_then(|jobs| jobs.get(LOAD_JOB)) else {
        return false;
    };
    let Some(steps) = job.get("steps").and_then(Value::as_sequence) else {
        return false;
    };
    let Some(routed) = unique_named_step(steps, ROUTED_STEP) else {
        return false;
    };

    routed
        .get("run")
        .and_then(Value::as_str)
        .is_some_and(|run| run.trim_end().ends_with(ROUTED_PROVENANCE_TAIL))
}

#[test]
fn live_routed_provenance_has_the_canonical_tail() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(routed_provenance_hash_table_is_stable(&source));
}

#[test]
fn hash_remapped_cmp_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          hash -p /bin/true cmp
          destination="/usr/local/bin/k""6"
          install -m 0755 /tmp/fake-k6 "$destination"
          {tail}
"#,
        tail = ROUTED_PROVENANCE_TAIL.replace('\n', "\n          ")
    );

    assert!(
        !routed_provenance_hash_table_is_stable(&source),
        "Bash `hash -p` can redirect cmp to /bin/true and falsely attest a replaced k6 binary"
    );
}
