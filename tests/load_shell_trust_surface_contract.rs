//! Fail-closed contract for `load-contract` shell selection and command namespace.
//!
//! Run-line privilege review is insufficient when a step can replace GitHub's normal shell with an
//! arbitrary command template such as `sudo bash {0}`. The load job therefore permits explicit
//! `shell:` only on the three reviewed traffic/tool steps, and each must be exactly `bash`; all
//! other steps stay on the GitHub-hosted runner's default shell contract. The routed evidence shell
//! also rejects alias, function, and dynamically loaded builtin mutations that can redirect
//! protected evidence commands before `PATH` resolution.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const K6_INSTALL_STEP: &str = "Install checksum-pinned k6 2.2.0";
const LOOPBACK_STEP: &str = "Exercise concurrent loopback traffic contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const PROTECTED_COMMANDS: &[&str] = &[
    "cargo",
    "git",
    "rustc",
    "curl",
    "kill",
    "sleep",
    "rm",
    "mkdir",
    "install",
    "sha256sum",
    "tar",
    "cmp",
    "target/release/cwl-pingora-pg-erd-migration",
    "/usr/local/bin/k6",
];

fn active_shell_lines(run: &str) -> impl Iterator<Item = &str> {
    run.lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

fn line_mutates_alias_namespace(line: &str) -> bool {
    if line.split_ascii_whitespace().any(|token| token == "expand_aliases") {
        return true;
    }

    let mut tokens = line.split_ascii_whitespace();
    match tokens.next() {
        Some("alias" | "unalias") => true,
        Some("builtin" | "command") => matches!(tokens.next(), Some("alias" | "unalias")),
        _ => false,
    }
}

fn line_mutates_builtin_namespace(line: &str) -> bool {
    let mut tokens = line.split_ascii_whitespace();
    match tokens.next() {
        Some("enable") => true,
        Some("builtin" | "command") => matches!(tokens.next(), Some("enable")),
        _ => false,
    }
}

fn function_name_from_line(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("function ") {
        let name = rest.split_ascii_whitespace().next()?;
        return Some(name.strip_suffix("()").unwrap_or(name));
    }

    let open_paren = trimmed.find('(')?;
    let name = trimmed[..open_paren].trim_end();
    if name.is_empty() {
        return None;
    }
    let after_open = trimmed[open_paren + 1..].trim_start();
    after_open.starts_with(')').then_some(name)
}

fn line_rebinds_protected_command(line: &str) -> bool {
    function_name_from_line(line).is_some_and(|name| PROTECTED_COMMANDS.contains(&name))
}

fn routed_step_preserves_command_namespace(steps: &[Value]) -> bool {
    let routed = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(ROUTED_STEP))
        .collect::<Vec<_>>();
    if routed.len() != 1 {
        return false;
    }

    routed[0]
        .get("run")
        .and_then(Value::as_str)
        .is_none_or(|run| {
            active_shell_lines(run).all(|line| {
                !line_mutates_alias_namespace(line)
                    && !line_mutates_builtin_namespace(line)
                    && !line_rebinds_protected_command(line)
            })
        })
}

fn load_shell_surface_is_canonical(source: &str) -> bool {
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

    let observed = steps
        .iter()
        .filter_map(|step| {
            let shell = step.get("shell").and_then(Value::as_str)?;
            let name = step.get("name").and_then(Value::as_str).unwrap_or("");
            Some((name, shell))
        })
        .collect::<Vec<_>>();

    observed
        == [
            (K6_INSTALL_STEP, "bash"),
            (LOOPBACK_STEP, "bash"),
            (ROUTED_STEP, "bash"),
        ]
        && routed_step_preserves_command_namespace(steps)
}

#[test]
fn live_load_job_shell_surface_is_canonical() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_shell_surface_is_canonical(&source),
        "load-contract must use only the reviewed Bash shells and preserve the routed command namespace"
    );
}

#[test]
fn additional_privileged_shell_template_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
        run: echo install
      - name: Exercise concurrent loopback traffic contract
        shell: bash
        run: echo loopback
      - name: Replace provenance comparator without sudo in run text
        shell: sudo bash {0}
        run: install -m 0755 /bin/true /usr/bin/cmp
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: echo routed
"#;

    assert!(
        !load_shell_surface_is_canonical(source),
        "a custom shell template can acquire root before run-line privilege checks see any sudo token"
    );
}

#[test]
fn changed_routed_shell_is_rejected() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
      - name: Exercise concurrent loopback traffic contract
        shell: bash
      - name: Run routed pg-erd loopback traffic
        shell: bash --noprofile --norc {0}
