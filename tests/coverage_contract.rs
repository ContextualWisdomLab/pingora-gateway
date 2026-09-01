//! Executable acceptance for owned production source coverage policy.

use std::fs;

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

/// Stable-toolchain coverage must fail closed below complete line and region coverage.
#[test]
fn stable_production_coverage_is_gated_at_one_hundred_percent() {
    let workflow = read_repository_file(".github/workflows/ci.yml");

    for required in [
        "cargo install cargo-llvm-cov --version 0.9.0 --locked",
        "rustup component add llvm-tools-preview --toolchain 1.98.0",
        "cargo llvm-cov --all-targets --locked --fail-under-lines 100 --fail-under-regions 100",
    ] {
        assert!(
            workflow.contains(required),
            "CI must preserve exact-head owned production coverage contract: {required}"
        );
    }
}
