# ADR 0011: Require a released Pingora parked-read shutdown repair

- Status: Proposed
- Date: 2026-09-03
- Owners: Runtime Isolation / Operability / Supply Chain

## Problem

The protected public Pingora revision used by the current migration stack, `09696b51bc59315353d96686355861604d0bb48c`, still uses one `tokio::sync::Notify` shared by each `HttpProxy` instance for HTTP/1 request reads parked in `HttpProxy::handle_new_request()`. The public source has two distinct commercial-runtime concerns.

First, shutdown is emitted as a one-shot `notify_waiters()` after setting an atomic shutdown flag, but the parked-read select path does not consult that flag before awaiting `Notified`. Upstream review of public PR #969 transplanted its regression to `09696b51...` and reproduced a lost-wakeup when shutdown occurs after the read is pending but before waiter registration. A separate adversarial jitter harness failed immediately on public main while the PR head passed 300/300 rounds.

Second, upstream issue #844 reports the shared `Notify` waiter list as a severe many-core synchronization bottleneck: 65.66% combined off-CPU futex wait was attributed to waiter registration/drop on a 128-core x86 NUMA host. The issue is labeled `Accepted`; the label description says the change is merged to Cloudflare's internal repository, but protected public main still contains the single-`Notify` implementation and no immutable public release carries the repair.

Public PR #969 proposes a sharded notifier plus lost-wakeup handshake, but it is open and unmerged. Its branch also carries an unrelated `.cargo/audit.toml` delta and review has identified both that conflict and a shard-sizing concern around host `available_parallelism()` versus Pingora's explicitly configured service worker count.

## Constraints

- `pingora-gateway` owns the shared edge Runtime Isolation and operability contract, not Cloudflare Pingora internals.
- Product authentication/business logic, Keyverse identity, Wardnet/EgressWeave verdicts, consumer deployment source, and route semantics must remain outside this repair.
- Existing PR #20 graceful-drain evidence covers already-admitted in-flight requests; parked reads waiting before request admission are a separate state and must not be inferred GREEN from that evidence.
- Release dependencies must be immutable and provenance-bound. An open mutable contributor branch is not an admissible Shared Kernel or release contract.
- Existing `.github#1605`, `pingora-gateway#13`, and supply-chain gates remain authoritative for any supplier revision or backport.
- A performance repair must be evaluated against actual configured service/runtime worker topology and realistic parked keep-alive concurrency; reducing concurrency until contention disappears is not acceptable evidence.

## Options considered

### Pin public PR #969 directly

Rejected. The branch is mutable, open, unmerged, currently unmergeable, carries an unrelated audit-policy delta, and has a review-relevant worker/shard sizing concern. It cannot satisfy immutable release provenance.

### Copy or vendor the proposed `pingora-proxy` shutdown implementation into `pingora-gateway`

Rejected. This duplicates supplier lifecycle authority, creates a long-lived fork of a security/performance-sensitive hot path, and violates the canonical owner / released-contract boundary.

### Keep the current supplier and rely on the external 30-second termination budget

Rejected as closure. The supervisor budget is a final process bound, not proof of prompt graceful termination. It masks the lost-wakeup until forced teardown and does nothing about the shared waiter-list contention on normal request-read registration/drop.

### Shorten grace, disable keep-alive, or reduce worker/connection pressure

Rejected. These changes weaken service semantics or measurement realism and can hide rather than repair the causal supplier defect.

### Require a maintainer-integrated immutable release, with a separately governed backport as the only temporary alternative

Selected. The preferred dependency is a public Pingora repair integrated by the supplier and consumed at an immutable reviewed revision/release. If release timing makes a temporary backport necessary, it must be separately approved, provenance-bound to reviewed source, exclude unrelated contributor-branch deltas, pass the existing dependency/security owner gates, and carry an explicit removal condition once the public supplier release is available.

## Decision

`pingora-gateway#46` remains release-blocking for claims about prompt/scalable parked-read shutdown. The gateway will not pin PR #969, copy its proxy implementation, reinterpret the existing external shutdown budget, or weaken concurrency to claim closure.

A candidate repair becomes eligible only when one of these is true:

1. Cloudflare integrates the shutdown repair into public protected source and an immutable/reviewable dependency identity is available; or
2. CWL explicitly governs a provenance-bound minimal backport of the reviewed supplier delta, reconciled with `.github#1605`, `pingora-gateway#13`, and all normal supply-chain evidence, with a documented removal trigger.

Whichever path is selected must prove both correctness and scaling. The lost-wakeup race is a functional RED/GREEN contract; the shared-`Notify` contention is a performance/operability contract. Neither may be substituted for the other.

## Effects and risks

The decision keeps lifecycle authority with the supplier and prevents a mutable contributor branch from becoming hidden production infrastructure. It also avoids accepting a correctness-only repair that leaves the many-core synchronization bottleneck, or a sharding-only optimization that still permits missed shutdown.

The trade-off is that #46 can block release/cutover credit while public Pingora integration is pending. A governed backport may shorten that delay, but only by accepting explicit temporary supply-chain ownership and removal work; it is not the default path.

## Verification

- RED on the affected exact supplier uses compiled generic and pg-erd composition roots with new and reused keep-alive HTTP/1 connections parked in `read_request()`, jitters SIGTERM around waiter registration, and demonstrates a survivor without shortening the configured production grace;
- a separate high-concurrency characterization keeps realistic configured workers and parked connections and records shutdown wall time, CPU and Linux off-CPU/futex evidence where supported;
- GREEN repeats the race adversarially and requires zero parked-read survivors beyond the bounded shutdown expectation;
- already-admitted in-flight requests must still complete inside the configured graceful-drain contract; `/livez`/`/readyz`, startup and recovery semantics remain valid;
- any notifier sharding is evaluated against actual configured service/runtime worker topology rather than assuming host `available_parallelism()` is equivalent;
- no request path, header, cookie, credential, or customer payload is required for the evidence;
- exact-head fmt, compile, clippy, warning-denied rustdoc, 100% owned-production line/region coverage, applicable load/OCI/security/supply-chain checks, and independent review remain mandatory;
- this ADR stays Proposed until the selected immutable supplier/backport path and exact-head RED→GREEN evidence are terminal. A clean documentation review alone cannot move it to Accepted.

## References

Cloudflare. (n.d.). *Pingora HTTP proxy implementation* [Source code, commit `09696b51bc59315353d96686355861604d0bb48c`]. GitHub.

Masterlvng. (2026, March 23). *`Notify`-based shutdown in `HttpProxy` causes severe lock contention on multi-core / NUMA systems* [GitHub issue #844]. Cloudflare Pingora.

nbarbier-265. (2026, August 20). *Shard the HttpProxy shutdown Notify to cut lock contention* [GitHub pull request #969]. Cloudflare Pingora.

ContextualWisdomLab. (2026, September 3). *runtime: bound HTTP/1 parked-read shutdown wakeup and contention* [GitHub issue #46].