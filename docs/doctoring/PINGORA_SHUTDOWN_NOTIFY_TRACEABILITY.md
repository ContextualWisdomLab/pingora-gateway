# Pingora HTTP/1 Parked-Read Shutdown Traceability

This note is a focused primary-source supplement for `pingora-gateway#46`. It records a supplier-bound Runtime Isolation / graceful-shutdown gap without copying Pingora implementation source into the gateway. The canonical product/technical status remains `docs/product-technical-gap-baseline.md`; release claims still require the main `docs/doctoring/TRACEABILITY.md` and all normal exact-head gates.

## Characterized supplier RED versus current protected supplier

The executable consumer RED in PR #70 is intentionally still compiled against the gateway's admitted supplier revision `09696b51bc59315353d96686355861604d0bb48c`. At that revision, `pingora-proxy/src/lib.rs` stores one `tokio::sync::Notify` and one `Arc<AtomicBool>` on each `HttpProxy`. `handle_new_request()` selects between `downstream_session.read_request()` and a newly created `self.shutdown.notified()` future; cleanup stores `shutdown_flag = true` with `Ordering::Release` and calls one-shot `notify_waiters()`.

That exact old-pin source explains the consumer RED. `notify_waiters()` wakes `Notified` futures that already exist but stores no permit for a future waiter. A later HTTP/1 keep-alive iteration can therefore create its next `Notified` only after cleanup already notified; because the old `handle_new_request()` does not pair that wait with a shutdown-state handshake, the new read can remain parked. Parked requests sharing one `HttpProxy` also register against the same `Notify` waiter-list mutex, which is a separate contention concern from the correctness race.

Protected public Pingora `main` has since advanced. On 2026-09-10 the protected ref is exact `702f69015e53f7244d6ad2e743de571d859a70a4`, and `refs/tags/0.9.0` points to that same commit. Current `pingora-proxy/src/lib.rs` contains a maintainer-integrated successor to the #844/#969 work:

- `HttpProxy` owns a `ShardedNotify` rather than one shared `Notify`;
- shard construction uses configured `ServerConf.threads`, rounded to a power of two and capped, instead of inferring service topology from host `available_parallelism()`;
- `await_shutdown()` registers/polls a notification together with an Acquire read of `shutdown_flag`, preventing a post-notification park from depending on a second `notify_waiters()` call.

The current 0.9.0 changelog explicitly records lower shutdown-notification contention and closure of the graceful-shutdown lost-wakeup race. Exact `702f690...` upstream build, Semgrep and CodeQL runs are terminal success. This is protected-source and release-preparation evidence, not yet an admissible CWL dependency: GitHub has no Release object for tag `0.9.0` at this observation point. The gateway therefore does not pin the tag or mutable branch merely to turn a test GREEN.

## Upstream provenance

Cloudflare Pingora issue #844 reported the shared-`Notify` path as the dominant scaling bottleneck on a 128-core NUMA host. Its supplied off-CPU profile attributed 34.88% to `Notified::poll_notified -> Mutex::lock -> do_futex` and 30.78% to `Notified::drop -> Mutex::lock -> do_futex`, 65.66% combined.

Public PR #969 remains open/non-mergeable on historical contributor lineage. It is useful provenance for the public repair but is no longer the causal integration prerequisite because protected `main@702f690...` already contains an evolved successor. CWL does not consume #969's unrelated `.cargo/audit.toml` delta, force-update its branch, or treat the open PR itself as release authority.

The protected successor also repairs the previously identified shard-sizing concern by deriving shard count from configured service threads. Performance credit nevertheless remains separate from correctness: realistic configured-worker concurrency/NUMA traffic must still demonstrate shutdown wall time, CPU behavior and available off-CPU/futex evidence before claiming the contention bottleneck removed for commercial deployment.

## CWL executable acceptance

