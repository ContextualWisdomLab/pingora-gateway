//! Fail-closed contract for inherited `cc` build-tool authority in routed-load evidence.
//!
//! `libz-ng-sys` is built through `cmake` 0.1.58, which uses `cc` to resolve native C/C++
//! compiler and flag authority. `openssl-src` also resolves its compiler and archiver through `cc`;
//! that exact build path documents `CROSS_COMPILE` as an input to `cc::Build` before OpenSSL later
//! removes it from the `./Configure` child environment. The routed rebuild therefore clears compiler,
//! archiver, flags, wrapper/prefix policy, and target-/host-prefixed authority, and fails closed when
//! a matching inherited name is not a valid Bash identifier.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CARGO_CLEAN: &str = "cargo clean --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";
const CANONICAL_SANITIZE: &str = r#"while IFS='=' read -r cc_env _; do
  case "$cc_env" in
    CC|CC_*|HOST_CC|HOST_CC_*|TARGET_CC|TARGET_CC_*|CFLAGS|CFLAGS_*|HOST_CFLAGS|HOST_CFLAGS_*|TARGET_CFLAGS|TARGET_CFLAGS_*|CXX|CXX_*|HOST_CXX|HOST_CXX_*|TARGET_CXX|TARGET_CXX_*|CXXFLAGS|CXXFLAGS_*|HOST_CXXFLAGS|HOST_CXXFLAGS_*|TARGET_CXXFLAGS|TARGET_CXXFLAGS_*|AR|AR_*|HOST_AR|HOST_AR_*|TARGET_AR|TARGET_AR_*|ARFLAGS|ARFLAGS_*|HOST_ARFLAGS|HOST_ARFLAGS_*|TARGET_ARFLAGS|TARGET_ARFLAGS_*|CXXSTDLIB|CXXSTDLIB_*|HOST_CXXSTDLIB|HOST_CXXSTDLIB_*|TARGET_CXXSTDLIB|TARGET_CXXSTDLIB_*|CRATE_CC_NO_DEFAULTS|CROSS_COMPILE)
      if [[ "$cc_env" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
        unset "$cc_env"
      else
        echo "unsupported inherited cc environment name: $cc_env" >&2
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

fn cc_environment_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let Some(sanitize_index) = run.find(CANONICAL_SANITIZE) else {
        return false;
    };
    if run[sanitize_index + CANONICAL_SANITIZE.len()..].contains(CANONICAL_SANITIZE) {
        return false;
    }
    let Some(clean_index) = run.find(CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = run.find(CARGO_BUILD) else {
        return false;
    };
    sanitize_index < clean_index && clean_index < build_index
}

#[test]
fn live_routed_rebuild_neutralizes_inherited_cc_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        cc_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited cc compiler/archiver/flag/wrapper/prefix authority and fail closed on unsafely named target-suffixed variables before rebuilding the measured candidate"
    );
}

#[test]
fn clean_rebuild_without_cc_sanitization_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!cc_environment_is_sanitized_before_rebuild(source));
}

#[test]
fn cross_compile_prefix_authority_must_not_survive() {
    let legacy_sanitize = CANONICAL_SANITIZE.replace("|CROSS_COMPILE)", ")");
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n",
        legacy_sanitize.replace('\n', "\n          ")
    );
    assert!(
        !cc_environment_is_sanitized_before_rebuild(&source),
        "CROSS_COMPILE can alter cc's native compiler/tool prefix before openssl-src removes it only from the later Configure child environment"
    );
}

#[test]
fn cc_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {}\n",
        CANONICAL_SANITIZE.replace('\n', "\n          ")
    );
    assert!(!cc_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_cc_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n",
        CANONICAL_SANITIZE.replace('\n', "\n          ")
    );
    assert!(cc_environment_is_sanitized_before_rebuild(&source));
}
