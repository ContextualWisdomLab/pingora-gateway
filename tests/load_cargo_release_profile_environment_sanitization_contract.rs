//! Fail-closed contract for inherited Cargo release-profile authority in routed-load evidence.
//!
//! Cargo profile settings can be overridden by `CARGO_PROFILE_RELEASE_*` environment variables,
//! including build-script/proc-macro settings under `CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_*`.
//! Workflow/job/step YAML checks do not cover variables inherited from the runner process, so the
//! routed benchmark must remove every inherited release-profile override before its release rebuild.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const SANITIZER_START: &str = "while IFS='=' read -r profile_env _; do";
const SANITIZER_MATCH: &str = "CARGO_PROFILE_RELEASE_*) unset \"$profile_env\" ;;";
const SANITIZER_SOURCE: &str = "done < <(/usr/bin/env)";
const RELEASE_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";

fn routed_run(source: &str) -> Option<String> {
    let document = serde_yaml::from_str::<Value>(source).ok()?;
    let steps = document
        .get("jobs")?
        .get(LOAD_JOB)?
        .get("steps")?
        .as_sequence()?;
    let mut routed = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP));
    let run = routed.next()?.get("run")?.as_str()?.to_owned();
    (routed.next().is_none()).then_some(run)
}

fn inherited_release_profile_authority_is_sanitized(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(start) = run.find(SANITIZER_START) else {
        return false;
    };
    let Some(matcher) = run.find(SANITIZER_MATCH) else {
        return false;
    };
    let Some(end) = run.find(SANITIZER_SOURCE) else {
        return false;
    };
    let Some(build) = run.find(RELEASE_BUILD) else {
        return false;
    };

    start < matcher && matcher < end && end < build
}

#[test]
fn live_routed_rebuild_sanitizes_inherited_release_profile_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        inherited_release_profile_authority_is_sanitized(&source),
        "routed release evidence must remove inherited CARGO_PROFILE_RELEASE_* authority before rebuilding"
    );
}

#[test]
fn missing_inherited_profile_sanitizer_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!inherited_release_profile_authority_is_sanitized(source));
}

#[test]
fn sanitizer_after_release_build_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
          while IFS='=' read -r profile_env _; do
            case "$profile_env" in
              CARGO_PROFILE_RELEASE_*) unset "$profile_env" ;;
            esac
          done < <(/usr/bin/env)
"#;
    assert!(!inherited_release_profile_authority_is_sanitized(source));
}

#[test]
fn prefix_wide_sanitizer_before_release_build_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          while IFS='=' read -r profile_env _; do
            case "$profile_env" in
              CARGO_PROFILE_RELEASE_*) unset "$profile_env" ;;
            esac
          done < <(/usr/bin/env)
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(inherited_release_profile_authority_is_sanitized(source));
}
