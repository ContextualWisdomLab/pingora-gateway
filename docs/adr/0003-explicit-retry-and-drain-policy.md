# ADR 0003: Make Retry and Drain Policy Explicit

Status: Accepted

## Context

The pinned Pingora server configuration defaults `max_retries` to 16 and leaves graceful-shutdown timing unset. The proxy implementation copies `ServerConf::max_retries` into the request loop and executes upstream attempts while the local counter is below that value. In that implementation, `1` means one total upstream attempt; `0` means no upstream attempt at all. Pingora's graceful-termination path also falls back to framework timing when the corresponding `ServerConf` fields are unset.

The pinned server lifecycle has another important operational detail: after the configured grace period, each service runtime is shut down with `Runtime::shutdown_timeout(timeout)` and its shutdown worker then sleeps for the same timeout. Service shutdown workers run in parallel, so the modeled worst-case process budget is `grace_period + 2 * graceful_shutdown_timeout`, excluding small scheduler/process-exit overhead.

Those framework defaults are unsafe as implicit CWL edge semantics. Automatic retry can replay non-idempotent requests without product-domain knowledge, while framework-owned drain timing can change independently of the gateway's operability and container-termination contract.

## Decision

Version 1 constructs `ServerConf` through `src/runtime_policy.rs` and sets:

- `max_retries = 1`, meaning one total upstream attempt and zero automatic retries;
- `grace_period_seconds = 5` after SIGTERM;
- `graceful_shutdown_timeout_seconds = 10` for each Pingora runtime-shutdown phase; and
- an external supervisor hard-kill budget of 30 seconds.

The 5 + 2×10 = 25 second modeled Pingora worst case leaves a small margin inside the required 30-second supervisor budget. A deployment with a shorter hard-kill deadline is not admitted by v1. Product-specific retry policy remains outside the generic gateway until a later version can express request eligibility, replay/body buffering, retry budget and observability as an explicit contract.

The constants are covered by an executable policy regression. `tests/graceful_shutdown.rs` additionally drives the compiled binary, holds an upstream response open, signals SIGTERM after the request is in flight, requires the response to complete during the grace period, and requires clean process exit before the external budget.

## Consequences

The initial shared runtime favors non-replay safety over transparent failover. A consumer that depends on retry behavior is not migration-compatible with v1 until that behavior is characterized and intentionally designed. Deployments must provision the documented termination budget, and exact release candidates must pass the process-level drain test; predecessor-head evidence does not transfer.
