//! Fail-closed acceptance for the compiler used by release-producing paths.

use serde_yaml::Value;
use std::{fs, path::Path};

const FIXED_INSTALL: &str = "rustup toolchain install 1.98.1 --profile minimal";
const FIXED_SELECT: &str = "rustup default 1.98.1";
const FIXED_VERIFY: &str = "rustc --version --verbose | grep -Fx 'release: 1.98.1'";

/// Reads one repository file required by the toolchain contract.
fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

/// Splits the workflow's top-level `jobs` mapping without accepting setup from sibling jobs.
fn workflow_jobs(workflow: &str) -> Vec<(String, String)> {
    let mut jobs = Vec::new();
    let mut current_name = None;
    let mut current_body = String::new();
    let mut in_jobs = false;

    for line in workflow.lines() {
        if line == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }

        let indent = line
            .len()
            .saturating_sub(line.trim_start_matches(' ').len());
        let trimmed = line.trim();
        if indent == 2 && trimmed.ends_with(':') && !trimmed.starts_with('-') {
            if let Some(name) = current_name.take() {
                jobs.push((name, std::mem::take(&mut current_body)));
            }
            current_name = Some(trimmed.trim_end_matches(':').to_owned());
            continue;
        }

        if current_name.is_some() {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    if let Some(name) = current_name {
        jobs.push((name, current_body));
    }
    jobs
}

/// Extracts shell scripts from each semantic YAML job so block indentation is not treated as shell text.
fn workflow_job_run_scripts(workflow: &str) -> Vec<(String, String)> {
    let document: Value = serde_yaml::from_str(workflow).unwrap_or_else(|error| {
        panic!("workflow YAML must parse before toolchain validation: {error}")
    });
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("workflow must contain a jobs mapping");

    jobs.iter()
        .map(|(name, job)| {
            let name = name
                .as_str()
                .expect("workflow job names must be strings")
                .to_owned();
            let mut scripts = String::new();
            if let Some(steps) = job.get("steps").and_then(Value::as_sequence) {
                for step in steps {
                    if let Some(run) = step.get("run").and_then(Value::as_str) {
                        scripts.push_str(run);
                        scripts.push('\n');
                    }
                }
            }
            (name, scripts)
        })
        .collect()
}

/// Rejects compiler selectors provided through GitHub Actions YAML environment mappings.
fn assert_environment_mapping_has_no_compiler_authority(
    context: &str,
    scope: &str,
    environment: Option<&Value>,
) {
    let Some(environment) = environment.and_then(Value::as_mapping) else {
        return;
    };

    for forbidden in ["RUSTC", "CARGO_BUILD_RUSTC"] {
        assert!(
            !environment
                .keys()
                .any(|key| key.as_str() == Some(forbidden)),
            "{context} must not set Cargo compiler authority {forbidden} in {scope} env"
        );
    }
}

/// Checks workflow-, job-, and step-level YAML env scopes for Cargo compiler overrides.
fn assert_no_yaml_compiler_environment(context: &str, workflow: &str, job_name: &str) {
    let document: Value = serde_yaml::from_str(workflow).unwrap_or_else(|error| {
        panic!("workflow YAML must parse before compiler environment validation: {error}")
    });
    assert_environment_mapping_has_no_compiler_authority(context, "workflow", document.get("env"));

    let job = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .and_then(|jobs| {
            jobs.iter()
                .find(|(name, _)| name.as_str() == Some(job_name))
                .map(|(_, job)| job)
        })
        .unwrap_or_else(|| panic!("{context} must exist in the semantic workflow jobs mapping"));
    assert_environment_mapping_has_no_compiler_authority(context, "job", job.get("env"));

    if let Some(steps) = job.get("steps").and_then(Value::as_sequence) {
        for (index, step) in steps.iter().enumerate() {
            let scope = format!("step {index}");
            assert_environment_mapping_has_no_compiler_authority(context, &scope, step.get("env"));
        }
    }
}

/// Mirrors the shell's removal of an unquoted backslash-newline pair before tokenization.
fn normalize_shell_continuations(script: &str) -> String {
    script.replace("\\\r\n", "").replace("\\\n", "")
}

/// Conservatively reduces shell quoting/escaping before comparing a security-sensitive word.
fn normalize_security_sensitive_shell_word(word: &str) -> String {
    let mut normalized = String::with_capacity(word.len());
    let mut characters = word.chars();

    while let Some(character) = characters.next() {
        match character {
            '\'' | '"' => {}
            '\\' => {
                if let Some(escaped) = characters.next() {
                    normalized.push(escaped);
                } else {
                    normalized.push('\\');
                }
            }
            _ => normalized.push(character),
        }
    }

    normalized
}

/// Returns byte positions for Cargo command tokens after shell-continuation normalization.
fn cargo_command_positions(script: &str) -> (String, Vec<usize>) {
    let normalized = normalize_shell_continuations(script);
    let mut positions = Vec::new();
    let mut body_offset = 0;

    for line in normalized.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let mut search_start = 0;
        for token in content.split_whitespace() {
            let relative = content[search_start..]
                .find(token)
                .expect("split token must exist in its source line");
            let token_start = search_start + relative;
            let command = normalize_security_sensitive_shell_word(token);
            let command = command.rsplit('/').next().unwrap_or(&command);
            if command == "cargo" {
                positions.push(body_offset + token_start);
            }
            search_start = token_start + token.len();
        }
        body_offset += line.len();
    }

    (normalized, positions)
}

