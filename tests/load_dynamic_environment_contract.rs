//! Fail-closed routed-load contract for indirect environment mutation.
//!
//! Literal `K6_*` scans are insufficient when Bash reconstructs an option name at runtime and
//! exports it before invoking k6. A process-local wrapper can do the same without mutating the
//! shell environment. This contract therefore admits only the exact routed k6 launch pair, the
//! canonical readonly command/toolchain/build trust-root bindings, and keeps the evidence shell
//! free of other generic export/declaration, allexport, and environment-wrapper primitives.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SET: &str = "set -euo pipefail";
const CANONICAL_EXPECTED_SHA: &str = "readonly EXPECTED_SHA";
const CANONICAL_BASH_CMDS: &str = "readonly BASH_CMDS";
const CANONICAL_PATH: &str =
    "readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
const CANONICAL_CARGO_HOME: &str = "declare -rx CARGO_HOME=/tmp/cwl-routed-cargo-home";
const CANONICAL_RUSTUP_HOME: &str = "declare -rx RUSTUP_HOME=/etc/skel/.rustup";
const CANONICAL_RUSTUP_TOOLCHAIN: &str =
    "declare -rx RUSTUP_TOOLCHAIN=1.98.1-x86_64-unknown-linux-gnu";
const ROUTED_URL_ASSIGNMENT: &str = "PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \\";
const ROUTED_K6: &str = "/usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js";
const K6_PATH: &str = "/usr/local/bin/k6";
const K6_PROVENANCE_CMP: &str =
    "cmp --silent /tmp/cwl-k6-routed/k6-v2.2.0-linux-amd64/k6 /usr/local/bin/k6";

fn unique_named_step<'a>(steps: &'a [Value], name: &str) -> Option<&'a Value> {
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name));
    let step = matches.next()?;
    matches.next().is_none().then_some(step)
}

fn active_lines(run: &str) -> impl Iterator<Item = &str> {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

fn mutates_exported_environment(line: &str) -> bool {
    line.starts_with("export ")
        || (line.starts_with("declare ")
            && line != CANONICAL_CARGO_HOME
            && line != CANONICAL_RUSTUP_HOME
            && line != CANONICAL_RUSTUP_TOOLCHAIN)
        || line.starts_with("typeset ")
        || (line.starts_with("readonly ")
            && line != CANONICAL_EXPECTED_SHA
            && line != CANONICAL_BASH_CMDS
            && line != CANONICAL_PATH)
        || (line.starts_with("set ") && line != CANONICAL_SET)
}

fn invokes_environment_wrapper(line: &str) -> bool {
    line.starts_with("env ")
        || line.starts_with("/usr/bin/env ")
        || line.starts_with("command env ")
        || line.starts_with("command /usr/bin/env ")
        || line.starts_with("\\env ")
}

fn has_exact_measurement_launch(run: &str) -> bool {
    let lines: Vec<_> = active_lines(run).collect();
    let exact_pairs = lines
        .windows(2)
        .filter(|pair| pair[0] == ROUTED_URL_ASSIGNMENT && pair[1] == ROUTED_K6)
        .count();
    let k6_lines_are_canonical = lines
        .iter()
        .all(|line| !line.contains(K6_PATH) || *line == ROUTED_K6 || *line == K6_PROVENANCE_CMP);
    exact_pairs == 1 && k6_lines_are_canonical
}

fn routed_shell_environment_is_stable(run: &str) -> bool {
    has_exact_measurement_launch(run)
        && active_lines(run).all(|line| {
            !line.contains("K6_")
                && !mutates_exported_environment(line)
                && !invokes_environment_wrapper(line)
        })
}

fn live_routed_shell_environment_is_stable(source: &str) -> bool {
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
        .is_some_and(routed_shell_environment_is_stable)
}

#[test]
fn live_routed_shell_does_not_mutate_k6_environment_indirectly() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        live_routed_shell_environment_is_stable(&source),
        "routed evidence must use the exact unwrapped k6 launch and reject indirect option injection"
    );
}

#[test]
fn reconstructed_no_thresholds_export_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
option_prefix=K6
option_name="${option_prefix}_NO_THRESHOLDS"
declare -x "${option_name}=true"
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn reconstructed_workload_export_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
prefix=K6
name="${prefix}_ITERATIONS"
export "${name}=1"
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn allexport_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
set -a
prefix=K6
${prefix}_VUS=1
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn process_local_env_wrapper_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
prefix=K6
name="${prefix}_NO_THRESHOLDS"
env "${name}=true" PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn absolute_env_wrapper_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
prefix=K6
name="${prefix}_ITERATIONS"
/usr/bin/env "${name}=1" PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn command_env_wrapper_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
prefix=K6
name="${prefix}_VUS"
command env "${name}=1" PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn reconstructed_environment_command_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
prefix=K6
name="${prefix}_NO_THRESHOLDS"
launcher=en
launcher="${launcher}v"
"${launcher}" "${name}=true" PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn generic_process_wrapper_must_not_claim_routed_release_evidence() {
    let run = r#"
set -euo pipefail
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
time /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn canonical_readonly_trust_root_remains_admitted() {
    let run = r#"
set -euo pipefail
readonly EXPECTED_SHA
readonly BASH_CMDS
readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
declare -rx CARGO_HOME=/tmp/cwl-routed-cargo-home
declare -rx RUSTUP_HOME=/etc/skel/.rustup
declare -rx RUSTUP_TOOLCHAIN=1.98.1-x86_64-unknown-linux-gnu
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(routed_shell_environment_is_stable(run));
}

#[test]
fn floating_rustup_toolchain_is_rejected() {
    let run = r#"
set -euo pipefail
readonly EXPECTED_SHA
readonly BASH_CMDS
readonly PATH=/etc/skel/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
declare -rx CARGO_HOME=/tmp/cwl-routed-cargo-home
declare -rx RUSTUP_HOME=/etc/skel/.rustup
declare -rx RUSTUP_TOOLCHAIN=stable-x86_64-unknown-linux-gnu
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn noncanonical_readonly_environment_mutation_is_rejected() {
    let run = r#"
set -euo pipefail
readonly OTHER=value
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(!routed_shell_environment_is_stable(run));
}

#[test]
fn canonical_shell_options_remain_admitted() {
    let run = r#"
set -euo pipefail
PG_ERD_GATEWAY_URL=http://127.0.0.1:18180 \
  /usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js
"#;
    assert!(routed_shell_environment_is_stable(run));
}