"#;

    assert!(!load_shell_surface_is_canonical(source));
}

#[test]
fn canonical_shell_surface_is_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
      - name: Exercise concurrent loopback traffic contract
        shell: bash
      - name: Run routed pg-erd loopback traffic
        shell: bash
"#;
    assert!(load_shell_surface_is_canonical(source));
}

#[test]
fn routed_alias_rebinding_must_not_claim_release_evidence() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
        run: echo install
      - name: Exercise concurrent loopback traffic contract
        shell: bash
        run: echo loopback
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          shopt -s expand_aliases
          alias cargo=/bin/true
          cargo clean --release
          cargo build --release --locked --bin cwl-pingora-pg-erd-migration
"#;

    assert!(
        !load_shell_surface_is_canonical(source),
        "non-interactive Bash can enable alias expansion and redirect bare cargo without changing PATH"
    );
}

#[test]
fn qualified_alias_builtins_are_rejected() {
    for invocation in [
        "builtin alias cargo=/bin/true",
        "command alias cargo=/bin/true",
        "unalias cargo",
        "builtin unalias cargo",
        "command unalias cargo",
    ] {
        let source = format!(
            "jobs:\n  load-contract:\n    steps:\n      - name: {K6_INSTALL_STEP}\n        shell: bash\n      - name: {LOOPBACK_STEP}\n        shell: bash\n      - name: {ROUTED_STEP}\n        shell: bash\n        run: |\n          {invocation}\n"
        );
        assert!(!load_shell_surface_is_canonical(&source));
    }
}

#[test]
fn routed_function_rebinding_must_not_claim_release_evidence() {
    for declaration in [
        "cargo() { :; }",
        "cargo () { :; }",
        "function cargo { :; }",
        "function cargo() { :; }",
    ] {
        let source = format!(
            "jobs:\n  load-contract:\n    steps:\n      - name: {K6_INSTALL_STEP}\n        shell: bash\n      - name: {LOOPBACK_STEP}\n        shell: bash\n      - name: {ROUTED_STEP}\n        shell: bash\n        run: |\n          {declaration}\n          cargo clean --release\n          cargo build --release --locked --bin cwl-pingora-pg-erd-migration\n"
        );
        assert!(
            !load_shell_surface_is_canonical(&source),
            "Bash resolves shell functions before PATH, so a same-shell cargo function can bypass the reviewed executable authority"
        );
    }
}

#[test]
fn dynamic_builtin_rebinding_must_not_claim_release_evidence() {
    for invocation in [
        "enable -f /tmp/fake-cargo.so cargo",
        "builtin enable -f /tmp/fake-cargo.so cargo",
        "command enable -f /tmp/fake-cargo.so cargo",
    ] {
        let source = format!(
            "jobs:\n  load-contract:\n    steps:\n      - name: {K6_INSTALL_STEP}\n        shell: bash\n      - name: {LOOPBACK_STEP}\n        shell: bash\n      - name: {ROUTED_STEP}\n        shell: bash\n        run: |\n          {invocation}\n          cargo clean --release\n          cargo build --release --locked --bin cwl-pingora-pg-erd-migration\n"
        );
        assert!(
            !load_shell_surface_is_canonical(&source),
            "Bash enable -f can load a same-name builtin that resolves before PATH and bypasses the reviewed executable authority"
        );
    }
}

#[test]
fn legitimate_cleanup_function_remains_admitted() {
    let source = r#"
jobs:
  load-contract:
    steps:
      - name: Install checksum-pinned k6 2.2.0
        shell: bash
      - name: Exercise concurrent loopback traffic contract
        shell: bash
      - name: Run routed pg-erd loopback traffic
        shell: bash
        run: |
          cleanup() {
            :
          }
"#;
    assert!(load_shell_surface_is_canonical(source));
}

#[test]
fn routed_hash_table_rebinding_must_not_claim_release_evidence() {
    for invocation in [
        "hash -p /bin/true cargo",
        "builtin hash -p /bin/true cargo",
        "command hash -p /bin/true cargo",
    ] {
        let source = format!(
            "jobs:\n  load-contract:\n    steps:\n      - name: {K6_INSTALL_STEP}\n        shell: bash\n      - name: {LOOPBACK_STEP}\n        shell: bash\n      - name: {ROUTED_STEP}\n        shell: bash\n        run: |\n          {invocation}\n          cargo clean --release\n          cargo build --release --locked --bin cwl-pingora-pg-erd-migration\n"
        );
        assert!(
            !load_shell_surface_is_canonical(&source),
            "Bash hash -p can bind a bare protected command to an arbitrary pathname without changing PATH"
        );
    }
}