/// Detects Cargo's explicit `+<toolchain>` selector across shell whitespace, continuations, and word quoting.
fn contains_explicit_cargo_toolchain_selector(script: &str) -> bool {
    let normalized = normalize_shell_continuations(script);

    normalized.lines().any(|line| {
        let tokens: Vec<_> = line.split_whitespace().collect();
        tokens.windows(2).any(|window| {
            let command = normalize_security_sensitive_shell_word(window[0]);
            let command = command.rsplit('/').next().unwrap_or(&command);
            let selector = normalize_security_sensitive_shell_word(window[1]);
            command == "cargo" && selector.starts_with('+') && selector.len() > 1
        })
    })
}

/// Rejects secondary toolchain authorities that could bypass the verified default compiler.
fn assert_no_alternate_toolchain_selector(context: &str, body: &str) {
    assert!(
        !contains_explicit_cargo_toolchain_selector(body),
        "{context} must not select a compiler with cargo +<toolchain>"
    );

    for line in body.lines() {
        let command = line.trim();
        for forbidden in [
            "RUSTC=",
            "CARGO_BUILD_RUSTC=",
            "RUSTUP_TOOLCHAIN",
            "rustup override",
            "rustup run",
            "dtolnay/rust-toolchain",
            "actions-rust-lang/setup-rust-toolchain",
        ] {
            assert!(
                !command.contains(forbidden),
                "{context} must not introduce secondary toolchain selector {forbidden}"
            );
        }

        if let Some((_, selection)) = command.split_once("rustup default ") {
            assert_eq!(
                selection.split_whitespace().next(),
                Some("1.98.1"),
                "{context} must not select a compiler other than Rust 1.98.1"
            );
        }
        if let Some((_, installation)) = command.split_once("rustup toolchain install ") {
            assert_eq!(
                installation.split_whitespace().next(),
                Some("1.98.1"),
                "{context} must not install an alternate release compiler"
            );
        }
    }
}

