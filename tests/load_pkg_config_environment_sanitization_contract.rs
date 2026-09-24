//! Fail-closed contract for inherited `pkg-config` authority in routed-load evidence.
//!
//! `openssl-sys` probes OpenSSL through `pkg-config` on Unix when no manual OpenSSL directory is
//! selected. `pkg-config` accepts executable, search-path, sysroot, cross-compilation, and linkage
//! policy from the process environment, including target-suffixed names that are not valid Bash
//! identifiers. The routed rebuild therefore clears shell-addressable authority and fails closed if
//! an inherited matching name cannot be unset safely by Bash.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CARGO_CLEAN: &str = "cargo clean --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";
const CANONICAL_SANITIZE: &str = r#"while IFS='=' read -r pkg_env _; do
  case "$pkg_env" in
    OPENSSL_DYNAMIC|OPENSSL_NO_PKG_CONFIG|PKG_CONFIG|PKG_CONFIG_*|HOST_PKG_CONFIG_*|TARGET_PKG_CONFIG_*)
      if [[ "$pkg_env" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
        unset "$pkg_env"
      else
        echo "unsupported inherited pkg-config environment name: $pkg_env" >&2
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

fn pkg_config_environment_is_sanitized_before_rebuild(source: &str) -> bool {
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
fn live_routed_rebuild_neutralizes_inherited_pkg_config_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        pkg_config_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited pkg-config/OpenSSL probe authority and fail closed on unsafely named target-suffixed variables before rebuilding the measured candidate"
    );
}

#[test]
fn clean_rebuild_without_pkg_config_sanitization_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!pkg_config_environment_is_sanitized_before_rebuild(source));
}

#[test]
fn pkg_config_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {}\n",
        CANONICAL_SANITIZE.replace('\n', "\n          ")
    );
    assert!(!pkg_config_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_pkg_config_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n",
        CANONICAL_SANITIZE.replace('\n', "\n          ")
    );
    assert!(pkg_config_environment_is_sanitized_before_rebuild(&source));
}