PR #70 turns the old-pin correctness gap into compiled consumer traffic without changing production gateway Rust or the supplier pin. The generic v1 and bounded pg-erd fixtures preserve the admitted-work control, while `tests/shutdown_post_notify_barrier.rs` adds an external ordering proof for a post-notification keep-alive iteration.

For each composition root, one subject request is admitted and held at its origin. A separate sentinel first completes gateway-local `/livez` over HTTP/1.1 keep-alive and sends a partial next request. Immediately before SIGTERM, a bounded read must observe only `WouldBlock` or `TimedOut`; EOF, response bytes or another error fails the fixture. After SIGTERM stops public accepts, that same sentinel must receive EOF inside a one-second evidence window that remains strictly before the five-second runtime fallback. Only after that cleanup barrier does the fixture release the admitted `Connection: keep-alive` response. The subject must complete its body and then receive EOF on its newly created next keep-alive read inside the same pre-fallback bound.

The origin release channel has a separate 30-second watchdog only to bound fixture leakage; it is not a second graceful-shutdown clock. HTTP success requires the exact `HTTP/1.1 200` status token followed by a space. Origin/header evidence remains finite at five seconds and 64 KiB.

## RED / GREEN evidence contract

RED is owned by the exact admitted gateway supplier revision, not by whichever upstream `main` happens to exist later. Preserve the admitted request across SIGTERM, externally prove the incomplete sentinel is open/response-free before signal and closed inside the post-signal bound, then release the admitted keep-alive response and demonstrate that the old supplier leaves the subject's newly created post-notification read parked beyond the evidence window. Do not manufacture RED by shortening production grace, disabling keep-alive, reducing worker topology or weakening the close oracle.

GREEN may be credited only after a supported supplier repair exists on a release-qualified identity (or a separately governed provenance-bound backport), the gateway performs an ordinary pin/lock bump, and the **unchanged** generic+pg-erd assertions pass while already-admitted in-flight drain remains GREEN. For a single-`Notify` implementation, the causal invariant is that a shutdown wait cannot be created after the last one-shot notification without also observing shutdown state. The current protected `ShardedNotify` successor uses its own stronger synchronization design; downstream acceptance is behavior-based rather than tied to one implementation shape.

After the supplier bump, repeat scheduling jitter and configured-worker high-concurrency tests. If the implementation is sharded, validate actual configured worker/runtime topology rather than host CPU count. No request path, header, cookie, credential or customer payload is required in this evidence.

The performance half is not closed by a low-core CI pass or by the existence of tag `0.9.0`. Use the largest available Linux/NUMA-capable performance environment before claiming removal of the supplier contention bottleneck. Sample reduction, worker reduction or disabling keep-alive merely to hide contention is inadmissible.

## References

Cloudflare. (2026). *Pingora HTTP proxy implementation* [Source code, commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-proxy/src/lib.rs

Cloudflare. (2026, September). *Pingora changelog: 0.9.0* [Source code, commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/CHANGELOG.md

Cloudflare. (2026). *Pingora HTTP proxy implementation before the protected shutdown successor* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/lib.rs

Masterlvng. (2026, March 23). *`Notify`-based shutdown in `HttpProxy` causes severe lock contention on multi-core / NUMA systems* [GitHub issue #844]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/844

nbarbier-265. (2026, August 20). *Shard the HttpProxy shutdown Notify to cut lock contention* [GitHub pull request #969]. Cloudflare Pingora. https://github.com/cloudflare/pingora/pull/969

Tokio Contributors. (n.d.). *Notify* [Rust API documentation]. docs.rs. https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html

Tokio Contributors. (n.d.). *Notified* [Rust API documentation]. docs.rs. https://docs.rs/tokio/latest/tokio/sync/futures/struct.Notified.html

ContextualWisdomLab. (2026, September 3). *runtime: bound HTTP/1 parked-read shutdown wakeup and contention* [GitHub issue #46]. https://github.com/ContextualWisdomLab/pingora-gateway/issues/46
