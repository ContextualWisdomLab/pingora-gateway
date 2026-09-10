//! Executable contract for the representative many-core/NUMA shutdown profile.
//!
//! The profile is intentionally manual and fail-closed: ordinary hosted CI proves the harness and
//! policy shape, while only a runner that actually satisfies the topology floor may emit commercial
//! contention evidence for #46.

const WORKFLOW: &str = include_str!("../.github/workflows/numa-shutdown-profile.yml");
const PROFILE: &str = include_str!("shutdown_contention_profile.rs");
const ADR: &str = include_str!("../docs/adr/0013-representative-numa-shutdown-profile.md");

#[test]
fn profile_workflow_is_manual_and_does_not_create_routine_self_hosted_pressure() {
    assert!(WORKFLOW.contains("workflow_dispatch:"));
    assert!(!WORKFLOW.contains("pull_request:"));
    assert!(!WORKFLOW.contains("push:"));
    assert!(WORKFLOW.contains("runs-on: [self-hosted, linux]"));
    assert!(WORKFLOW.contains("persist-credentials: false"));
    assert!(WORKFLOW.contains("test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\""));
}

#[test]
fn profile_workflow_fails_closed_before_measurement_on_nonrepresentative_topology() {
    for required in [
        "MIN_ONLINE_CPUS: 64",
        "MIN_NUMA_NODES: 2",
        "SERVICE_THREADS: 64",
        "PARKED_CONNECTIONS: 4096",
        "PROFILE_ROUNDS: 25",
        "CLOSE_BOUND_MS: 1000",
    ] {
        assert!(
            WORKFLOW.contains(required),
            "missing topology/load contract: {required}"
        );
    }
    assert!(WORKFLOW.contains("lscpu -p=CPU,NODE,SOCKET"));
    assert!(WORKFLOW.contains("CWL_NUMA_PROFILE=1"));
    assert!(WORKFLOW.contains("--ignored --exact representative_numa_shutdown_profile"));
}

#[test]
fn profile_fixture_preserves_external_shutdown_and_scheduler_evidence_invariants() {
    for required in [
        "CARGO_BIN_EXE_cwl-pingora-gateway",
        "Connection: keep-alive",
        "/livez",
        "-TERM",
        "/proc/",
        "/task",
        "/schedstat",
        "voluntary_ctxt_switches",
        "nonvoluntary_ctxt_switches",
        "configured_proxy_service_threads",
        "parked_connections",
        "shutdown_close_p50_ms",
        "shutdown_close_p95_ms",
        "shutdown_close_p99_ms",
        "shutdown_close_max_ms",
        "survivors_at_close_bound",
    ] {
        assert!(
            PROFILE.contains(required),
            "missing profile evidence invariant: {required}"
        );
    }
    assert!(PROFILE.contains("#[ignore ="));
    assert!(PROFILE.contains("CWL_NUMA_PROFILE"));
}

#[test]
fn adr_keeps_performance_evidence_separate_from_correctness_and_release_credit() {
    for required in [
        "Status: Proposed",
        "64 logical CPUs",
        "2 NUMA nodes",
        "4096",
        "25",
        "one second",
        "independent `APPROVED`",
        "does not authorize",
    ] {
        assert!(ADR.contains(required), "missing ADR boundary: {required}");
    }
}
