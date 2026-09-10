# ADR 0013: Require representative NUMA shutdown contention evidence

Status: Proposed

## Problem

PR #70 closes the downstream lost-wakeup correctness defect after the gateway consumes Pingora 0.9.0, and PR #73 makes the proxy worker topology explicit. Neither result proves that parked HTTP/1 keep-alive cleanup scales acceptably on a many-core NUMA host. The ordinary hosted lane has four visible CPUs in one NUMA node/socket, while the supplier contention report that motivated #46 was observed on a 128-core x86 NUMA system.

A low-core GREEN therefore cannot be promoted into a commercial scaling claim. The remaining evidence must exercise the released supplier behavior with the gateway's actual configured proxy worker count and must distinguish socket-cleanup correctness from scheduler/contention characterization.

## Constraints

The profile remains inside Runtime Isolation and operability evidence. It does not move product authentication, business routing, Keyverse identity, Wardnet/EgressWeave policy, certificate authority, or consumer deployment authority into the gateway.

The profile must not become a routine pull-request workload because a self-hosted NUMA job would consume scarce runner capacity and could silently queue on unsuitable hardware. It must be explicitly dispatched and must fail closed before measurement unless the runner exposes at least 64 logical CPUs and 2 NUMA nodes. Those values are an evidence-admission floor, not a statement that 64 CPUs represent every production target. A deployment with materially larger topology still needs target-representative evidence.

The initial commercial characterization keeps 64 configured proxy workers, one Prometheus worker override, 4096 already-established reusable HTTP/1 keep-alive connections, 25 shutdown rounds, and the existing one second parked-connection close bound. Worker, connection, round, or close-bound reductions are not admissible merely to obtain a GREEN result.

## Decision

Add an ignored Linux integration profile plus a manual `self-hosted, linux` workflow. Normal CI compiles the integration target and an executable contract verifies that the workflow remains manual, source-bound, topology-bounded, and unable to masquerade as ordinary hosted evidence.

The manual workflow binds checkout to the dispatched exact SHA, requires the reviewed Rust toolchain and OpenSSL build prerequisites, raises and verifies the file-descriptor soft limit, records `lscpu -p=CPU,NODE,SOCKET`, and rejects a host below the topology floor before it starts the gateway.

For each round, the Rust profile launches the real `cwl-pingora-gateway` binary with the configured proxy worker count, establishes and verifies the health response on every keep-alive connection, proves the connections remain open and response-free immediately before SIGTERM, signals the real process, and records each socket's cleanup latency. A round fails if any parked connection survives the one-second bound or closes with unexpected response bytes/errors. Process exit must also remain bounded.

Scheduler evidence is collected from Linux `/proc/<pid>/task/<tid>/schedstat` and task status counters immediately before notification and again after socket cleanup when the process remains observable. The receipt records aggregate CPU runtime, runqueue wait, timeslices, voluntary context switches, and nonvoluntary context switches per round. Linux documents the three schedstat fields as CPU time, runqueue wait time, and timeslice count. The workflow additionally wraps the profile with `perf stat` for task-clock, context switches, CPU migrations, and the futex syscall tracepoint when the host permits it; lack of `perf` permission is recorded rather than converted into fabricated evidence.

The resulting artifact records exact gateway SHA, exact Pingora package family, configured proxy and metrics worker counts, registered service count, topology, connection/round pressure, p50/p95/p99/max socket-close latency, survivors at the close bound, maximum process-exit time, scheduler deltas, topology rows, and optional perf counters.

## Alternatives rejected

Running the existing four-CPU hosted capacity job at higher virtual-user counts was rejected because concurrency pressure does not create NUMA topology or equivalent scheduler behavior. Inferring worker count from `nproc` was rejected because #73 makes worker topology an explicit Admin Config input. Automatically triggering the self-hosted profile on every pull request was rejected because it would create routine queue pressure and could select an unrepresentative host. Pinning or copying an unreleased supplier branch to recreate the historical shared-`Notify` implementation was rejected because mutable supplier code is not a release authority and would violate the gateway ownership boundary.

## Evidence and promotion

A GREEN artifact from this profile is necessary evidence for the performance/scaling half of #46, but it is not sufficient by itself for protected integration or release. The run must be reviewed against the actual target deployment topology, the historical supplier contention report, exact current source, and the rest of #58's dependency graph. If the target environment is larger or materially different, the profile must be rerun there without reducing pressure.

This ADR does not authorize an independent `APPROVED` review, protected merge, immutable release, canary/shadow traffic, rollback acceptance, cutover, or Nginx/OpenResty removal. Those remain separate governance and migration gates.

## Traceability

- `tests/shutdown_contention_profile.rs` — real-process parked-connection profile and source-bound evidence receipt.
- `tests/numa_shutdown_profile_contract.rs` — fail-closed executable workflow/profile policy.
- `.github/workflows/numa-shutdown-profile.yml` — explicit manual execution lane for a representative self-hosted Linux host.
- ContextualWisdomLab/pingora-gateway#46 — canonical Runtime Isolation contention acceptance.
- ContextualWisdomLab/pingora-gateway#73 — explicit proxy-worker topology prerequisite.
- cloudflare/pingora#844 — historical many-core shutdown-notification contention report.

## References

Cloudflare, Inc. (2026, September 9). *Pingora 0.9.0* [Software release]. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.9.0

Linux kernel developers. (2026). *Scheduler statistics*. The Linux Kernel documentation. https://docs.kernel.org/scheduler/sched-stats.html

util-linux project. (2026). *lscpu(1) — display information about the CPU architecture*. Linux manual page. https://man7.org/linux/man-pages/man1/lscpu.1.html
