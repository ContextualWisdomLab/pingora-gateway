# ADR 0011: Require a released Pingora parked-read shutdown repair

- Status: Proposed
- Date: 2026-09-03
- Last revalidated: 2026-09-20
- Owners: Runtime Isolation / Operability / Supply Chain

## Problem

The currently consumed gateway ancestry still pins Cloudflare Pingora `09696b51bc59315353d96686355861604d0bb48c`. At that revision, each `HttpProxy` instance uses one shared `tokio::sync::Notify` for HTTP/1 reads parked in `handle_new_request()`. The old public source has two commercially relevant defects: the waiter-list mutex becomes a many-core contention point, and a one-shot shutdown notification can be missed when shutdown fires after `read_request()` is pending but before the waiter is registered.

Upstream #844 measured 65.66% combined off-CPU futex wait in `Notified` registration/drop on a 128-core NUMA host. Historical review of PR #969 reproduced the lost-wakeup race against the affected public revision.

The supplier state has since changed. `cloudflare/pingora#844` was closed as completed on 2026-09-09. Public PR #969 was closed without a GitHub merge on 2026-09-11 after its fix was accepted into Cloudflare's internal line. Pingora 0.9.0, tag `702f69015e53f7244d6ad2e743de571d859a70a4`, was published on 2026-09-09 and its release notes explicitly record sharded proxy shutdown notifications, reduced lock contention, and closure of the lost-wakeup race. The 0.9.0 source implements `ShardedNotify`, derives shard count from configured `ServerConf::threads`, registers the waiter before consulting `shutdown_flag`, and notifies all shards during cleanup.

Therefore #46 is no longer blocked on existence of a public supplier repair. The current gap is **consumer adoption and representative validation**: the active gateway ancestry still consumes the old 0.8.0 Git revision; #70 owns the exact 0.9.0 consumer transition, #73 owns explicit bounded service-worker topology, and #74 owns representative Linux/NUMA contention evidence. Those lanes are currently Draft/stale in the parent-first release graph and their historical GREEN does not transfer across ancestry movement.

## Constraints

- `pingora-gateway` owns the shared edge Runtime Isolation and operability contract, not Cloudflare Pingora internals.
- Product authentication/business logic, Keyverse identity, Wardnet/EgressWeave verdicts, consumer deployment source, and route semantics remain outside this repair.
- Existing graceful-drain evidence covers already-admitted in-flight requests; parked reads waiting before request admission are a separate state.
- The supplier transition must use a versioned/reviewable dependency identity and normal supply-chain gates. A mutable contributor branch or gateway-local copy of `pingora-proxy` remains inadmissible.
- Correctness and scaling are separate acceptance dimensions. Low-core hosted GREEN cannot stand in for representative configured-worker/NUMA evidence.
- The supplier 0.9.0 transition does not bypass independent blockers such as `derivative 2.2.0 / RUSTSEC-2024-0388`, exact-current CodeQL/security settlement, or protected release provenance.

## Options considered

### Pin historical contributor PR #969

Rejected. The branch is not the consumed supplier authority, is closed without direct GitHub merge, and is unnecessary now that the supplier shipped the behavior in Pingora 0.9.0.

### Copy or vendor the shutdown implementation into `pingora-gateway`

Rejected. This duplicates supplier lifecycle authority and creates a long-lived fork of a security/performance-sensitive hot path.

### Keep the old supplier and rely on the external 30-second termination budget

Rejected as closure. The supervisor budget is a final process bound, not proof of prompt parked-read wakeup or many-core scaling.

### Shorten grace, disable keep-alive, or reduce worker/connection pressure

Rejected. These changes weaken semantics or measurement realism and can hide rather than repair the causal issue.

### Consume the released supplier repair and validate it in the gateway topology

Selected. Pingora 0.9.0 is the supplier repair identity to characterize. The gateway must adopt it through the ordinary release-line ancestry, preserve exact dependency provenance, and then prove the behavior through #70/#73/#74 rather than reimplementing supplier internals.

## Decision

`pingora-gateway#46` remains open, but the reason has changed. Supplier availability is no longer the blocker. Closure now requires:

1. ordinary parent-first reconciliation of the #44/#47 release line with current foundation authority;
2. #70 adoption of exact Pingora 0.9.0 on that ancestry and fresh same-head correctness/supply evidence;
3. #73 explicit configured service-worker topology on the current #70 ancestry;
4. #74 representative Linux/NUMA execution with realistic parked keep-alive pressure, repeated shutdown jitter, zero survivors, complete scheduler evidence, and no pressure reduction merely to obtain GREEN;
5. normal review/governance, protected integration, and the broader release/security gates before any cutover or legacy-removal credit.

Historical supplier RED and historical #70/#73/#74 GREEN are characterization evidence only. They do not transfer after ancestry movement.

## Effects and risks

This preserves supplier ownership while giving CWL a concrete released repair to consume. It also separates three questions that must not be collapsed: whether the supplier fixed the race, whether the gateway actually consumes that release under its own exact contracts, and whether the configured worker topology scales under representative pressure.

The remaining risk is primarily promotion/integration debt rather than absence of a supplier implementation. Pingora 0.9.0 itself is not sufficient gateway release evidence: the current foundation still has independent security and hosted-gate blockers, and #74 still lacks representative self-hosted NUMA evidence on protected ancestry.

## Verification

- historical RED remains tied to the affected exact supplier and is not rerun merely to recreate known failure;
- #70 must prove the released 0.9.0 consumer path with new/reused parked HTTP/1 connections and preservation of admitted in-flight drain;
- #73 must make the proxy worker count explicit and bounded rather than inferring it from host CPU visibility;
- #74 must retain at least the documented representative pressure floor, repeated pre-signal jitter, zero parked-read survivors, bounded shutdown tail/process exit, stable-TID scheduler completeness, and CPU/runqueue/timeslice/context-switch evidence; off-CPU/futex evidence is captured where permitted;
- `/livez`/`/readyz`, startup/recovery and payload-free logging remain intact;
- exact-head fmt, compile, Clippy, warning-denied rustdoc, 100% owned-production line/region coverage, applicable load/OCI/security/supply-chain checks and independent review remain mandatory;
- this ADR remains Proposed until the released supplier transition and representative gateway evidence are integrated into the current protected ancestry.

## References

Cloudflare. (2026, September 9). *Pingora 0.9.0* [Software release, tag `702f69015e53f7244d6ad2e743de571d859a70a4`]. GitHub.

Cloudflare. (2026). *Pingora HTTP proxy implementation* [Source code, Pingora 0.9.0]. GitHub.

Masterlvng. (2026, March 23). *`Notify`-based shutdown in `HttpProxy` causes severe lock contention on multi-core / NUMA systems* [GitHub issue #844]. Cloudflare Pingora.

nbarbier-265. (2026, August 20). *Shard the HttpProxy shutdown Notify to cut lock contention* [GitHub pull request #969]. Cloudflare Pingora.

ContextualWisdomLab. (2026, September 3). *runtime: bound HTTP/1 parked-read shutdown wakeup and contention* [GitHub issue #46].