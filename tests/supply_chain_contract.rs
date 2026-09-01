//! Executable supply-chain evidence contracts for the shared edge runtime.

use std::fs;

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
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

/// Pingora git dependencies must carry both immutable revision and exact package-version assertions.
#[test]
fn pinned_pingora_dependencies_are_not_wildcard_versions() {
    let manifest = read_repository_file("Cargo.toml");

    for required in [
        "pingora = { version = \"=0.8.0\", git = \"https://github.com/cloudflare/pingora.git\", rev = \"09696b51bc59315353d96686355861604d0bb48c\"",
        "pingora-prometheus = { version = \"=0.8.0\", git = \"https://github.com/cloudflare/pingora.git\", rev = \"09696b51bc59315353d96686355861604d0bb48c\"",
    ] {
        assert!(
            manifest.contains(required),
            "Pingora dependencies must be exact-version and exact-revision pinned: {required}"
        );
    }
}

/// Dependency policy must fail closed while distinguishing upstream maintenance status from security defects.
#[test]
fn dependency_source_and_advisory_policy_is_fail_closed() {
    let policy = read_repository_file("deny.toml");

    for required in [
        "unmaintained = \"workspace\"",
        "unsound = \"all\"",
        "\"CC0-1.0\"",
        "unknown-registry = \"deny\"",
        "unknown-git = \"deny\"",
        "required-git-spec = \"rev\"",
        "allow-registry = [\"https://github.com/rust-lang/crates.io-index\"]",
        "allow-git = [\"https://github.com/cloudflare/pingora.git\"]",
    ] {
        assert!(
            policy.contains(required),
            "dependency policy must preserve the fail-closed supply-chain contract: {required}"
        );
    }
}
