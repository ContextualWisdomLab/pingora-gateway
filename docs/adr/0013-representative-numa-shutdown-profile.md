# ADR 0013: Require representative NUMA shutdown contention evidence

Status: Proposed

## Problem

PR #70 closes the downstream lost-wakeup correctness defect after the gateway consumes Pingora 0.9.0, and PR #73 makes the proxy worker topology explicit. Neither result proves that parked HTTP/1 keep-alive cleanup scales acceptably on a many-core NUMA host. The ordinary hosted lane has four visible CPUs in one NUMA node/socket, while the supplier contention report that motivated #46 was observed on a 128-core x86 NUMA system.

A low-core GREEN therefore cannot be promoted into a commercial scaling claim. The remaining evidence must exercise the released supplier behavior with the gateway's actual configured proxy worker count and must distinguish socket-cleanup correctness from scheduler/contention characterization. It must also exercise the release-optimized gateway candidate rather than Cargo's integration-test/debug binary; test-profile timing is not release performance evidence.

Compiler identity is part of that evidence boundary. Rust 1.98.1 is the release compiler selected by prerequisite #56 because the Rust Release Team published it on September 3, 2026 to repair a vtable-generation miscompilation in Rust 1.98.0. A performance receipt built with 1.98.0 would therefore measure a compiler revision that the release path has already rejected and cannot be promoted as release-candidate evidence.

## Constraints

The profile remains inside Runtime Isolation and operability evidence. It does not move product authentication, business routing, Keyverse identity, Wardnet/EgressWeave policy, certificate authority, or consumer deployment authority into the gateway.

The profile must not become a routine pull-request workload because a self-hosted NUMA job would consume scarce runner capacity and could silently queue on unsuitable hardware. It must be explicitly dispatched and must fail closed before measurement unless the profiling process itself is allowed at least 64 logical CPUs spanning at least 2 NUMA nodes. The process-allowed CPU set is the single source for CPU-count admission and the filtered CPU→NUMA/socket topology recorded in the receipt; host CPUs outside the runner/process affinity or cpuset do not count. Those values are an evidence-admission floor, not a statement that 64 CPUs represent every production target. A deployment with materially larger topology still needs target-representative evidence.

The initial commercial characterization keeps 64 configured proxy workers, one Prometheus worker override, 4096 already-established reusable HTTP/1 keep-alive connections, 25 shutdown rounds, and the existing one second parked-connection close bound. Worker, connection, round, or close-bound reductions are not admissible merely to obtain a GREEN result.

GitHub only accepts `workflow_dispatch` when the workflow file exists on the repository default branch. The representative run therefore cannot be used as a pre-merge shortcut for this profiling-harness PR. Normal PR checks prove that the ignored Rust target compiles and that the dispatch/topology policy remains executable; after ordinary protected integration of the harness, an operator may dispatch the default-branch workflow against the exact integrated or descendant candidate ref. This sequencing keeps the scarce self-hosted lane out of routine PR execution without weakening review governance.

## Decision

Add an ignored Linux integration profile plus a manual `self-hosted, linux` workflow. Normal CI compiles the integration target and an executable contract verifies that the workflow remains manual, source-bound, topology-bounded, release-profile-bound, release-compiler-bound, and unable to masquerade as ordinary hosted evidence.

The manual workflow binds checkout to the dispatched exact SHA, requires the Rust 1.98.1 release compiler selected by #56 and OpenSSL build prerequisites, raises and verifies the file-descriptor soft limit, derives the process-allowed CPU list from `/proc/self/status`, filters `lscpu -p=CPU,NODE,SOCKET` to that same set, and rejects a host below the topology floor before it starts the gateway. The Rust profile independently applies the same process-allowed CPU boundary before it admits or records topology, so workflow preflight and runtime receipt cannot count different CPU sets.

Before measurement, the workflow runs `cargo build --release --locked --bin cwl-pingora-gateway` at that exact checkout, records the executable SHA-256 plus Rust/Cargo provenance, and passes the absolute release-binary path and expected digest to the ignored profile. The profile recomputes the digest before launch and fails closed on any mismatch. Cargo's `CARGO_BIN_EXE_cwl-pingora-gateway` integration-test artifact is deliberately not used for measured rounds because it is built in the unoptimized test profile and can materially distort scheduler, CPU, and shutdown-tail behavior.

For each round, the Rust profile launches that release-built `cwl-pingora-gateway` binary with the configured proxy worker count, establishes and verifies the health response on every keep-alive connection, proves the connections remain open and response-free immediately before SIGTERM, varies a small pre-signal scheduling delay across rounds, signals the real process, and records each socket's cleanup latency. A round fails if any parked connection survives the one-second bound or closes with unexpected response bytes/errors. Process exit must also remain bounded.

