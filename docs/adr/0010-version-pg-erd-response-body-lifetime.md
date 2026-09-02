# ADR 0010: Version the pg-erd upstream response-body lifetime

- Status: Proposed
- Date: 2026-09-03
- Owners: Runtime Isolation / bounded pg-erd migration

## Problem

The characterized pg-erd candidate configures Pingora `read_timeout`, but at the pinned Pingora revision `09696b51bc59315353d96686355861604d0bb48c` that setting applies to each individual upstream read and resets after a successful read. A backend can therefore retain an admitted request indefinitely by continuing to send response-body progress before each inactivity timeout. This can consume the process-local in-flight budget even though no single read stalls.

The existing version-1 pg-erd Admin Config has no whole-response field. Reinterpreting `read_ms`, deriving a hidden multiplier, or reusing `total_connection_ms` would change an existing contract without an explicit version boundary; `total_connection_ms` also belongs to connection establishment, including TLS, rather than response delivery.

## Constraints

- Generic gateway v1 must not silently gain product-specific long-response semantics.
- The pg-erd migration may add only edge/runtime authority. Product retry, idempotency, authentication, business routing, Wardnet/EgressWeave verdicts, and Keyverse identity remain outside the gateway.
- Existing pg-erd version-1 fixtures and predecessor PRs must remain readable while the stack is unreleased.
- A failure after the downstream response header is committed cannot be converted into a second HTTP status or silently failed over.
- The pinned Pingora callback API exposes response-header and response-body progress callbacks but does not expose an exact absolute-deadline interrupt for a currently pending upstream read.

## Options considered

### Keep only `read_ms`

Rejected. It protects inactivity, not total response-body lifetime, so continuous slow-drip remains unbounded.

### Reinterpret `read_ms` or derive a multiplier

Rejected. This creates undocumented semantics, couples two distinct failure modes, and makes operator intent impossible to audit.

### Reuse `total_connection_ms`

Rejected. Pingora applies that budget to connection establishment; changing its meaning at the gateway layer would conflict with supplier semantics.

### Add an explicit pg-erd version-2 body-lifetime budget

Selected. Version 2 requires positive `max_upstream_response_body_ms`. Version 1 rejects that field and otherwise retains its existing behavior. Runtime Isolation owns monotonic elapsed-time accounting, while the migration adapter starts the budget on the first non-informational upstream response header and checks it on each upstream response-body callback.

## Decision

Introduce pg-erd Admin Config version 2 with mandatory positive `max_upstream_response_body_ms`. Preserve version 1 without a hidden lifetime. On a version-2 request, start the response-body budget at the first non-informational upstream response header. If a later body-progress callback arrives at or beyond the budget, raise an upstream-scoped fatal error. Preserve the existing post-commit invariant: terminate the incomplete downstream response, record low-cardinality request-error telemetry, release the request admission lease with its context, and do not invent retry or failover.

This is a progress-driven bound, not an exact timer interrupt. A continuously progressing body is stopped at the first body callback at or after the configured lifetime. A quiescent upstream is still bounded by its independent per-read `read_ms`. Slow-drip of an incomplete response header remains a separate gap because the current callback guard has not yet established an absolute header-read deadline.

## Effects and risks

The selected design makes slow-drip body retention explicit and auditable and keeps it in the Runtime Isolation bounded context. It avoids changing generic v1 and preserves the pg-erd v1 characterization stack. Operators must choose the version-2 value from observed long-response requirements before canary or cutover; example values are not production SLOs.

The remaining limitation is timer precision: scheduler delay and Pingora read cadence can move actual termination past the configured instant until the next body callback. A future supplier/runtime capability may justify an exact timer boundary, but this ADR does not claim one.

## Verification

- deterministic Runtime Isolation tests cover absent, dormant, active, and expired response-body budgets;
- versioned Admin Config tests cover v1 compatibility, v1 rejection of v2 fields, v2 explicit positive values, zero rejection, and incomplete v2 schema rejection;
- real-listener pg-erd traffic commits HTTP 200 and drips one body byte every 60 ms while `read_ms` is 500 ms and `max_upstream_response_body_ms` is 300 ms; the response must terminate before the declared 20-byte body completes, without a second status or route failover, while error telemetry, readiness, and an independent route recover;
- exact-head format, compile, clippy, rustdoc, 100% owned-production coverage, load, OCI, security, supply-chain, and independent-review evidence remain required before this ADR may move from Proposed to Accepted.

## References

Cloudflare. (2026). *Pingora peer configuration and timeout semantics* (revision `09696b51bc59315353d96686355861604d0bb48c`). GitHub.

Cloudflare. (2026). *ProxyHttp response filtering callbacks* (revision `09696b51bc59315353d96686355861604d0bb48c`). GitHub.
