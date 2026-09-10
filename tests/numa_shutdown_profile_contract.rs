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
    for required in [
        "Cpus_allowed_list",
        "filtered_topology",
        "process_allowed_cpu_list",
        "test \"$online_cpus\" -eq \"$(nproc)\"",
        "lscpu -p=CPU,NODE,SOCKET",
    ] {
        assert!(
            WORKFLOW.contains(required),
            "workflow must derive admission and receipt topology from one process-allowed CPU set: {required}"
        );
    }
    assert!(WORKFLOW.contains("CWL_NUMA_PROFILE: 1"));
    assert!(WORKFLOW.contains("--ignored --exact representative_numa_shutdown_profile"));
}

#[test]
fn representative_profile_runs_the_release_built_gateway_candidate() {
    for required in [
        "rustup toolchain install 1.98.1",
        "rustup default 1.98.1",
        "release: 1.98.1",
        "cargo build --release --locked --bin cwl-pingora-gateway",
        "CWL_PROFILE_GATEWAY_BINARY",
        "target/release/cwl-pingora-gateway",
        "gateway_binary_sha256",
    ] {
        assert!(
            WORKFLOW.contains(required),
            "representative workflow must bind the reviewed release compiler and release-built gateway candidate: {required}"
        );
    }
    assert!(!WORKFLOW.contains("rustup toolchain install 1.98.0"));
    assert!(PROFILE.contains("CWL_PROFILE_GATEWAY_BINARY"));
    assert!(PROFILE.contains("gateway_binary_sha256"));
    assert!(!PROFILE.contains("CARGO_BIN_EXE_cwl-pingora-gateway"));
    assert!(ADR.contains("Rust 1.98.1"));
}

#[test]
fn profile_fixture_preserves_external_shutdown_and_scheduler_evidence_invariants() {
    for required in [
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
        "pre_signal_jitter_ms",
        "shutdown_close_p50_ms",
        "shutdown_close_p95_ms",
        "shutdown_close_p99_ms",
        "shutdown_close_max_ms",
        "survivors_at_close_bound",
        "Cpus_allowed_list",
        "process_allowed_cpus",
        "same_task_set",
        "checked_delta",
        "checked_sub",
        "scheduler_sample_complete",
        "scheduler_task_set_changed",
        "scheduler_counter_regressed",
    ] {
        assert!(
            PROFILE.contains(required),
            "missing profile evidence invariant: {required}"
        );
    }
    assert!(PROFILE.contains("BTreeMap<u32, SchedulerCounters>"));
    assert!(!PROFILE.contains("saturating_delta"));
    assert!(PROFILE.contains("#[ignore ="));
    assert!(PROFILE.contains("CWL_NUMA_PROFILE"));
}

#[test]
fn adr_keeps_performance_evidence_separate_from_correctness_and_release_credit() {
    for required in [
        "Status: Proposed",
        "64 logical CPUs",
        "2 NUMA nodes",
        "process-allowed CPU set",
        "stable TID identity",
        "incomplete",
        "task-set churn",
        "4096",
        "25",
        "one second",
        "independent `APPROVED`",
        "does not authorize",
    ] {
        assert!(ADR.contains(required), "missing ADR boundary: {required}");
    }
}
