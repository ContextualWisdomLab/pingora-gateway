//! Fail-closed contract for inherited k6 option authority in routed-load evidence.
//!
//! Grafana k6 gives `K6_*` environment options higher precedence than the checked-in script options.
//! In particular, inherited workload controls can replace the reviewed 4-VU / 400-iteration shape,
//! and `K6_NO_THRESHOLDS` can disable the latency/failure gates entirely. Workflow YAML checks only
//! reject options explicitly declared in the repository; they cannot remove runner-process state.
//! The routed evidence step therefore clears every inherited `K6_*` name immediately before the
//! pinned k6 binary is executed and fails closed if a matching environment name is not a Bash-safe
//! identifier.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_K6: &str = "/usr/local/bin/k6 run --quiet tests/load/pg_erd_gateway_smoke.js";
const CANONICAL_SANITIZE: &str = r#"while IFS='=' read -r k6_env _; do
  case "$k6_env" in
    K6_*)
      if [[ "$k6_env" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
        unset "$k6_env"
      else
        echo "unsupported inherited k6 environment name: $k6_env" >&2
        exit 1
      fi
      ;;
  esac
done < <(/usr/bin/env)"#;

fn routed_run(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let run = matches.next()?.get("run")?.as_str()?.to_owned();
    matches.next().is_none().then_some(run)
}

fn inherited_k6_environment_is_sanitized_before_run(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(sanitize_index) = run.find(CANONICAL_SANITIZE) else {
        return false;
    };
    if run[sanitize_index + CANONICAL_SANITIZE.len()..].contains(CANONICAL_SANITIZE) {
        return false;
    }
    let Some(k6_index) = run.find(ROUTED_K6) else {
        return false;
    };
    sanitize_index < k6_index
}

#[test]
fn live_routed_load_neutralizes_inherited_k6_option_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        inherited_k6_environment_is_sanitized_before_run(&source),
        "routed evidence must clear inherited K6_* option authority before executing the pinned k6 binary"
    );
}

#[test]
fn inherited_k6_options_without_sanitization_are_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {ROUTED_K6}\n"
    );
    assert!(!inherited_k6_environment_is_sanitized_before_run(&source));
}

#[test]
fn k6_sanitization_after_measurement_is_rejected() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {ROUTED_K6}\n          {}\n",
        CANONICAL_SANITIZE.replace('\n', "\n          ")
    );
    assert!(!inherited_k6_environment_is_sanitized_before_run(&source));
}

#[test]
fn partial_named_option_cleanup_is_rejected() {
    let partial = "unset K6_NO_THRESHOLDS K6_VUS K6_ITERATIONS";
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {partial}\n          {ROUTED_K6}\n"
    );
    assert!(
        !inherited_k6_environment_is_sanitized_before_run(&source),
        "enumerating only known k6 option names leaves future or less obvious K6_* overrides authoritative"
    );
}

#[test]
fn canonical_dynamic_sanitization_before_measurement_is_admitted() {
    let source = format!(
        "jobs:\n  {LOAD_JOB}:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {}\n          {ROUTED_K6}\n",
        CANONICAL_SANITIZE.replace('\n', "\n          ")
    );
    assert!(inherited_k6_environment_is_sanitized_before_run(&source));
}
