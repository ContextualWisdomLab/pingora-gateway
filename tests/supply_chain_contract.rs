//! Executable supply-chain evidence contracts for the shared edge runtime.

use std::fs;

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

/// Candidate supply-chain evidence must be generated from the exact reviewed source revision.
#[test]
fn supply_chain_workflow_binds_evidence_to_exact_source() {
    let workflow = read_repository_file(".github/workflows/supply-chain.yml");

    for required in [
        "EXPECTED_SHA: ${{ github.event.pull_request.head.sha || github.sha }}",
        "ref: ${{ env.EXPECTED_SHA }}",
        "test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"",
        "cargo install cargo-deny --version 0.20.2 --locked",
        "cargo deny check advisories licenses sources bans",
        "anchore/sbom-action@3ad7283483fc7af8ff2b4ea19663c2d5ca935e26",
        "aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25",
        "output-file: candidate.spdx.json",
        "output: trivy-image.json",
        "sha256sum Cargo.lock deny.toml candidate.spdx.json trivy-image.json",
        "candidate-evidence-${{ env.EXPECTED_SHA }}",
    ] {
        assert!(
            workflow.contains(required),
            "supply-chain workflow must preserve exact-source evidence contract: {required}"
        );
    }
}

/// Dependency sources must fail closed except for crates.io and the explicitly pinned Pingora repository.
#[test]
fn dependency_source_policy_is_fail_closed() {
    let policy = read_repository_file("deny.toml");

    for required in [
        "unknown-registry = \"deny\"",
        "unknown-git = \"deny\"",
        "allow-registry = [\"https://github.com/rust-lang/crates.io-index\"]",
        "allow-git = [\"https://github.com/cloudflare/pingora.git\"]",
    ] {
        assert!(
            policy.contains(required),
            "dependency source policy must preserve fail-closed contract: {required}"
        );
    }
}