/// Requires every workflow job that executes host `cargo` to bind that job to Rust 1.98.1.
fn assert_host_cargo_jobs_use_fixed_compiler(path: &str, workflow: &str) {
    let raw_jobs = workflow_jobs(workflow);

    for (job_name, scripts) in workflow_job_run_scripts(workflow) {
        let (normalized_scripts, cargo_positions) = cargo_command_positions(&scripts);
        if cargo_positions.is_empty() {
            continue;
        }

        let context = format!("{path} job {job_name}");
        let raw_job = raw_jobs
            .iter()
            .find(|(name, _)| name == &job_name)
            .map(|(_, body)| body)
            .unwrap_or_else(|| panic!("{context} must use a canonical block-style job mapping"));

        assert_no_yaml_compiler_environment(&context, workflow, &job_name);
        assert_eq!(
            normalized_scripts.matches(FIXED_INSTALL).count(),
            1,
            "{context} must install Rust 1.98.1 exactly once"
        );
        assert_eq!(
            normalized_scripts.matches(FIXED_SELECT).count(),
            1,
            "{context} must select Rust 1.98.1 exactly once"
        );
        assert_eq!(
            normalized_scripts.matches(FIXED_VERIFY).count(),
            1,
            "{context} must verify Rust 1.98.1 exactly once"
        );

        let install_position = normalized_scripts
            .find(FIXED_INSTALL)
            .expect("installation count was checked");
        let select_position = normalized_scripts
            .find(FIXED_SELECT)
            .expect("selection count was checked");
        let verify_position = normalized_scripts
            .find(FIXED_VERIFY)
            .expect("verification count was checked");
        assert!(
            install_position < select_position && select_position < verify_position,
            "{context} must install, select, then verify Rust 1.98.1 in that order"
        );
        for cargo_position in cargo_positions {
            assert!(
                verify_position < cargo_position,
                "{context} must verify Rust 1.98.1 before every host cargo command"
            );
        }

        assert_no_alternate_toolchain_selector(&context, &normalized_scripts);
        assert_no_alternate_toolchain_selector(&context, raw_job);
        let after_verify = &normalized_scripts[verify_position + FIXED_VERIFY.len()..];
        assert!(
            !contains_explicit_cargo_toolchain_selector(after_verify),
            "{context} must not select a cargo toolchain after compiler verification"
        );
        for forbidden in [
            "rustup default ",
            "rustup toolchain install ",
            "RUSTC=",
            "CARGO_BUILD_RUSTC=",
            "RUSTUP_TOOLCHAIN",
            "rustup override",
            "rustup run",
            "dtolnay/rust-toolchain",
            "actions-rust-lang/setup-rust-toolchain",
        ] {
            assert!(
                !after_verify.contains(forbidden),
                "{context} must not change toolchain authority after compiler verification: {forbidden}"
            );
        }
    }
}

/// Requires every hosted host-Cargo build/evidence job to select and verify the fixed point release.
#[test]
fn hosted_release_paths_select_rust_1_98_1_per_job() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/supply-chain.yml",
    ] {
        let workflow = read_repository_file(path);
        assert_host_cargo_jobs_use_fixed_compiler(path, &workflow);
    }

    for path in ["rust-toolchain", "rust-toolchain.toml"] {
        assert!(
            !Path::new(path).exists(),
            "{path} would become a second repository-level compiler selector; govern it explicitly before adding it"
        );
    }
}

/// Requires the OCI release binary to select the fixed compiler before any `cargo build` runs.
#[test]
fn image_build_selects_fixed_compiler_before_gateway_compilation() {
    let dockerfile = read_repository_file("Dockerfile");
    let install_position = dockerfile
        .find(FIXED_INSTALL)
        .expect("Dockerfile must install Rust 1.98.1 before building the gateway");
    let select_position = dockerfile
        .find(FIXED_SELECT)
        .expect("Dockerfile must select Rust 1.98.1 before building the gateway");
    let verify_position = dockerfile
        .find(FIXED_VERIFY)
        .expect("Dockerfile must verify Rust 1.98.1 before building the gateway");
    let build_positions: Vec<_> = dockerfile
        .match_indices("cargo build")
        .map(|(index, _)| index)
        .collect();

    assert_eq!(
        build_positions.len(),
        1,
        "Dockerfile must keep exactly one gateway cargo build authority"
    );
    assert!(
        install_position < select_position && select_position < verify_position,
        "Dockerfile must install, select, then verify Rust 1.98.1 in that order"
    );
    assert!(
        verify_position < build_positions[0],
        "Dockerfile must verify Rust 1.98.1 before gateway compilation"
    );

    let compiler_to_build = &dockerfile[install_position..build_positions[0]];
    assert_no_alternate_toolchain_selector("Dockerfile compiler-to-build path", compiler_to_build);
    let after_verify = &dockerfile[verify_position + FIXED_VERIFY.len()..build_positions[0]];
    for forbidden in [
        "rustup default ",
        "rustup toolchain install ",
        "RUSTUP_TOOLCHAIN",
        "rustup override",
        "rustup run",
    ] {
        assert!(
            !after_verify.contains(forbidden),
            "Dockerfile must not change toolchain authority after verification: {forbidden}"
        );
    }
}

