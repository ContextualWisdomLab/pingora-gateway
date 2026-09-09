# Pingora HTTP/1 Parked-Read Shutdown Traceability

This note is a focused primary-source supplement for `pingora-gateway#46`. It records a supplier-bound Runtime Isolation / graceful-shutdown gap without copying Pingora implementation source into the gateway. The canonical product/technical status remains `docs/product-technical-gap-baseline.md`; release claims still require the main `docs/doctoring/TRACEABILITY.md` and all normal exact-head gates.

## Exact supplier state

Protected Cloudflare Pingora `main` was revalidated at `09696b51bc59315353d96686355861604d0bb48c` on 2026-09-09. At that exact revision, `pingora-proxy/src/lib.rs` stores one `tokio::sync::Notify` and one `Arc<AtomicBool>` on each `HttpProxy` instance. `handle_new_request()` uses a biased `tokio::select!` between `downstream_session.read_request()` and a newly created `self.shutdown.notified()` future. `http_cleanup()` stores `shutdown_flag = true` with `Ordering::Release` and then invokes `notify_waiters()`.

That public source has two independently relevant consequences:

- `Notify::notify_waiters()` wakes `Notified` futures that already exist but deliberately stores no permit for a future waiter. Tokio documents that a `Notified` created before `notify_waiters()` receives that notification even if it has not yet been polled. The real one-shot gap is therefore a later HTTP/1 keep-alive iteration that creates its next `Notified` only after `http_cleanup()` has already called `notify_waiters()`: public `handle_new_request()` does not consult `shutdown_flag`, so that post-notification park can wait for a notification that will never be repeated;
- parked requests sharing an `HttpProxy` register and unregister against that instance's `Notify` waiter-list mutex, creating an instance-local synchronization point on a hot HTTP/1 path.

This is not the same contract as an admitted request already executing upstream work. Existing gateway drain tests prove that admitted in-flight requests can finish inside the configured grace period. Issue #46 owns the distinct state in which a downstream connection is parked waiting for a request or a subsequent keep-alive request.

## Upstream evidence

Cloudflare Pingora issue #844 reports the shared-`Notify` path as the dominant scaling bottleneck on a 128-core NUMA host. Its supplied off-CPU profile attributes 34.88% to `Notified::poll_notified -> Mutex::lock -> do_futex` and 30.78% to `Notified::drop -> Mutex::lock -> do_futex`, 65.66% combined. The issue remains open but labeled `Accepted`, whose upstream description states the change is accepted and merged to Cloudflare's internal repository. That label is not public-source or release evidence for CWL; protected public `main@09696b51...` still contains the single-`Notify`-per-`HttpProxy` implementation.

Public PR #969 remains open and proposes a sharded notification path plus an explicit shutdown-flag handshake. Its two regression tests have different purposes: `shutdown_wakes_parked_read_requests` protects the requirement that waiters created before cleanup are promptly interrupted; `shutdown_before_read_request_parks_returns_immediately` protects the post-notification park where no `notify_waiters()` permit exists for a future `Notified`. The proposed helper creates/enables its `Notified` before an Acquire load of `shutdown_flag`; either the already-set flag returns immediately or a later cleanup can notify the already-created waiter.

An independent review on #969 transplanted only those tests onto public `main@09696b51...`. The ordinary parked-read wake test passed, while `shutdown_before_read_request_parks_returns_immediately` failed with a five-second timeout. The reviewer also ran an independent jitter harness for 300 rounds × 8 tasks: public main failed in round 0 with a parked read that never woke, while the PR head passed 300/300 rounds. This makes the correctness gap executable rather than a source-inspection hypothesis.

The contributor PR is not an admissible dependency. Its current patch also changes `.cargo/audit.toml`; the independent review found that delta redundant and weaker than current upstream wording, and identified it as the PR's merge conflict. The same review also noted that sizing notification shards from `available_parallelism()` can diverge from Pingora's explicitly configured service worker count. CWL therefore waits for a maintainer-integrated immutable/released repair or a separately governed provenance-bound backport; it does not pin the contributor branch or vendor `pingora-proxy` source.

