//! Executable supply-chain evidence contracts for the shared edge runtime.

use std::fs;

fn read_repository_file(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("required repository evidence {path} is missing: {error}"))
}

/// Matches only an exact, active entry in Cargo's root `[dependencies]` table.
fn contains_exact_root_dependency_line(manifest: &str, expected: &str) -> bool {
    let mut in_root_dependencies = false;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_root_dependencies = trimmed == "[dependencies]";
            continue;
        }

        if in_root_dependencies && !trimmed.starts_with('#') && trimmed == expected {
            return true;
        }
    }

    false
}

/// Matches the reviewed registry identity only when all fields belong to one Cargo lock package block.
fn lock_contains_reviewed_registry_package(
    lock: &str,
    name: &str,
    version: &str,
    checksum: &str,
) -> bool {
    let expected_name = format!("name = \"{name}\"");
    let expected_version = format!("version = \"{version}\"");
    let expected_checksum = format!("checksum = \"{checksum}\"");
    let expected_source = "source = \"registry+https://github.com/rust-lang/crates.io-index\"";

    lock.split("[[package]]").skip(1).any(|package| {
        let mut has_name = false;
        let mut has_version = false;
        let mut has_source = false;
        let mut has_checksum = false;

        for line in package.lines() {
            let trimmed = line.trim();
            has_name |= trimmed == expected_name;
            has_version |= trimmed == expected_version;
            has_source |= trimmed == expected_source;
            has_checksum |= trimmed == expected_checksum;
        }

        has_name && has_version && has_source && has_checksum
    })
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
            contains_exact_root_dependency_line(&manifest, required),
            "released Pingora dependencies must remain exact entries in the root dependency table: {required}"
        );
    }

    for forbidden in [
        "git = \"https://github.com/cloudflare/pingora.git\"",
        "09696b51bc59315353d96686355861604d0bb48c",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "released Pingora dependencies must not fall back to the superseded Pingora git source: {forbidden}"
        );
    }

    for forbidden in [
        "source = \"git+",
        "github.com/cloudflare/pingora.git",
        "09696b51bc59315353d96686355861604d0bb48c",
    ] {
        assert!(
            !lock.contains(forbidden),
            "Cargo.lock must not retain mutable git dependency identity after released-registry adoption: {forbidden}"
        );
    }

    for (name, checksum) in [
        (
            "pingora",
            "fc02712a3847828d6b798ecf31f0ac64e138df26b9513e20d52319cf3cecd11e",
        ),
        (
            "pingora-prometheus",
            "56fc7764cf4a5ff68aae5e373a4e8e2cbff077dd9dea975cc04ca9aa863abb2c",
        ),
    ] {
        assert!(
            lock_contains_reviewed_registry_package(&lock, name, "0.9.0", checksum),
            "Cargo.lock must bind the reviewed Pingora 0.9.0 registry checksum to package {name}: {checksum}"
        );
    }
}

/// A reviewed checksum elsewhere in the lockfile must not authenticate the Pingora package block.
#[test]
fn unrelated_lock_package_checksum_bait_is_not_pingora_evidence() {
    let reviewed_checksum =
        "fc02712a3847828d6b798ecf31f0ac64e138df26b9513e20d52319cf3cecd11e";
    let lock = format!(
        "[[package]]\nname = \"pingora\"\nversion = \"0.9.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"wrong\"\n\n[[package]]\nname = \"decoy\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"{reviewed_checksum}\"\n"
    );

    assert!(
        !lock_contains_reviewed_registry_package(&lock, "pingora", "0.9.0", reviewed_checksum),
        "an unrelated package carrying the reviewed checksum must not manufacture Pingora lock evidence"
    );
}

/// Dependency policy must fail closed and must not retain a mutable Pingora git-source exception.
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
    ] {
        assert!(
            policy.contains(required),
            "dependency policy must preserve the fail-closed supply-chain contract: {required}"
        );
    }

    for forbidden in ["allow-git", "https://github.com/cloudflare/pingora.git"] {
        assert!(
            !policy.contains(forbidden),
            "released registry-only Pingora consumption must not retain a mutable git-source allowance: {forbidden}"
        );
    }
}

/// A commented dependency line must never satisfy the executable released-supplier contract.
#[test]
fn commented_manifest_dependency_bait_is_not_active_evidence() {
    let required = "pingora = { version = \"=0.9.0\", features = [\"proxy\", \"openssl\"] }";
    let manifest = format!(
        "[dependencies]\n# {required}\npingora = {{ version = \"=0.8.0\" }}\n"
    );

    assert!(
        !contains_exact_root_dependency_line(&manifest, required),
        "commented dependency text must not manufacture released-supplier evidence"
    );
}

/// An exact-looking key outside `[dependencies]` must not satisfy the released-supplier contract.
#[test]
fn package_metadata_dependency_bait_is_not_active_evidence() {
    let required = "pingora = { version = \"=0.9.0\", features = [\"proxy\", \"openssl\"] }";
    let manifest = format!(
        "[package.metadata]\n{required}\n\n[dependencies]\npingora = {{ version = \"=0.8.0\", features = [\"proxy\", \"openssl\"] }}\n"
    );

    assert!(
        !contains_exact_root_dependency_line(&manifest, required),
        "metadata text outside the root dependency table must not manufacture released-supplier evidence"
    );
}
