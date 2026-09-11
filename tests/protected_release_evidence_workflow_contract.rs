const PROTECTED_RELEASE_EVIDENCE_WORKFLOW: &str =
    include_str!("../.github/workflows/protected-release-evidence.yml");

const CHECKOUT_ACTION_SHA: &str = "08c6903cd8c0fde910a37f88322edcfb5dd907a8";
const UPLOAD_ARTIFACT_ACTION_SHA: &str = "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";

#[test]
fn protected_release_evidence_runs_only_as_an_explicit_main_dispatch() {
    assert!(PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains("workflow_dispatch:"));
    assert!(!PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains("pull_request:"));
    assert!(!PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains("push:"));
    assert!(
        PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains("test \"$GITHUB_REF\" = \"refs/heads/main\"")
    );
    assert!(PROTECTED_RELEASE_EVIDENCE_WORKFLOW
        .contains("test \"$(git rev-parse HEAD)\" = \"$SOURCE_SHA\""));
}

#[test]
fn bundle_reuses_exact_successful_push_evidence_without_rebuilding() {
    for required in [
        "reproducibility_run_id:",
        "supply_chain_run_id:",
        "test \"$(jq -r '.status' <<<\"$run_json\")\" = \"completed\"",
        "test \"$(jq -r '.conclusion' <<<\"$run_json\")\" = \"success\"",
        "test \"$(jq -r '.head_sha' <<<\"$run_json\")\" = \"$SOURCE_SHA\"",
        "test \"$(jq -r '.head_branch' <<<\"$run_json\")\" = \"main\"",
        "test \"$(jq -r '.event' <<<\"$run_json\")\" = \"push\"",
        "release-reproducibility-${SOURCE_SHA}",
        "candidate-evidence-${SOURCE_SHA}",
    ] {
        assert!(
            PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains(required),
            "release evidence workflow must enforce {required}"
        );
    }

    for forbidden in [
        "cargo build",
        "cargo install",
        "docker build",
        "rustup toolchain install",
        "gh release create",
        "git tag",
    ] {
        assert!(
            !PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains(forbidden),
            "release evidence assembly must not create a second build or publication authority: {forbidden}"
        );
    }
}

#[test]
fn bundle_requires_protected_source_identity_and_binary_provenance() {
    for required in [
        "grep -Fx \"checkout_sha=${SOURCE_SHA}\"",
        "grep -Fx \"attested_source_sha=${SOURCE_SHA}\"",
        "grep -Fx \"signer_sha=${SOURCE_SHA}\"",
        "grep -Fx \"source_sha=${SOURCE_SHA}\"",
        "evidence/reproducibility/release-binaries/cwl-pingora-gateway",
        "evidence/reproducibility/release-binaries/cwl-pingora-pg-erd-migration",
        "sha256sum --check --strict -",
        "cmp --silent Cargo.lock evidence/supply-chain/Cargo.lock",
        "cmp --silent deny.toml evidence/supply-chain/deny.toml",
        "--cert-identity \"$CERT_IDENTITY\"",
        "--source-digest \"$SOURCE_SHA\"",
        "--signer-digest \"$SOURCE_SHA\"",
        "--deny-self-hosted-runners",
    ] {
        assert!(
            PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains(required),
            "release evidence workflow must enforce {required}"
        );
    }
}

#[test]
fn bundle_is_digest_bound_executable_and_explicitly_unpublished() {
    for required in [
        "chmod 0755",
        "evidence_kind=protected-source-release-evidence-bundle",
        "publication_state=unpublished",
        "SHA256SUMS",
        "--sort=name",
        "--mtime=\"@${source_date_epoch}\"",
        "--owner=0",
        "--group=0",
        "--numeric-owner",
        "-cf -",
        "gzip -n",
    ] {
        assert!(
            PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains(required),
            "release evidence bundle must preserve deterministic evidence: {required}"
        );
    }

    assert!(PROTECTED_RELEASE_EVIDENCE_WORKFLOW
        .contains(&format!("uses: actions/checkout@{CHECKOUT_ACTION_SHA}")));
    assert!(PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains(&format!(
        "uses: actions/upload-artifact@{UPLOAD_ARTIFACT_ACTION_SHA}"
    )));
    assert!(!PROTECTED_RELEASE_EVIDENCE_WORKFLOW.contains("publication_state=published"));
}
