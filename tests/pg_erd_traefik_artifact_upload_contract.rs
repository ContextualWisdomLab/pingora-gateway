//! Fail-closed artifact-upload contract for pg-erd Traefik reload characterization.
//!
//! The characterization is useful only when a failed execution leaves an explicit
//! evidence outcome. A missing receipt/log set must therefore fail the upload step
//! instead of being silently ignored.

const WORKFLOW: &str =
    include_str!("../.github/workflows/pg-erd-traefik-reload-characterization.yml");

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
