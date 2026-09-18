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
fn harness_delegates_ephemeral_host_port_allocation_to_docker() {
    assert!(
        !HARNESS.contains("sock.bind((\"127.0.0.1\", 0))"),
        "bind-then-close host-port discovery leaves a TOCTOU window before Compose publishes the port"
    );
    for required in [
        "127.0.0.1::8080",
        "127.0.0.1::5432",
        "compose port traefik 8080",
        "host_port_allocation docker-managed-ephemeral",
    ] {
        assert!(
            HARNESS.contains(required),
            "Docker must atomically own ephemeral host-port publication: {required}"
        );
    }
}

#[test]
fn harness_distinguishes_observations_from_controlled_recreate_fallbacks() {
    for required in [
        "in_place_reload_detected",
        "lkg_generation_requires_recreate",
        "live_reload_observation_path",
        "malformed_last_known_good",
        "semantic_invalid_observable",
        "recovery_requires_recreate",
        "post_invalid_recovery_requires_recreate",
        "atomic_replace_detected",
        "atomic_replace_after_recreate_detected",
        "final_baseline_requires_recreate",
        "recovery_detected",
        "concurrent_probe_failures",
        "X-CWL-Reload-Generation",
        "mv \"$replacement\" \"$dynamic_file\"",
        "--force-recreate traefik",
        "[[ \"$concurrent_probe_samples\" -ge 20 ]]",
    ] {
        assert!(
            HARNESS.contains(required),
            "missing reload characterization: {required}"
        );
    }
}

#[test]
fn invalid_reload_observations_use_bounded_transition_windows() {
    for required in [
        "wait_for_observation_change",
        "malformed_observation_window_ms",
        "semantic_invalid_observation_window_ms",
        "render_generation \"semantic-invalid\"",
    ] {
        assert!(
            HARNESS.contains(required),
            "invalid reload observation must wait for a bounded externally observable transition: {required}"
        );
    }
    assert!(
        !HARNESS.contains("printf 'http:\\n  routers: [\\n' >\"$dynamic_file\"\n  sleep 2"),
        "malformed-input classification must not use a fixed sleep followed by one observation"
    );
    assert!(
        !HARNESS.contains("write_in_place \"$semantic_invalid\"\n  sleep 2"),
        "semantic-invalid classification must not use a fixed sleep followed by one observation"
    );
}

#[test]
fn post_invalid_recovery_requires_a_fresh_generation_marker() {
    for required in [
        "post_invalid_recovery_candidate",
        "render_generation \"post-invalid-recovery\"",
        "activate_generation_with_fallback \"$post_invalid_recovery_candidate\" \"post-invalid-recovery\" \"post_invalid_recovery_requires_recreate\"",
    ] {
        assert!(
            HARNESS.contains(required),
            "post-invalid recovery must prove a fresh generation transition: {required}"
        );
    }
    assert!(
        !HARNESS.contains("activate_generation_with_fallback \"$recovery_candidate\" \"recovery\" \"post_invalid_recovery_requires_recreate\""),
        "post-invalid recovery must not accept the already-visible recovery generation as fresh evidence"
    );
}

#[test]
fn recreate_fallback_preserves_the_container_logs_that_caused_it() {
    assert!(
        HARNESS.contains(">>\"$PG_ERD_TRAEFIK_LOG\""),
        "Traefik evidence must append snapshots instead of overwriting earlier containers"
    );

    let recreate = HARNESS
        .find("docker compose -f \"$compose_file\" up -d --no-deps --force-recreate traefik")
        .expect("missing controlled Traefik recreation");
    let prefix = &HARNESS[..recreate];
    let capture = prefix
        .rfind("capture_traefik_log pre-recreate")
        .expect("recreation must preserve the outgoing container log before replacement");
    assert!(
        capture < recreate,
        "outgoing Traefik logs must be captured before force-recreate destroys the container"
    );
}

#[test]
fn uploaded_traefik_log_digest_covers_the_final_cleanup_snapshot() {
    let cleanup_start = HARNESS.find("cleanup() {").expect("missing cleanup function");
    let cleanup_end = HARNESS[cleanup_start..]
        .find("\n}\ntrap cleanup EXIT")
        .map(|offset| cleanup_start + offset)
        .expect("missing cleanup function terminator");
    let cleanup = &HARNESS[cleanup_start..cleanup_end];

    let capture = cleanup
        .find("capture_traefik_log cleanup")
        .expect("cleanup must preserve a final Traefik log snapshot");
    let digest = cleanup
        .find("record traefik_log_sha256")
        .expect("the uploaded Traefik log digest must be finalized during cleanup");

    assert!(
        capture < digest,
        "the log digest must be recorded only after the final cleanup snapshot is appended"
    );
    assert_eq!(
        HARNESS.matches("record traefik_log_sha256").count(),
        1,
        "exactly one final digest must describe the uploaded Traefik log artifact"
    );
    assert!(
        !cleanup[digest..].contains("capture_traefik_log"),
        "no Traefik log snapshot may mutate the artifact after its digest is recorded"
    );
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
