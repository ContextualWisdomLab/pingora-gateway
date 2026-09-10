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

/// Released Pingora dependencies must use exact registry versions and Cargo-recorded checksums.
#[test]
fn released_pingora_dependencies_are_exact_registry_packages() {
    let manifest = read_repository_file("Cargo.toml");
    let lock = read_repository_file("Cargo.lock");

    for required in [
        "pingora = { version = \"=0.9.0\", features = [\"proxy\", \"openssl\"] }",
        "pingora-prometheus = \"=0.9.0\"",
    ] {
        assert!(
            manifest.contains(required),
            "released Pingora dependencies must remain exact registry versions: {required}"
        );
    }

    for forbidden in ["github.com/cloudflare/pingora.git", "rev = "] {
        assert!(
            !manifest.contains(forbidden),
            "released Pingora dependencies must not fall back to mutable git-source consumption: {forbidden}"
        );
    }

    for required in [
        "fc02712a3847828d6b798ecf31f0ac64e138df26b9513e20d52319cf3cecd11e",
        "56fc7764cf4a5ff68aae5e373a4e8e2cbff077dd9dea975cc04ca9aa863abb2c",
    ] {
        assert!(
            lock.contains(required),
            "Cargo.lock must preserve the reviewed Pingora 0.9.0 registry checksum: {required}"
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
