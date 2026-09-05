//! Fail-closed acceptance for the compiler used by release-producing paths.

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

/// Detects Cargo's explicit `+<toolchain>` selector across shell whitespace and line continuations.
fn contains_explicit_cargo_toolchain_selector(body: &str) -> bool {
    let normalized = body.replace("\\\n", " ");

    normalized.lines().any(|line| {
        let tokens: Vec<_> = line.split_whitespace().collect();
        tokens.windows(2).any(|window| {
            let command = window[0].trim_matches(|character| character == '\'' || character == '"');
            let command = command.rsplit('/').next().unwrap_or(command);
            command == "cargo" && window[1].starts_with('+') && window[1].len() > 1
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
    for (job_name, job) in workflow_jobs(workflow) {
        let cargo_positions: Vec<_> = job
            .match_indices("cargo ")
            .map(|(index, _)| index)
            .collect();
        if cargo_positions.is_empty() {
            continue;
        }

        let context = format!("{path} job {job_name}");
        assert_eq!(
            job.matches(FIXED_INSTALL).count(),
            1,
            "{context} must install Rust 1.98.1 exactly once"
        );
        assert_eq!(
            job.matches(FIXED_SELECT).count(),
            1,
            "{context} must select Rust 1.98.1 exactly once"
        );
        assert_eq!(
            job.matches(FIXED_VERIFY).count(),
            1,
            "{context} must verify Rust 1.98.1 exactly once"
        );

        let install_position = job
            .find(FIXED_INSTALL)
            .expect("installation count was checked");
        let select_position = job.find(FIXED_SELECT).expect("selection count was checked");
        let verify_position = job
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

        assert_no_alternate_toolchain_selector(&context, &job);
        let after_verify = &job[verify_position + FIXED_VERIFY.len()..];
        assert!(
            !contains_explicit_cargo_toolchain_selector(after_verify),
            "{context} must not select a cargo toolchain after compiler verification"
        );
        for forbidden in [
            "rustup default ",
            "rustup toolchain install ",
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

/// Keeps Cargo's explicit toolchain shorthand fail-closed under valid shell whitespace variants.
#[test]
fn cargo_toolchain_selector_detection_normalizes_shell_spacing() {
    for command in [
        "cargo +1.98.0 build",
        "cargo  +1.98.0 build",
        "cargo\t+1.98.0 build",
        "cargo \\\n  +1.98.0 build",
        "/home/runner/.cargo/bin/cargo  +1.98.0 build",
    ] {
        assert!(contains_explicit_cargo_toolchain_selector(command));
    }

    assert!(!contains_explicit_cargo_toolchain_selector(
        "cargo build --release --locked"
    ));
}