/// Rejects metadata that still permits the compiler release carrying the vtable miscompilation.
#[test]
fn crate_requires_fixed_rust_point_release() {
    let manifest = read_repository_file("Cargo.toml");

    assert!(
        manifest.contains("rust-version = \"1.98.1\""),
        "Cargo metadata must reject Rust 1.98.0 for this production candidate"
    );
}

/// Keeps Cargo's explicit toolchain shorthand fail-closed under valid shell word variants.
#[test]
fn cargo_toolchain_selector_detection_normalizes_shell_spacing() {
    for command in [
        "cargo +1.98.0 build",
        "cargo  +1.98.0 build",
        "cargo\t+1.98.0 build",
        "cargo \\\n  +1.98.0 build",
        "car\\\ngo +1.98.0 build",
        "/home/runner/.cargo/bin/cargo  +1.98.0 build",
        "cargo '+1.98.0' build",
        "cargo \"+1.98.0\" build",
        "cargo \\+1.98.0 build",
        "cargo \"+\"1.98.0 build",
        "\"/home/runner/.cargo/bin/cargo\" '+1.98.0' build",
        "car\"go\" '+1.98.0' build",
        "car\\go +1.98.0 build",
    ] {
        assert!(contains_explicit_cargo_toolchain_selector(command));
    }

    assert!(!contains_explicit_cargo_toolchain_selector(
        "cargo build --release --locked"
    ));
}

/// Proves host-Cargo job discovery cannot ignore valid shell whitespace around the command.
#[test]
fn host_cargo_job_detection_rejects_tab_separated_command_without_fixed_compiler() {
    let workflow = "jobs:\n  build:\n    steps:\n      - run: cargo\tbuild --release --locked\n";
    let result = std::panic::catch_unwind(|| {
        assert_host_cargo_jobs_use_fixed_compiler("synthetic.yml", workflow);
    });

    assert!(
        result.is_err(),
        "tab-separated cargo command must still require the fixed compiler contract"
    );
}

/// Proves YAML block-scalar indentation cannot hide a Cargo token split by shell continuation.
#[test]
fn host_cargo_job_detection_rejects_split_command_without_fixed_compiler() {
    let workflow = "jobs:\n  build:\n    steps:\n      - run: |\n          car\\\n          go build --release --locked\n";
    let result = std::panic::catch_unwind(|| {
        assert_host_cargo_jobs_use_fixed_compiler("synthetic.yml", workflow);
    });

    assert!(
        result.is_err(),
        "Cargo split across a shell continuation must still require the fixed compiler contract"
    );
}

/// Proves shell word quoting cannot hide a Cargo command from job-scoped compiler admission.
#[test]
fn host_cargo_job_detection_rejects_quoted_command_without_fixed_compiler() {
    let workflow = "jobs:\n  build:\n    steps:\n      - run: car\"go\" build --release --locked\n";
    let result = std::panic::catch_unwind(|| {
        assert_host_cargo_jobs_use_fixed_compiler("synthetic.yml", workflow);
    });

    assert!(
        result.is_err(),
        "quoted Cargo command token must still require the fixed compiler contract"
    );
}

