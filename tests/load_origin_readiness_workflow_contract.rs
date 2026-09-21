//! Regression contract for measured-origin readiness in the load workflow.
//!
//! The gateway health endpoint proves only the proxy process is accepting traffic. It does not
//! prove the measured upstream fixture has bound its socket. The load harness must therefore prove
//! origin readiness directly before starting the gateway candidate or k6 measurement.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";

fn strip_shell_comments(script: &str) -> String {
    script
        .lines()
        .map(|line| {
            let mut single_quoted = false;
            let mut double_quoted = false;
            let mut escaped = false;
            let mut visible = String::new();

            for character in line.chars() {
                if escaped {
                    visible.push(character);
                    escaped = false;
                    continue;
                }
                if character == '\\' && !single_quoted {
                    visible.push(character);
                    escaped = true;
                    continue;
                }
                if character == '\'' && !double_quoted {
                    single_quoted = !single_quoted;
                    visible.push(character);
                    continue;
                }
                if character == '"' && !single_quoted {
                    double_quoted = !double_quoted;
                    visible.push(character);
                    continue;
                }
                if character == '#' && !single_quoted && !double_quoted {
                    break;
                }
                visible.push(character);
            }

            visible
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn load_job_scripts(source: &str) -> Option<Vec<String>> {
    let document: Value = serde_yaml::from_str(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;

    Some(
        steps
            .iter()
            .filter(|step| step.get("if").is_none())
            .filter_map(|step| step.get("run").and_then(Value::as_str))
            .map(strip_shell_comments)
            .collect(),
    )
}

fn script_proves_readiness(source: &str) -> bool {
    let Some(origin_start) = source.find("/tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &")
    else {
        return false;
    };
    let Some(origin_ready) = source.find("http://127.0.0.1:18081/fixture-ready") else {
        return false;
    };
    let Some(origin_liveness) = source.find("kill -0 \"$upstream_pid\"") else {
        return false;
    };
    let Some(gateway_start) =
        source.find("target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml")
    else {
        return false;
    };
    let Some(measured_traffic) = source.find("GATEWAY_URL=http://127.0.0.1:18080 k6 run") else {
        return false;
    };

    origin_start < origin_ready
        && origin_ready < origin_liveness
        && origin_liveness < gateway_start
        && gateway_start < measured_traffic
}

fn readiness_contract_accepts(source: &str) -> bool {
    load_job_scripts(source)
        .is_some_and(|scripts| scripts.iter().any(|script| script_proves_readiness(script)))
}

#[test]
fn load_contract_proves_origin_readiness_before_gateway_measurement() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");

    assert!(
        readiness_contract_accepts(&source),
        "origin readiness and liveness must be established inside one unconditional measured load step before gateway startup and measured traffic"
    );
}

#[test]
fn unrelated_job_decoy_must_not_manufacture_readiness_order_evidence() {
    let source = r#"
jobs:
  unrelated:
    steps:
      - run: |
          /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
          curl http://127.0.0.1:18081/fixture-ready
          kill -0 "$upstream_pid"
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
  load-contract:
    steps:
      - run: |
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
"#;

    assert!(
        !readiness_contract_accepts(source),
        "readiness strings in another job must not let the measured load job omit its own origin-readiness proof"
    );
}

#[test]
fn skipped_step_decoy_must_not_manufacture_readiness_order_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - if: ${{ false }}
        run: |
          /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
          curl http://127.0.0.1:18081/fixture-ready
          kill -0 "$upstream_pid"
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
      - run: |
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
"#;

    assert!(
        !readiness_contract_accepts(source),
        "a skipped step must not manufacture origin-readiness ordering for the measured load lane"
    );
}

#[test]
fn commented_liveness_decoy_must_not_manufacture_readiness_order_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
          curl http://127.0.0.1:18081/fixture-ready
          # kill -0 "$upstream_pid"
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
"#;

    assert!(
        !readiness_contract_accepts(source),
        "commented shell text must not manufacture origin-liveness evidence for the measured load lane"
    );
}

#[test]
fn split_step_decoy_must_not_manufacture_one_process_readiness_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - run: |
          /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
          curl http://127.0.0.1:18081/fixture-ready
      - run: |
          kill -0 "$upstream_pid"
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
"#;

    assert!(
        !readiness_contract_accepts(source),
        "markers spread across separate shell processes must not manufacture one-step readiness/liveness evidence"
    );
}

#[test]
fn shell_data_must_not_manufacture_liveness_evidence() {
    for archived_liveness in [
        "archive='kill -0 \"$upstream_pid\"'",
        "cat <<'ARCHIVE'\nkill -0 \"$upstream_pid\"\nARCHIVE",
    ] {
        let source = format!(
            r#"
jobs:
  load-contract:
    steps:
      - run: |
          /tmp/load_origin >/tmp/upstream-fixture.log 2>&1 &
          if curl http://127.0.0.1:18081/fixture-ready; then
            break
          fi
          {archived_liveness}
          target/release/cwl-pingora-gateway --config /tmp/gateway-load.yaml
          GATEWAY_URL=http://127.0.0.1:18080 k6 run
"#
        );

        assert!(
            !readiness_contract_accepts(&source),
            "quoted assignments and heredoc payloads are shell data, not executable liveness proof"
        );
    }
}

#[test]
fn quoted_hashes_are_not_treated_as_shell_comments() {
    let source = "printf '%s\\n' '# fixture marker' \"# second marker\" # actual comment";
    assert_eq!(
        strip_shell_comments(source),
        "printf '%s\\n' '# fixture marker' \"# second marker\" "
    );
}
