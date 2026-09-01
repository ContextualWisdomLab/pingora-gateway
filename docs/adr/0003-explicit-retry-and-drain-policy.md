# ADR 0003: Make Retry and Drain Policy Explicit

Status: Accepted

## Context

The pinned Pingora server configuration defaults `max_retries` to 16 and leaves graceful-shutdown timing unset. The proxy implementation copies `ServerConf::max_retries` into the request loop and executes upstream attempts while the local counter is below that value. In that implementation, `1` means one total upstream attempt; `0` means no upstream attempt at all. Pingora's graceful-termination path also falls back to framework timing when the corresponding `ServerConf` fields are unset.

Those framework defaults are unsafe as implicit CWL edge semantics. Automatic retry can replay non-idempotent requests without product-domain knowledge, while framework-owned drain timing can change independently of the gateway's operability contract.

## Decision

Version 1 constructs `ServerConf` through `src/runtime_policy.rs` and sets:

- `max_retries = 1`, meaning one total upstream attempt and zero automatic retries;
- `grace_period_seconds = 5` after SIGTERM; and
- `graceful_shutdown_timeout_seconds = 30` for the final runtime shutdown phase.

Product-specific retry policy remains outside the generic gateway until a later version can express request eligibility, replay/body buffering, retry budget and observability as an explicit contract. These constants are covered by an executable regression so a Pingora dependency update cannot silently restore framework defaults.

## Consequences

The initial shared runtime favors non-replay safety over transparent failover. A consumer that depends on retry behavior is not migration-compatible with v1 until that behavior is characterized and intentionally designed. The bounded shutdown configuration is deterministic, but release readiness still requires a real in-flight SIGTERM test proving listener stop, request completion and timeout behavior on the compiled binary.
