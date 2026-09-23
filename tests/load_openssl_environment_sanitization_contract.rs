//! Fail-closed contract for inherited OpenSSL build-selection authority in routed-load evidence.
//!
//! `openssl-sys` accepts manual OpenSSL installation and linkage authority from environment
//! variables, including target-prefixed forms. The exact Pingora graph also enables the vendored
//! OpenSSL feature, so `openssl-src` selects the Perl executable used to run OpenSSL `Configure`
//! from `OPENSSL_SRC_PERL` or `PERL`. Perl itself consumes a wider `PERL*` process-environment
//! namespace for startup switches, module search, I/O layers, Unicode handling, hash behavior,
//! signals, and other runtime semantics. Because the measured candidate links through this path,
//! the routed release rebuild clears manual OpenSSL authority, the explicit openssl-src executable
//! override, and all inherited Perl runtime authority before Cargo resolves and links the candidate.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const CANONICAL_SANITIZE: &str = "unset OPENSSL_DIR OPENSSL_LIB_DIR OPENSSL_INCLUDE_DIR OPENSSL_STATIC OPENSSL_LIBS OPENSSL_NO_VENDOR OPENSSL_CONFIG_DIR X86_64_UNKNOWN_LINUX_GNU_OPENSSL_DIR X86_64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR X86_64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR X86_64_UNKNOWN_LINUX_GNU_OPENSSL_STATIC X86_64_UNKNOWN_LINUX_GNU_OPENSSL_LIBS X86_64_UNKNOWN_LINUX_GNU_OPENSSL_NO_VENDOR X86_64_UNKNOWN_LINUX_GNU_OPENSSL_CONFIG_DIR";
const CANONICAL_OPENSSL_PERL_EXECUTABLE_SANITIZE: &str = "unset OPENSSL_SRC_PERL";
const CANONICAL_PERL_ENV_SANITIZE: &str = r#"while IFS='=' read -r perl_env _; do
  case "$perl_env" in
    PERL*)
      if [[ "$perl_env" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
        unset "$perl_env"
      else
        echo "unsupported inherited Perl environment name: $perl_env" >&2
        exit 1
      fi
      ;;
  esac
done < <(/usr/bin/env)"#;
const CARGO_CLEAN: &str = "cargo clean --release";
const CARGO_BUILD: &str = "cargo build --release --locked --bin cwl-pingora-pg-erd-migration";

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

fn active_lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

fn openssl_environment_is_sanitized_before_rebuild(source: &str) -> bool {
    let Some(run) = routed_run(source) else {
        return false;
    };
    let lines = active_lines(&run);
    let sanitize_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == CANONICAL_SANITIZE).then_some(index))
        .collect();
    let executable_positions: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            (*line == CANONICAL_OPENSSL_PERL_EXECUTABLE_SANITIZE).then_some(index)
        })
        .collect();
    let Some(perl_env_index) = run.find(CANONICAL_PERL_ENV_SANITIZE) else {
        return false;
    };
    if run[perl_env_index + CANONICAL_PERL_ENV_SANITIZE.len()..]
        .contains(CANONICAL_PERL_ENV_SANITIZE)
        || sanitize_positions.len() != 1
        || executable_positions.len() != 1
    {
        return false;
    }
    let Some(clean_index) = run.find(CARGO_CLEAN) else {
        return false;
    };
    let Some(build_index) = run.find(CARGO_BUILD) else {
        return false;
    };
    let executable_line_index = lines
        .iter()
        .position(|line| *line == CANONICAL_OPENSSL_PERL_EXECUTABLE_SANITIZE)
        .expect("single executable sanitizer was established above");
    let clean_line_index = lines
        .iter()
        .position(|line| *line == CARGO_CLEAN)
        .expect("clean command was established above");
    sanitize_positions[0] < clean_line_index
        && executable_line_index < clean_line_index
        && perl_env_index < clean_index
        && clean_index < build_index
}

#[test]
fn live_routed_rebuild_clears_inherited_openssl_build_authority() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        openssl_environment_is_sanitized_before_rebuild(&source),
        "routed evidence must clear inherited openssl-sys selection, openssl-src Perl executable override, and the complete inherited PERL* runtime namespace before rebuilding the measured candidate"
    );
}

#[test]
fn clean_rebuild_without_openssl_environment_sanitization_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Run routed pg-erd loopback traffic
        run: |
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;
    assert!(!openssl_environment_is_sanitized_before_rebuild(source));
}

#[test]
fn enumerating_only_known_perl_variables_is_rejected() {
    let legacy = "unset OPENSSL_SRC_PERL PERL PERL5OPT PERL5LIB PERLLIB PERL_USE_UNSAFE_INC";
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {legacy}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n"
    );
    assert!(
        !openssl_environment_is_sanitized_before_rebuild(&source),
        "an allowlist of known Perl variables leaves residual interpreter runtime authority such as PERLIO, PERL_UNICODE, and future PERL* controls"
    );
}

#[test]
fn perl_namespace_sanitization_after_build_is_rejected() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CANONICAL_OPENSSL_PERL_EXECUTABLE_SANITIZE}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n          {}\n",
        CANONICAL_PERL_ENV_SANITIZE.replace('\n', "\n          ")
    );
    assert!(!openssl_environment_is_sanitized_before_rebuild(&source));
}

#[test]
fn canonical_openssl_sanitization_before_clean_rebuild_is_admitted() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {ROUTED_STEP}\n        run: |\n          {CANONICAL_SANITIZE}\n          {CANONICAL_OPENSSL_PERL_EXECUTABLE_SANITIZE}\n          {}\n          {CARGO_CLEAN}\n          {CARGO_BUILD}\n",
        CANONICAL_PERL_ENV_SANITIZE.replace('\n', "\n          ")
    );
    assert!(openssl_environment_is_sanitized_before_rebuild(&source));
}
