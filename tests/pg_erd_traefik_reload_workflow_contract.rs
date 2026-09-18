//! Executable contract for pg-erd-cloud Traefik file-provider reload characterization.
//!
//! This lane characterizes the exact consumer edge before any shared Pingora hot-reload
//! implementation is selected. It must remain manual, source-bound, payload-free, and
//! incapable of mutating the consumer repository.

const WORKFLOW: &str = include_str!("../.github/workflows/pg-erd-traefik-reload-characterization.yml");
const HARNESS: &str = include_str!("load/characterize_pg_erd_traefik_reload.sh");
const TRACEABILITY: &str =
    include_str!("../docs/doctoring/PG_ERD_TRAEFIK_RELOAD_TRACEABILITY.md");

const PG_ERD_SOURCE_SHA: &str = "8dc746920c12988f082e914879d95e13c9693535";
const TRAEFIK_IMAGE: &str = "traefik:v3.5.4@sha256:4df0a50fcf71b454c0d7ad17675776dc8d37359deae3291895bdaa008c1b9972";

#[test]
fn workflow_is_manual_protected_main_only_and_source_bound() {
    for required in [
        "workflow_dispatch:",
        "github.ref == 'refs/heads/main'",
        "persist-credentials: false",
        "repository: ContextualWisdomLab/pg-erd-cloud",
        PG_ERD_SOURCE_SHA,
        "test \"$(git rev-parse HEAD)\" = \"$PG_ERD_SOURCE_SHA\"",
        "permissions:\n  contents: read",
        "cancel-in-progress: false",
    ] {
        assert!(
            WORKFLOW.contains(required),
            "missing workflow authority bound: {required}"
        );
    }
    assert!(!WORKFLOW.contains("pull_request:"));
    assert!(!WORKFLOW.contains("push:"));
}

#[test]
fn harness_uses_the_exact_consumer_traefik_shape_without_product_policy_copying() {
    for required in [
        "compose.prod.yaml",
        "deploy/traefik/dynamic.yaml",
        "--providers.file.filename=/etc/traefik/dynamic.yaml",
        "--providers.file.watch=true",
        TRAEFIK_IMAGE,
        "./deploy/traefik/dynamic.yaml:/etc/traefik/dynamic.yaml:ro",
    ] {
        assert!(
            HARNESS.contains(required),
            "missing exact consumer-shape assertion: {required}"
        );
    }
    assert!(HARNESS.contains(
        "git diff --exit-code -- compose.prod.yaml deploy/traefik/dynamic.yaml"
    ));
}

#[test]
fn harness_distinguishes_in_place_and_atomic_replace_reload_semantics() {
    for required in [
        "in_place_reload_detected",
        "atomic_replace_detected",
        "atomic_replace_after_recreate_detected",
        "malformed_last_known_good",
        "recovery_detected",
        "concurrent_probe_failures",
        "X-CWL-Reload-Generation",
        "mv \"$replacement\" \"$dynamic_file\"",
        "--force-recreate traefik",
    ] {
        assert!(
            HARNESS.contains(required),
            "missing reload characterization: {required}"
        );
    }
}

#[test]
fn evidence_is_bounded_payload_free_and_uploaded_even_on_failure() {
    for required in [
        "PG_ERD_TRAEFIK_RELOAD_EVIDENCE",
        "pg-erd-traefik-reload.txt",
        "pg-erd-traefik-log.txt",
        "Upload Traefik reload characterization evidence",
        "if: ${{ always() }}",
        "retention-days: 7",
        "Authorization",
        "Cookie",
        "request body",
    ] {
        assert!(
            WORKFLOW.contains(required)
                || HARNESS.contains(required)
                || TRACEABILITY.contains(required),
            "missing evidence/privacy contract: {required}"
        );
    }
}

#[test]
fn traceability_keeps_product_ownership_and_pingora_implementation_undecided() {
    for required in [
        "Issue #109",
        "Admin Config",
        "runtime reload is supported",
        "controlled restart/redeployment",
        "last-known-good",
        "single-file bind mount",
        "atomic rename/replace",
        "No Pingora hot-reload implementation is selected",
        "pg-erd-cloud owner",
        "product route policy remains consumer-owned",
    ] {
        assert!(
            TRACEABILITY.contains(required),
            "missing traceability boundary: {required}"
        );
    }
}
