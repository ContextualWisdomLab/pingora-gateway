# Pingora HTTP/1 Parked-Read Shutdown Traceability

This note is a focused primary-source supplement for `pingora-gateway#46`. It records the supplier-bound Runtime Isolation / graceful-shutdown history, the released repair, and CWL's executable consumer acceptance without copying Pingora implementation source into the gateway. The canonical product/technical status remains `docs/product-technical-gap-baseline.md`; release claims still require the main `docs/doctoring/TRACEABILITY.md` and normal exact-head gates.

## Characterized supplier RED and released repair

The historical consumer RED in PR #70 was compiled against the gateway's admitted supplier revision `09696b51bc59315353d96686355861604d0bb48c`. At that revision, `pingora-proxy/src/lib.rs` stores one `tokio::sync::Notify` and one `Arc<AtomicBool>` on each `HttpProxy`. `handle_new_request()` selects between `downstream_session.read_request()` and a newly created `self.shutdown.notified()` future; cleanup stores `shutdown_flag = true` with `Ordering::Release` and calls one-shot `notify_waiters()`.

That exact old-pin source explains the reproduced RED. `notify_waiters()` wakes `Notified` futures that already exist but stores no permit for a future waiter. A later HTTP/1 keep-alive iteration can therefore create its next `Notified` only after cleanup already notified; because the old `handle_new_request()` does not pair that wait with a shutdown-state handshake, the new read can remain parked. Parked requests sharing one `HttpProxy` also register against the same `Notify` waiter-list mutex, which is a separate contention concern from the correctness race.

Protected public Pingora `main` advanced to exact `702f69015e53f7244d6ad2e743de571d859a70a4`, and `refs/tags/0.9.0` points to that commit. The released `pingora-proxy/src/lib.rs` contains a maintainer-integrated successor to the #844/#969 work:

- `HttpProxy` owns a `ShardedNotify` rather than one shared `Notify`;
- shard construction uses configured `ServerConf.threads`, rounded to a power of two and capped, instead of inferring service topology from host `available_parallelism()`;
- `await_shutdown()` registers/polls a notification together with an Acquire read of `shutdown_flag`, preventing a post-notification park from depending on a second `notify_waiters()` call.

The 0.9.0 changelog records lower shutdown-notification contention and closure of the graceful-shutdown lost-wakeup race. GitHub Release **Pingora 0.9.0** was published at `2026-09-09T23:34:48Z` (2026-09-10 08:34:48 KST), is not a prerelease, has no attached assets, and GitHub reports the Release object as `immutable:false`.

CWL now consumes exact crates.io `pingora = "=0.9.0"` and `pingora-prometheus = "=0.9.0"` rather than the historical mutable Git source. Cargo generated the committed registry lock; the direct package checksums are `fc02712a3847828d6b798ecf31f0ac64e138df26b9513e20d52319cf3cecd11e` for `pingora` and `56fc7764cf4a5ff68aae5e373a4e8e2cbff077dd9dea975cc04ca9aa863abb2c` for `pingora-prometheus`. Mutable tag movement therefore cannot silently alter the admitted dependency.

## Upstream provenance

Cloudflare Pingora issue #844 reported the shared-`Notify` path as the dominant scaling bottleneck on a 128-core NUMA host. Its supplied off-CPU profile attributed 34.88% to `Notified::poll_notified -> Mutex::lock -> do_futex` and 30.78% to `Notified::drop -> Mutex::lock -> do_futex`, 65.66% combined.

Public PR #969 remains historical contributor provenance. It is no longer the causal integration prerequisite because released `0.9.0@702f690...` contains an evolved successor. CWL does not consume #969's unrelated `.cargo/audit.toml` delta, force-update its branch, or treat that mutable PR as dependency authority.

The released successor also repairs the previously identified shard-sizing concern by deriving shard count from configured service threads. Performance credit nevertheless remains separate from correctness: realistic configured-worker concurrency/NUMA traffic must still demonstrate shutdown wall time, CPU behavior and available off-CPU/futex evidence before claiming the contention bottleneck removed for commercial deployment.

## CWL executable acceptance

PR #70 turns the supplier race into compiled consumer traffic without moving product-domain authority into the gateway. The generic v1 and bounded pg-erd fixtures preserve the admitted-work control, while `tests/shutdown_post_notify_barrier.rs` adds an external ordering proof for a post-notification keep-alive iteration.