/// Proves a verified default cannot be bypassed by Cargo's direct `RUSTC` environment override.
#[test]
fn host_cargo_job_rejects_rustc_environment_override_after_verification() {
    let workflow = "jobs:\n  build:\n    steps:\n      - run: |\n          rustup toolchain install 1.98.1 --profile minimal\n          rustup default 1.98.1\n          rustc --version --verbose | grep -Fx 'release: 1.98.1'\n          RUSTC=/tmp/rustc-1.98.0 cargo build --release --locked\n";
    let result = std::panic::catch_unwind(|| {
        assert_host_cargo_jobs_use_fixed_compiler("synthetic.yml", workflow);
    });

    assert!(
        result.is_err(),
        "Cargo's RUSTC environment authority must not bypass the verified Rust 1.98.1 compiler"
    );
}

/// Proves GitHub Actions YAML env scopes cannot rebind Cargo after standalone compiler verification.
#[test]
fn host_cargo_job_rejects_yaml_environment_compiler_override() {
    for workflow in [
        "env:\n  RUSTC: /tmp/rustc-1.98.0\njobs:\n  build:\n    steps:\n      - run: |\n          rustup toolchain install 1.98.1 --profile minimal\n          rustup default 1.98.1\n          rustc --version --verbose | grep -Fx 'release: 1.98.1'\n          cargo build --release --locked\n",
        "jobs:\n  build:\n    env:\n      CARGO_BUILD_RUSTC: /tmp/rustc-1.98.0\n    steps:\n      - run: |\n          rustup toolchain install 1.98.1 --profile minimal\n          rustup default 1.98.1\n          rustc --version --verbose | grep -Fx 'release: 1.98.1'\n          cargo build --release --locked\n",
        "jobs:\n  build:\n    steps:\n      - env:\n          RUSTC: /tmp/rustc-1.98.0\n        run: |\n          rustup toolchain install 1.98.1 --profile minimal\n          rustup default 1.98.1\n          rustc --version --verbose | grep -Fx 'release: 1.98.1'\n          cargo build --release --locked\n",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_host_cargo_jobs_use_fixed_compiler("synthetic.yml", workflow);
        });

        assert!(
            result.is_err(),
            "workflow/job/step YAML compiler authority must not bypass the verified Rust 1.98.1 compiler"
        );
    }
}

/// Proves shell word quoting/escaping cannot hide direct compiler authority from the shared workflow/OCI guard.
#[test]
fn alternate_toolchain_guard_rejects_shell_normalized_authority_words() {
    for command in [
        "rustup\tdefault\t1.98.0",
        "rust\"up\" default 1.98.0",
        "rust\\up toolchain install 1.98.0",
        "RUST\"C\"=/tmp/rustc-1.98.0 cargo build --release --locked",
        "CARGO_BUILD_RUST\\C=/tmp/rustc-1.98.0 cargo build --release --locked",
    ] {
        let result = std::panic::catch_unwind(|| {
            assert_no_alternate_toolchain_selector("synthetic compiler path", command);
        });

        assert!(
            result.is_err(),
            "normalized shell authority must be rejected: {command}"
        );
    }
}

/// Proves a valid Rust 1.98.1 verification cannot be followed by shell-obfuscated authority changes.
#[test]
fn host_cargo_job_rejects_shell_normalized_authority_after_verification() {
    for bypass in [
        "rustup\tdefault\t1.98.0",
        "rust\"up\" default 1.98.0",
        "RUST\"C\"=/tmp/rustc-1.98.0",
        "CARGO_BUILD_RUST\\C=/tmp/rustc-1.98.0",
    ] {
        let workflow = format!(
            "jobs:\n  build:\n    steps:\n      - run: |\n          rustup toolchain install 1.98.1 --profile minimal\n          rustup default 1.98.1\n          rustc --version --verbose | grep -Fx 'release: 1.98.1'\n          {bypass} cargo build --release --locked\n"
        );
        let result = std::panic::catch_unwind(|| {
            assert_host_cargo_jobs_use_fixed_compiler("synthetic.yml", &workflow);
        });

        assert!(
            result.is_err(),
            "post-verification shell authority must be rejected: {bypass}"
        );
    }
}
