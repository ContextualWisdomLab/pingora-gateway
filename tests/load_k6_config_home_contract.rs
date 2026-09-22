//! Fail-closed contract for k6's implicit user configuration search path.
//!
//! k6 reads a default user `config.json` when no explicit config file is supplied. That config can
//! alter options which are not fixed by the checked-in script, including global request-rate
//! limiting, so a pre-existing or earlier-step config can turn routed concurrency evidence into a
//! throttled run without changing the canonical k6 command or the 4-VU / 400-iteration script.
//! The routed evidence therefore resets a dedicated temporary config root immediately before the
//! measurement and binds `XDG_CONFIG_HOME` to that empty root for the k6 process only.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CONFIG_ROOT: &str = "/tmp/cwl-k6-routed-config-home";
const CONFIG_RESET: &str = "rm -rf /tmp/cwl-k6-routed-config-home";
const CONFIG_ASSIGNMENT: &str = "XDG_CONFIG_HOME=/tmp/cwl-k6-routed-config-home \\";
const GATEWAY_ASSIGNMENT: &str = "PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \\";
const ROUTED_K6: &str = "/usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js";

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

fn routed_k6_config_home_is_isolated(source: &str) -> bool {
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

    let active = active_lines(run);
    active.ends_with(&[
        CONFIG_RESET,
        CONFIG_ASSIGNMENT,
        GATEWAY_ASSIGNMENT,
        ROUTED_K6,
    ]) && active
        .iter()
        .filter(|line| line.contains("XDG_CONFIG_HOME"))
        .count()
        == 1
        && active
            .iter()
            .filter(|line| line.contains(CONFIG_ROOT))
            .count()
            == 2
}

#[test]
fn live_routed_load_uses_an_empty_dedicated_k6_config_root() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        routed_k6_config_home_is_isolated(&source),
        "routed release evidence must not inherit k6's implicit user config; reset and bind a dedicated XDG config root immediately before measurement"
    );
}

#[test]
fn inherited_default_k6_config_must_not_claim_routed_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Seed a user k6 config that throttles request rate
        run: |
          mkdir -p "$HOME/.config/k6"
          printf '{"rps":1}\n' > "$HOME/.config/k6/config.json"
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(
        !routed_k6_config_home_is_isolated(source),
        "k6's default user config can throttle a nominal 4-VU / 400-iteration run and invalidate concurrency evidence"
    );
}

#[test]
fn resetting_config_root_without_binding_it_is_insufficient() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          rm -rf /tmp/cwl-k6-routed-config-home
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_k6_config_home_is_isolated(source));
}

#[test]
fn binding_config_root_without_point_of_use_reset_is_insufficient() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          XDG_CONFIG_HOME=/tmp/cwl-k6-routed-config-home \
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(!routed_k6_config_home_is_isolated(source));
}

#[test]
fn canonical_point_of_use_config_isolation_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          set -euo pipefail
          rm -rf /tmp/cwl-k6-routed-config-home
          XDG_CONFIG_HOME=/tmp/cwl-k6-routed-config-home \
          PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
            /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;

    assert!(routed_k6_config_home_is_isolated(source));
}