For each composition root, one subject request is admitted and held at its origin. A separate sentinel first completes gateway-local `/livez` over HTTP/1.1 keep-alive and sends a partial next request. Immediately before SIGTERM, a bounded read must observe only `WouldBlock` or `TimedOut`; EOF, response bytes or another error fails the fixture. After SIGTERM stops public accepts, that same sentinel must receive EOF inside a one-second evidence window that remains strictly before the five-second runtime fallback. Only after that cleanup barrier does the fixture release the admitted `Connection: keep-alive` response. The subject must complete its body and then receive EOF on its newly created next keep-alive read inside the same pre-fallback bound.

The origin release channel has a separate 30-second watchdog only to bound fixture leakage; it is not a second graceful-shutdown clock. HTTP success requires the exact `HTTP/1.1 200` status token followed by a space. Origin/header evidence remains finite at five seconds and 64 KiB.

On the historical `09696b51...` supplier, both generic and pg-erd post-notification cases reproducibly exceeded the one-second close bound while admitted in-flight drain still succeeded. After the ordinary crates.io 0.9.0 manifest/lock transition and the repository-local supply-chain contract repair, exact candidate `4e24feb34835a291120f72b58f0865ad5da786b6` passed the unchanged shutdown target together with the admitted-work drains. Its exact CI also passed formatting, all-target compilation/tests, Clippy, rustdoc, 100% owned-production coverage enforcement, resolved-lock verification, load-contract and rootless OCI runtime checks. The same exact candidate passed the independent Supply Chain workflow and bounded-origin capacity workflow.

That GREEN closes the downstream **shutdown correctness** acceptance for the 0.9.0 consumer candidate. It does not close the separate many-core contention/performance half of #46, make the GitHub Release object immutable, resolve `derivative 2.2.0`, supply monotonic whole-header lifetime or parser-admission controls, fix mixed-protocol Cookie/body-framing roots, or authorize protected merge/cutover by itself.

## RED / GREEN evidence contract

RED remains historical causal evidence tied to the old admitted gateway supplier revision. It preserves the admitted request across SIGTERM, externally proves the incomplete sentinel is open/response-free before signal and closed inside the post-signal bound, then releases the admitted keep-alive response and demonstrates that the old supplier leaves the subject's newly created post-notification read parked beyond the evidence window. RED must not be manufactured by shortening production grace, disabling keep-alive, reducing worker topology or weakening the close oracle.

GREEN requires an ordinary, reviewable dependency/lock update to an admissible exact supplier identity containing the repair and the unchanged generic+pg-erd assertions to pass while already-admitted in-flight drain remains GREEN. That condition is now satisfied on the 0.9.0 consumer candidate described above. For a single-`Notify` implementation, the causal invariant is that a shutdown wait cannot be created after the last one-shot notification without also observing shutdown state. The released `ShardedNotify` successor uses its own synchronization design; downstream acceptance remains behavior-based rather than tied to one implementation shape.

Because the GitHub 0.9.0 Release object is reported as `immutable:false`, its existence alone is not the immutable-release proof required by CWL's later promotion policy. The exact crates.io package identities and Cargo lock bind this candidate's consumed code, while gateway release provenance/signing/reproducibility remain later release gates.

The performance half is not closed by this low-core CI GREEN or by publication of 0.9.0. Repeat scheduling jitter and configured-worker high-concurrency tests on the largest available Linux/NUMA-capable environment, validate actual configured service/runtime worker topology rather than host CPU count, and capture shutdown wall time, CPU behavior and off-CPU/futex evidence where available. Sample reduction, worker reduction or disabling keep-alive merely to hide contention is inadmissible.

## References

Cloudflare. (2026). *Pingora HTTP proxy implementation* [Source code, commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-proxy/src/lib.rs

Cloudflare. (2026, September). *Pingora changelog: 0.9.0* [Source code, commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/CHANGELOG.md

Cloudflare. (2026, September 9). *Pingora 0.9.0* [Software release]. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.9.0

Cloudflare. (2026). *Pingora HTTP proxy implementation before the protected shutdown successor* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/lib.rs

Masterlvng. (2026, March 23). *`Notify`-based shutdown in `HttpProxy` causes severe lock contention on multi-core / NUMA systems* [GitHub issue #844]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/844

nbarbier-265. (2026, August 20). *Shard the HttpProxy shutdown Notify to cut lock contention* [GitHub pull request #969]. Cloudflare Pingora. https://github.com/cloudflare/pingora/pull/969

Tokio Contributors. (n.d.). *Notify* [Rust API documentation]. docs.rs. https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html

Tokio Contributors. (n.d.). *Notified* [Rust API documentation]. docs.rs. https://docs.rs/tokio/latest/tokio/sync/futures/struct.Notified.html

ContextualWisdomLab. (2026, September 3). *runtime: bound HTTP/1 parked-read shutdown wakeup and contention* [GitHub issue #46]. https://github.com/ContextualWisdomLab/pingora-gateway/issues/46
