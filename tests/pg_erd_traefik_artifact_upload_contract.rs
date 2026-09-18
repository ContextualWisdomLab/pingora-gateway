//! Fail-closed artifact-upload contract for pg-erd Traefik reload characterization.
//!
//! The characterization is useful only when a failed execution leaves an explicit
//! evidence outcome. A missing receipt/log set must therefore fail the upload step
//! instead of being silently ignored. The uploaded evidence set must also carry its
//! own exact gateway and consumer source identity rather than relying on artifact
//! names or GitHub UI metadata after download.

const WORKFLOW: &str =
    include_str!("../.github/workflows/pg-erd-traefik-reload-characterization.yml");
const HARNESS: &str = include_str!("load/characterize_pg_erd_traefik_reload.sh");

#[test]
fn missing_characterization_artifacts_fail_closed() {
    assert!(
        WORKFLOW.contains("if: ${{ always() }}"),
        "characterization evidence upload must run even after a failed harness step"
    );
    assert!(
        WORKFLOW.contains("if-no-files-found: error"),
        "missing characterization artifacts must be an explicit workflow failure"
    );
    assert!(
        !WORKFLOW.contains("if-no-files-found: ignore"),
        "missing characterization artifacts must not be silently accepted"
    );
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
        .find("pg-erd-traefik-source-identity.txt")
        .expect("missing source-identity marker creation");
    let characterization = WORKFLOW
        .find("Characterize exact Traefik file-provider reload semantics")
        .expect("missing characterization step");
    assert!(
        marker < characterization,
        "source identity must be persisted before characterization can fail"
    );
}

#[test]
fn characterization_receipt_cross_binds_uploaded_source_marker() {
    assert!(
        WORKFLOW.contains("PG_ERD_TRAEFIK_SOURCE_IDENTITY: ${{ runner.temp }}/pg-erd-traefik-source-identity.txt"),
        "the harness must receive the exact uploaded source-identity marker path"
    );

    for required in [
        ": \"${EXPECTED_SHA:?EXPECTED_SHA must be set}\"",
        ": \"${PG_ERD_TRAEFIK_SOURCE_IDENTITY:?PG_ERD_TRAEFIK_SOURCE_IDENTITY must be set}\"",
        "record gateway_source_sha \"$EXPECTED_SHA\"",
        "record consumer_source_sha \"$PG_ERD_SOURCE_SHA\"",
        "record source_identity_sha256",
    ] {
        assert!(
            HARNESS.contains(required),
            "the receipt must cross-bind the detached source marker: {required}"
        );
    }
}
