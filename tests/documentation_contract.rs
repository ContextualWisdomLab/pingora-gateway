//! Executable acceptance for public API documentation completeness.

use std::fs;

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

fn cargo_rust_version(cargo_toml: &str) -> &str {
    cargo_toml
        .lines()
        .find_map(|line| {
            line.strip_prefix("rust-version = \"")
                .and_then(|value| value.strip_suffix('"'))
        })
        .expect("Cargo.toml must declare package.rust-version")
}

/// The library and hosted CI must fail closed when public Rust API documentation regresses.
#[test]
fn public_rustdoc_is_a_required_exact_head_gate() {
    let library = read_repository_file("src/lib.rs");
    let workflow = read_repository_file(".github/workflows/ci.yml");

    assert!(
        library.contains("#![deny(missing_docs)]"),
        "library crate must deny missing public documentation"
    );
    assert!(
        workflow.contains("RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --locked"),
        "hosted CI must build public documentation with warnings denied"
    );
}

/// The TRD must describe the lifecycle call used by the compiled composition root.
#[test]
fn trd_names_the_actual_server_lifecycle_entrypoint() {
    let trd = read_repository_file("TRD.md");
    let binary = read_repository_file("src/bin/cwl-pingora-gateway.rs");

    assert!(
        binary.contains("server.run(RunArgs::default());"),
        "compiled composition root must retain the documented graceful lifecycle entrypoint"
    );
    assert!(
        trd.contains("server.run(RunArgs::default())"),
        "TRD must name the lifecycle call actually used by the compiled composition root"
    );
    assert!(
        !trd.contains("delegates lifecycle handling to `Server::run_forever()`"),
        "TRD must not claim that the composition root invokes a different Pingora lifecycle method"
    );
}

/// The TRD minimum Rust version must stay identical to the package compiler contract.
#[test]
fn trd_tracks_the_declared_minimum_rust_version() {
    let cargo_toml = read_repository_file("Cargo.toml");
    let trd = read_repository_file("TRD.md");
    let rust_version = cargo_rust_version(&cargo_toml);

    assert!(
        trd.contains(&format!("minimum Rust `{rust_version}`")),
        "TRD must state Cargo.toml rust-version {rust_version} exactly"
    );
}