## CWL executable acceptance

PR #70 turns the correctness gap into compiled consumer traffic without changing production gateway Rust or the supplier pin. The generic v1 and bounded pg-erd fixtures preserve the existing admitted-work control, while `tests/shutdown_post_notify_barrier.rs` adds the causal ordering proof that listener closure plus a fixed sleep alone cannot provide.

For each composition root, one subject request is admitted and held at its origin. A separate sentinel connection first completes gateway-local `/livez` over HTTP/1.1 keep-alive, sends a partial next request, and is given scheduler time to enter the next request read **before** SIGTERM. After SIGTERM stops public accepts, the sentinel must receive EOF inside a one-second window that remains strictly before the five-second runtime fallback. That EOF is the external barrier proving cleanup has already interrupted a waiter that existed before shutdown. Only after that barrier does the fixture release the held `Connection: keep-alive` response on the subject connection. The subject must complete its admitted body and then receive EOF on its newly created next keep-alive read inside the same pre-fallback bound. Pinned public Pingora is expected to fail that final post-notification close because `notify_waiters()` stores no permit and `handle_new_request()` does not consult `shutdown_flag` before parking.

The sentinel changes false-GREEN direction intentionally. If a loaded runner delays the sentinel so it was not actually parked before cleanup, the sentinel close assertion fails and the fixture produces no supplier credit; it cannot accidentally let the subject start before the one-shot notification and pass. Origin/header evidence remains finite at five seconds and 64 KiB, and HTTP success requires the exact `HTTP/1.1 200 ` status token so numeric-prefix lookalikes cannot manufacture GREEN.

## RED / GREEN evidence contract

RED must exercise the exact compiled generic and bounded pg-erd composition roots. Preserve an admitted request across SIGTERM, externally prove that cleanup has already closed a preexisting sentinel waiter, then release the admitted keep-alive response and demonstrate that the affected supplier leaves the subject's newly created post-notification read parked until the bounded evidence window expires. Separately run repeated scheduling jitter around shutdown and characterize high-concurrency registration/unregistration at realistic configured worker counts, capturing shutdown wall time, CPU utilization, and Linux off-CPU/futex evidence where the environment permits it. Do not manufacture RED by shrinking the production grace period, disabling keep-alive, reducing worker topology, or weakening the close oracle.

GREEN requires a supported supplier capability on an immutable/released or explicitly governed backport identity. Re-run the deterministic sentinel-barrier assertion, repeated jitter and high concurrency; require zero parked-read survivors beyond the bounded shutdown expectation, retain successful completion of already-admitted in-flight requests inside the configured grace, and preserve `/livez`/`/readyz` behavior. If the supplier repair shards waiters, verify the shard count against actual configured worker/runtime topology rather than assuming host `available_parallelism()` is equivalent. No request path, header, cookie, credential, or customer payload is needed in this evidence.

The performance part of #46 is not closed by a low-core CI pass. The largest available Linux/NUMA-capable performance environment must be used before claiming removal of the supplier contention bottleneck. Sample reduction, worker reduction, or turning off keep-alive merely to make contention disappear is not acceptable.

## References

Cloudflare. (n.d.). *Pingora HTTP proxy implementation* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/lib.rs

Masterlvng. (2026, March 23). *`Notify`-based shutdown in `HttpProxy` causes severe lock contention on multi-core / NUMA systems* [GitHub issue #844]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/844

nbarbier-265. (2026, August 20). *Shard the HttpProxy shutdown Notify to cut lock contention* [GitHub pull request #969]. Cloudflare Pingora. https://github.com/cloudflare/pingora/pull/969

Tokio Contributors. (n.d.). *Notify* [Rust API documentation]. docs.rs. https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html

Tokio Contributors. (n.d.). *Notified* [Rust API documentation]. docs.rs. https://docs.rs/tokio/latest/tokio/sync/futures/struct.Notified.html

ContextualWisdomLab. (2026, September 3). *runtime: bound HTTP/1 parked-read shutdown wakeup and contention* [GitHub issue #46]. https://github.com/ContextualWisdomLab/pingora-gateway/issues/46