Scheduler evidence is collected from Linux `/proc/<pid>/task/<tid>/schedstat` and task status counters immediately before notification and repeatedly during socket cleanup while the process remains observable. Samples retain stable TID identity rather than only pre-aggregated counters. An incomplete sample (a partial per-TID read or missing required counter), task-set churn, or counter regression makes that round's scheduler delta unavailable; only complete samples with identical TID membership may compute per-TID monotonic deltas before aggregation. The receipt records scheduler completeness, task-set churn/counter-regression disposition, and aggregate CPU runtime, runqueue wait, timeslices, voluntary context switches, and nonvoluntary context switches only when that stable-TID contract holds. Linux documents the three schedstat fields as CPU time, runqueue wait time, and timeslice count. The workflow additionally wraps the profile with `perf stat` for task-clock, context switches, CPU migrations, and the futex syscall tracepoint when the host permits it; lack of `perf` permission is recorded rather than converted into fabricated evidence.

Every profiling round must produce a complete stable-TID scheduler delta before the workflow may report a representative GREEN. The profile may still record why a round became unavailable, but the receipt gate rejects any round with unavailable or incomplete scheduler evidence rather than silently promoting socket-close timing without the scheduler/contention evidence required by #46.

The resulting artifact records exact gateway SHA, release build profile and executable SHA-256, Rust/Cargo build provenance, exact Pingora package family, configured proxy and metrics worker counts, registered service count, process-allowed topology, connection/round pressure, per-round pre-signal jitter, p50/p95/p99/max socket-close latency, survivors at the close bound, maximum process-exit time, scheduler completeness/task-set disposition and valid deltas, filtered topology rows, and optional perf counters.

## Alternatives rejected

Running the existing four-CPU hosted capacity job at higher virtual-user counts was rejected because concurrency pressure does not create NUMA topology or equivalent scheduler behavior. Inferring worker count from `nproc` was rejected because #73 makes worker topology an explicit Admin Config input. Using `nproc` for CPU count while admitting unfiltered `lscpu` rows was rejected because container affinity/cpuset restrictions can make those views describe different CPU populations. Aggregate scheduler subtraction without TID identity was rejected because shutdown removes tasks and can make two aggregate samples incomparable. Allowing a representative run to GREEN when one or more rounds lacks a stable scheduler delta was rejected because #46 requires scheduler/contention characterization, not socket-close correctness alone. Profiling Cargo's `CARGO_BIN_EXE_cwl-pingora-gateway` integration-test artifact was rejected because Cargo builds it in the unoptimized test profile; a commercial scheduler/contention receipt must launch the exact release-built executable instead. Using Rust 1.98.0 for that release build was rejected because the Rust 1.98.1 point release fixes a vtable-generation miscompilation in 1.98.0 and #56 already establishes 1.98.1 as the gateway release compiler. Automatically triggering the self-hosted profile on every pull request was rejected because it would create routine queue pressure and could select an unrepresentative host. Adding a temporary write-capable workflow only to execute the branch before review was rejected because `workflow_dispatch` still requires the workflow definition on the default branch and a self-modifying execution path would violate the repository's evidence and workflow-ownership rules. Pinning or copying an unreleased supplier branch to recreate the historical shared-`Notify` implementation was rejected because mutable supplier code is not a release authority and would violate the gateway ownership boundary.

## Evidence and promotion

A GREEN artifact from this profile is necessary evidence for the performance/scaling half of #46, but it is not sufficient by itself for protected integration or release. The run must be reviewed against the actual target deployment topology, the historical supplier contention report, exact current source, the exact release compiler selected by #56, and the rest of #58's dependency graph. Every one of the configured repeated rounds must retain a complete stable-TID scheduler delta; a receipt with unavailable scheduler rounds remains RED even when every parked socket closes inside the one-second correctness bound. If the target environment is larger or materially different, the profile must be rerun there without reducing pressure.

This ADR does not authorize an independent `APPROVED` review, protected merge, immutable release, canary/shadow traffic, rollback acceptance, cutover, or Nginx/OpenResty removal. Those remain separate governance and migration gates.

## Traceability

- `tests/shutdown_contention_profile.rs` — real-process parked-connection profile and source-bound release-binary evidence receipt.
- `tests/numa_shutdown_profile_contract.rs` — fail-closed executable workflow/profile policy.
- `.github/workflows/numa-shutdown-profile.yml` — explicit manual execution lane for a representative self-hosted Linux host after the workflow definition is present on the default branch.
- ContextualWisdomLab/pingora-gateway#46 — canonical Runtime Isolation contention acceptance.
- ContextualWisdomLab/pingora-gateway#56 — Rust 1.98.1 release-compiler prerequisite.
- ContextualWisdomLab/pingora-gateway#73 — explicit proxy-worker topology prerequisite.
- cloudflare/pingora#844 — historical many-core shutdown-notification contention report.

## References

Cloudflare, Inc. (2026, September 9). *Pingora 0.9.0* [Software release]. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.9.0

GitHub. (2026). *Manually running a workflow*. GitHub Docs. https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow

Linux kernel developers. (2026). *Scheduler statistics*. The Linux Kernel documentation. https://docs.kernel.org/scheduler/sched-stats.html

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/

util-linux project. (2026). *lscpu(1) — display information about the CPU architecture*. Linux manual page. https://man7.org/linux/man-pages/man1/lscpu.1.html
