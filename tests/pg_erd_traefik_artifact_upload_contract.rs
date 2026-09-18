//! Fail-closed artifact-upload contract for pg-erd Traefik reload characterization.
//!
//! The characterization is useful only when a failed execution leaves an explicit
//! evidence outcome. A missing receipt/log set must therefore fail the workflow
//! instead of being silently accepted. The uploaded evidence set must also carry its
//! own exact gateway and consumer source identity rather than relying on artifact
//! names or GitHub UI metadata after download.

const WORKFLOW: &str =
    include_str!("../.github/workflows/pg-erd-traefik-reload-characterization.yml");

#[test]
fn missing_characterization_artifacts_fail_closed() {
    assert!(
        WORKFLOW.contains("if: ${{ always() }}"),
        "characterization evidence handling must run even after a failed harness step"
    );
    assert!(
        WORKFLOW.contains("if-no-files-found: error"),
        "an entirely missing characterization artifact set must be an explicit upload failure"
    );
    assert!(
        !WORKFLOW.contains("if-no-files-found: ignore"),
        "missing characterization artifacts must not be silently accepted"
    );
    for required in [
        "test -f \"$PG_ERD_TRAEFIK_SOURCE_IDENTITY\"",
        "test -f \"$PG_ERD_TRAEFIK_RELOAD_EVIDENCE\"",
        "test -f \"$PG_ERD_TRAEFIK_LOG\"",
        "test -f \"$PG_ERD_TRAEFIK_PROBE_LOG\"",
    ] {
        assert!(
            WORKFLOW.contains(required),
            "every required evidence file must be checked before upload: {required}"
        );
    }
}

#[test]
fn artifact_set_self_binds_exact_gateway_and_consumer_sources() {
    for required in [
        "pg-erd-traefik-source-identity.txt",
        "gateway_source_sha=%s\\n",
        "consumer_source_sha=%s\\n",
        "\"$EXPECTED_SHA\"",
        "\"$PG_ERD_SOURCE_SHA\"",
    ] {
        assert!(
            WORKFLOW.contains(required),
            "uploaded characterization evidence must self-bind exact source identity: {required}"
        );
    }

    let marker = WORKFLOW
        .find("Persist exact characterization source identity")
        .expect("missing source-identity marker creation");
    let characterization = WORKFLOW
        .find("Characterize exact Traefik file-provider reload semantics")
        .expect("missing characterization step");
    let binding = WORKFLOW
        .find("Cross-bind characterization receipt and artifact set")
        .expect("missing receipt cross-binding step");
    let upload = WORKFLOW
        .find("Upload Traefik reload characterization evidence")
        .expect("missing characterization upload step");
    assert!(
        marker < characterization && characterization < binding && binding < upload,
        "source marker must precede characterization, and receipt cross-binding must precede upload"
    );
}

#[test]
fn characterization_receipt_cross_binds_uploaded_source_marker() {
    for required in [
        "grep -Fx -- \"gateway_source_sha=$EXPECTED_SHA\" \"$PG_ERD_TRAEFIK_SOURCE_IDENTITY\"",
        "grep -Fx -- \"consumer_source_sha=$PG_ERD_SOURCE_SHA\" \"$PG_ERD_TRAEFIK_SOURCE_IDENTITY\"",
        "grep -Fx -- \"consumer_source_sha=$PG_ERD_SOURCE_SHA\" \"$PG_ERD_TRAEFIK_RELOAD_EVIDENCE\"",
        "recorded_source_identity_sha256=\"$(sha256sum",
        "printf 'gateway_source_sha=%s\\nsource_identity_sha256=%s\\n'",
        "\"$recorded_source_identity_sha256\"",
    ] {
        assert!(
            WORKFLOW.contains(required),
            "receipt must cross-bind the detached source marker before upload: {required}"
        );
    }
}
