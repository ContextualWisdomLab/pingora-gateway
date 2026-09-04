//! Fail-closed acceptance for the compiler used by release-producing paths.

use std::fs;

/// Reads one repository file required by the toolchain contract.
fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

/// Requires every hosted Rust build/evidence path to select and verify the fixed point release.
#[test]
fn hosted_release_paths_select_rust_1_98_1() {
    for path in [
        ".github/workflows/ci.yml",
        ".github/workflows/supply-chain.yml",
    ] {
        let workflow = read_repository_file(path);

        assert!(
            workflow.contains("rustup toolchain install 1.98.1 --profile minimal"),
            "{path} must install Rust 1.98.1"
        );
        assert!(
            workflow.contains("rustup default 1.98.1"),
            "{path} must select Rust 1.98.1"
        );
        assert!(
            workflow.contains("rustc --version --verbose | grep -Fx 'release: 1.98.1'"),
            "{path} must verify the active compiler point release"
        );
        assert!(
            !workflow.contains("rustup default 1.98.0"),
            "{path} must not compile release evidence with the known-bad 1.98.0 compiler"
        );
    }
}

/// Requires the OCI release binary to select the fixed compiler before `cargo build` runs.
#[test]
fn image_build_selects_fixed_compiler_before_gateway_compilation() {
    let dockerfile = read_repository_file("Dockerfile");
    let install_position = dockerfile
        .find("rustup toolchain install 1.98.1 --profile minimal")
        .expect("Dockerfile must install Rust 1.98.1 before building the gateway");
    let select_position = dockerfile
        .find("rustup default 1.98.1")
        .expect("Dockerfile must select Rust 1.98.1 before building the gateway");
    let verify_position = dockerfile
        .find("rustc --version --verbose | grep -Fx 'release: 1.98.1'")
        .expect("Dockerfile must verify Rust 1.98.1 before building the gateway");
    let build_position = dockerfile
        .find("RUN cargo build --locked --release --bin cwl-pingora-gateway")
        .expect("Dockerfile must retain the locked release build");

    for (step, position) in [
        ("install", install_position),
        ("select", select_position),
        ("verify", verify_position),
    ] {
        assert!(
            position < build_position,
            "Rust 1.98.1 {step} must occur before release compilation"
        );
    }
    assert!(
        !dockerfile.contains("rustup default 1.98.0"),
        "the image build must never reselect the known-bad Rust 1.98.0 compiler"
    );
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
