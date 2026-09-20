# Pingora HTTP/1 Parked-Read Shutdown Traceability

This focused primary-source note supports `pingora-gateway#46`. It keeps supplier/runtime evidence separate from product authentication/business logic, Keyverse identity, Wardnet/EgressWeave authority, and consumer deployment ownership. Repository-wide product/technical status and general TRACEABILITY remain dedicated #61 authority.

## Consumed supplier state

The current #44/#47 release-line parent still consumes Cloudflare Pingora 0.8.0 from exact Git revision `09696b51bc59315353d96686355861604d0bb48c`. At that revision, each `HttpProxy` instance has one shared `tokio::sync::Notify` for HTTP/1 reads parked in `handle_new_request()` plus an atomic shutdown flag. The parked-read path can miss a one-shot notification if shutdown occurs after `read_request()` is pending but before its `Notified` waiter registers. The same shared waiter list is also a synchronization hotspot under many-core keep-alive pressure.

These defects are distinct from already-admitted request drain. Existing gateway graceful-drain acceptance proves a held admitted request can finish inside the configured grace; it does not by itself prove prompt wakeup or scalable registration/unregistration for connections waiting for a new HTTP/1 request.

## Historical supplier evidence

Cloudflare issue #844 reported 34.88% off-CPU time in `Notified::poll_notified -> Mutex::lock -> do_futex` and 30.78% in `Notified::drop -> Mutex::lock -> do_futex`, 65.66% combined, on a 128-core NUMA host. Historical PR #969 proposed sharded shutdown notification plus a shutdown-flag handshake and included separate parked-wakeup and lost-wakeup regressions.

Independent review of the affected public revision reproduced the lost-wakeup condition and also showed why correctness and contention must be assessed separately. That evidence remains useful characterization of the old supplier; it is not a reason to keep treating the public repair as unavailable.

## Released supplier state — revalidated 2026-09-20

The supplier state changed materially in September 2026:

- `cloudflare/pingora#844` is closed as completed as of 2026-09-09 and carries the upstream `Accepted` disposition;
- public PR #969 was closed without a GitHub merge on 2026-09-11 after the repair was accepted through Cloudflare's supplier process;
- Pingora 0.9.0 was published on 2026-09-09 at tag exact `702f69015e53f7244d6ad2e743de571d859a70a4`;
- the 0.9.0 release notes explicitly record sharded proxy shutdown notifications to reduce lock contention and closure of the graceful-shutdown lost-wakeup race;
- 0.9.0 source uses `ShardedNotify`, sizes it from configured `ServerConf::threads`, registers the shutdown waiter before consulting `shutdown_flag`, and notifies all shards during cleanup.

That is sufficient to change the owner-path conclusion: the missing item is no longer a public supplier implementation. The live CWL release line still pins the affected 0.8.0 revision and must adopt and validate the released behavior normally.

## CWL adoption and evidence chain

The current owner chain is parent-first:

1. #44/#47 establish the current release-line documentation and dependency boundary without copying supplier source.
2. #70 owns the released-supplier consumer transition to exact crates.io `pingora = "=0.9.0"` and `pingora-prometheus = "=0.9.0"`. Its live PR is Draft because its ancestry is stale against current foundation/release-line work; historical exact GREEN is characterization only.
3. #73 owns explicit bounded `service_threads`, propagating configured worker count into Pingora rather than inferring data-plane topology from host CPU visibility. It is Draft pending current #70 ancestry.
4. #74 owns the representative Linux/NUMA parked-read contention harness. Its historical branch-local CI/Supply/capacity gates were GREEN, but no representative self-hosted NUMA artifact has been produced and the PR is Draft pending #73/#70 parent movement.

No predecessor receipt transfers after these branches are reconciled. No contributor fork or mutable PR is a dependency authority.

## RED / GREEN contract

Historical RED remains tied to the affected 0.8.0 exact supplier and need not be recreated by deliberately weakening or reverting current work. Current GREEN requires the released supplier to be consumed on the current protected ancestry and then validated through the gateway's actual topology:

- #70: repeated new/reused parked HTTP/1 shutdown correctness with zero lost-wakeup survivors while preserving already-admitted in-flight drain, process exit, readiness, recovery, and payload-safe diagnostics;
- #73: explicit configured proxy-worker topology with bounded/fail-closed Admin Config and separate metrics-worker behavior;
- #74: representative Linux/NUMA execution with realistic configured workers, at least the documented parked keep-alive pressure and repeated shutdown jitter, zero correctness survivors, bounded shutdown tail/process exit, complete stable-TID scheduler evidence, and CPU/runqueue/timeslice/context-switch evidence; off-CPU/futex evidence is retained where the environment permits it.

Low-core loopback results cannot close the many-core contention claim. Reducing workers, parked connections, keep-alive use, sample count, or evidence requirements merely to obtain GREEN is inadmissible.

## References

Cloudflare. (2026, September 9). *Pingora 0.9.0* [Software release, tag `702f69015e53f7244d6ad2e743de571d859a70a4`]. GitHub.

Cloudflare. (2026). *Pingora HTTP proxy implementation* [Source code, Pingora 0.9.0]. GitHub.

Masterlvng. (2026, March 23). *`Notify`-based shutdown in `HttpProxy` causes severe lock contention on multi-core / NUMA systems* [GitHub issue #844]. Cloudflare Pingora.

nbarbier-265. (2026, August 20). *Shard the HttpProxy shutdown Notify to cut lock contention* [GitHub pull request #969]. Cloudflare Pingora.

ContextualWisdomLab. (2026, September 3). *runtime: bound HTTP/1 parked-read shutdown wakeup and contention* [GitHub issue #46].