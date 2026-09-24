//! Fail-closed contract for `load-contract` shell selection and command namespace.
//!
//! Install/loopback tooling keeps GitHub's reviewed explicit `bash` shell. The routed evidence step
//! uses a stricter custom shell that removes pre-start compatibility/POSIX authority and enables Bash
//! privileged mode so startup files, imported functions, and inherited shell-option authority cannot
//! run before the reviewed script. Inside that shell, alias, function, command-hash, and dynamically
//! loaded builtin mutations remain forbidden for protected evidence commands.

use serde_yaml::Value;
use std::fs;

const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const LOAD_JOB: &str = "load-contract";
const K6_INSTALL_STEP: &str = "Install checksum-pinned k6 2.2.0";
const LOOPBACK_STEP: &str = "Exercise concurrent loopback traffic contract";
const ROUTED_STEP: &str = "Run routed pg-erd loopback traffic";
const ROUTED_SHELL: &str = "/usr/bin/env -u POSIXLY_CORRECT -u BASH_COMPAT /bin/bash --noprofile --norc -p -eo pipefail {0}";
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

fn line_mutates_hash_namespace(line: &str) -> bool {
    let mut tokens = line.split_ascii_whitespace();
    match tokens.next() {
        Some("hash") => true,
        Some("builtin" | "command") => matches!(tokens.next(), Some("hash")),
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
                    && !line_mutates_hash_namespace(line)
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
            (ROUTED_STEP, ROUTED_SHELL),
        ]
        && routed_step_preserves_command_namespace(steps)
}

fn canonical_source(run: &str) -> String {
    format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {K6_INSTALL_STEP}\n        shell: bash\n      - name: {LOOPBACK_STEP}\n        shell: bash\n      - name: {ROUTED_STEP}\n        shell: {ROUTED_SHELL}\n        run: |\n{}",
        run.lines()
            .map(|line| format!("          {line}\n"))
            .collect::<String>()
    )
}

#[test]
fn live_load_job_shell_surface_is_canonical() {
    let source = fs::read_to_string(CI_WORKFLOW).expect("CI workflow should be readable UTF-8");
    assert!(
        load_shell_surface_is_canonical(&source),
        "load-contract must use only the reviewed shells and preserve the routed command namespace"
    );
}

#[test]
fn additional_privileged_shell_template_must_not_claim_release_evidence() {
    let source = format!(
        "jobs:\n  load-contract:\n    steps:\n      - name: {K6_INSTALL_STEP}\n        shell: bash\n      - name: {LOOPBACK_STEP}\n        shell: bash\n      - name: Replace provenance comparator without sudo in run text\n        shell: sudo bash {{0}}\n        run: install -m 0755 /bin/true /usr/bin/cmp\n      - name: {ROUTED_STEP}\n        shell: {ROUTED_SHELL}\n        run: echo routed\n"
    );
    assert!(!load_shell_surface_is_canonical(&source));
}

#[test]
fn plain_or_changed_routed_shell_is_rejected() {
    for shell in ["bash", "bash --noprofile --norc {0}", "/bin/bash -p {0}"] {
        let source = format!(
            "jobs:\n  load-contract:\n    steps:\n      - name: {K6_INSTALL_STEP}\n        shell: bash\n      - name: {LOOPBACK_STEP}\n        shell: bash\n      - name: {ROUTED_STEP}\n        shell: {shell}\n"
        );
        assert!(!load_shell_surface_is_canonical(&source));
    }
}

#[test]
fn canonical_shell_surface_is_admitted() {
    assert!(load_shell_surface_is_canonical(&canonical_source("echo measured")));
}

#[test]
fn routed_alias_rebinding_must_not_claim_release_evidence() {
    for invocation in [
        "shopt -s expand_aliases",
        "alias cargo=/bin/true",
        "unalias cargo",
        "builtin alias cargo=/bin/true",
        "command unalias cargo",
    ] {
        assert!(!load_shell_surface_is_canonical(&canonical_source(invocation)));
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
        assert!(!load_shell_surface_is_canonical(&canonical_source(declaration)));
    }
}

#[test]
fn dynamic_builtin_rebinding_must_not_claim_release_evidence() {
    for invocation in [
        "enable -f /tmp/fake-cargo.so cargo",
        "builtin enable -f /tmp/fake-cargo.so cargo",
        "command enable -f /tmp/fake-cargo.so cargo",
    ] {
        assert!(!load_shell_surface_is_canonical(&canonical_source(invocation)));
    }
}

#[test]
fn routed_hash_table_rebinding_must_not_claim_release_evidence() {
    for invocation in [
        "hash -p /bin/true cargo",
        "builtin hash -p /bin/true cargo",
        "command hash -p /bin/true cargo",
    ] {
        assert!(!load_shell_surface_is_canonical(&canonical_source(invocation)));
    }
}

#[test]
fn legitimate_cleanup_function_remains_admitted() {
    assert!(load_shell_surface_is_canonical(&canonical_source(
        "cleanup() {\n  :\n}"
    )));
}
