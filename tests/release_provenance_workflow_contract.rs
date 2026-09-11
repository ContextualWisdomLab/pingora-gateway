const RELEASE_REPRODUCIBILITY_WORKFLOW: &str =
    include_str!("../.github/workflows/release-reproducibility.yml");

const ATTEST_ACTION_SHA: &str = "1e69f48acb82d1966a394da916b4c1698aa569d6";
const SIGNER_WORKFLOW: &str =
    "ContextualWisdomLab/pingora-gateway/.github/workflows/release-reproducibility.yml";

#[test]
fn release_candidate_requires_oidc_signed_provenance_for_both_binaries() {
    for permission in [
        "id-token: write",
        "attestations: write",
        "artifact-metadata: write",
    ] {
        assert!(
            RELEASE_REPRODUCIBILITY_WORKFLOW.contains(permission),
            "release provenance lane must grant {permission}"
        );
    }

    let compare = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("name: Compare release binaries byte for byte")
        .expect("byte-identical release comparison must run before provenance");
    let attest = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("name: Attest exact release binaries")
        .expect("release binaries must receive signed build provenance");
    let verify = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("name: Verify exact release binary provenance")
        .expect("generated provenance must be independently verified in the same run");
    let upload = RELEASE_REPRODUCIBILITY_WORKFLOW
        .find("name: Upload exact reproducibility evidence")
        .expect("reproducibility evidence upload must remain present");

    assert!(compare < attest && attest < verify && verify < upload);
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains(&format!("uses: actions/attest@{ATTEST_ACTION_SHA}")));

    for binary in ["cwl-pingora-gateway", "cwl-pingora-pg-erd-migration"] {
        let path = format!("release-binaries/{binary}");
        assert!(
            RELEASE_REPRODUCIBILITY_WORKFLOW.contains(&path),
            "attestation subject must include {binary}"
        );
        let verification = format!("gh attestation verify {path} --repo \"$GITHUB_REPOSITORY\"");
        assert!(
            RELEASE_REPRODUCIBILITY_WORKFLOW.contains(&verification),
            "release lane must verify provenance for {binary}"
        );
    }

    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains(&format!("--signer-workflow \"{SIGNER_WORKFLOW}\"")));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("--source-digest \"$EXPECTED_SHA\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("--signer-digest \"$SIGNER_SHA\""));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("--deny-self-hosted-runners"));
}

#[test]
fn pull_request_provenance_separates_exact_source_from_workflow_signer_commit() {
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW
        .contains("EXPECTED_SHA: ${{ github.event.pull_request.head.sha || github.sha }}"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("SIGNER_SHA: ${{ github.sha }}"));
    assert!(RELEASE_REPRODUCIBILITY_WORKFLOW.contains("signer_sha=%s"));
    assert!(!RELEASE_REPRODUCIBILITY_WORKFLOW.contains("--signer-digest \"$EXPECTED_SHA\""));
}

#[test]
fn provenance_action_is_immutable_and_has_no_mutable_tag_fallback() {
    assert_eq!(
        RELEASE_REPRODUCIBILITY_WORKFLOW
            .matches("uses: actions/attest@")
            .count(),
        1
    );
    assert!(!RELEASE_REPRODUCIBILITY_WORKFLOW.contains("actions/attest@v"));
    assert!(!RELEASE_REPRODUCIBILITY_WORKFLOW.contains("attest-build-provenance@"));
}
