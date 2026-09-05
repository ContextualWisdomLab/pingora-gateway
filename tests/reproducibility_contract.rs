//! Executable release-resolution contracts for the shared edge runtime.

const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");
const DOCKERFILE: &str = include_str!("../Dockerfile");

/// Release-relevant build paths must resolve exactly the reviewed dependency graph.
#[test]
fn release_builds_use_the_committed_lockfile() {
    assert!(
        CI_WORKFLOW.contains("cargo test --all-targets --locked"),
        "repository CI must test the committed dependency graph with --locked"
    );
    assert!(
        CI_WORKFLOW.contains("cargo clippy --all-targets --locked -- -D warnings"),
        "repository linting must compile the committed dependency graph with --locked"
    );
    assert!(
        DOCKERFILE.contains("COPY Cargo.toml Cargo.lock ./"),
        "the OCI builder must receive the reviewed Cargo.lock"
    );
    assert!(
        DOCKERFILE.contains("cargo build --locked --release --bin cwl-pingora-gateway"),
        "the OCI release build must reject lockfile drift"
    );
}
