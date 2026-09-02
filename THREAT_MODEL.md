# Threat Model

## Assets

Network authority, upstream identity, availability, operator configuration integrity, request metadata, release artifacts, and consumer trust in the shared runtime.

## Principal threats and controls

| Threat | Current control | Residual gap |
| --- | --- | --- |
| Request-controlled SSRF | Upstream socket authority is startup configuration only; generic v1 admits one fixed upstream and the bounded pg-erd profile admits only characterized `backend`/`frontend` identities | Config write compromise remains privileged; future discovery/dynamic resolution must preserve the same fail-closed authority boundary |
| Recursive self-proxy / metrics-surface routing | Traffic and metrics listeners cannot overlap; admitted upstreams also cannot overlap either gateway-owned listener under exact, wildcard, or conservative IPv6-wildcard/IPv4 dual-stack authority semantics | Exact-head hosted execution of the new source contract is still pending; this control does not replace deployment-network ACLs for metrics |
| TLS MITM/upstream impersonation | Explicit upstream SNI plus certificate/hostname verification; optional operator trust bundle is materialized once before listeners open | Successful pg-erd TLS-listener parity and downstream TLS are not yet proven |
| Forwarded-header spoofing | Generic v1 strips the full request-controlled forwarding identity set and emits only gateway-owned `Forwarded: proto=http`; the bounded migration policy rebuilds characterized compatibility fields from accepted transport/request authority | No trusted-proxy/client-IP chain feature; HTTPS needs a separate TLS-derived scheme contract |
| Request-body / concurrency exhaustion | Explicit declared and streamed body limits; mandatory process-wide in-flight admission fails fast at 503 while health/metrics remain reachable | Configurable header, connection/per-route, larger origin-capacity and broader streaming budgets remain gaps |
| Slow/dead/aborting upstream | Explicit connect/total-connect/read/write/idle budgets plus source acceptance for refused origin, connected-silent read timeout, orderly truncated response, pre-header TCP RST and post-commit TCP RST; no automatic replay/failover is invented | Slow-drip/whole-response lifetime, broader streaming/Upgrade failure and representative origin-capacity evidence remain open |
| Hop-by-hop/request smuggling ambiguity | Pingora standard policy strips hop-by-hop/connection-nominated headers; exact dependency source is pinned and the gateway adds explicit forwarding-identity handling | Supplier advisory disposition and authoritative Dependency Review evidence remain release blockers |
| Credential or product-data leakage in shared telemetry | Shared observability emits bounded status/outcome/body-byte facts and low-cardinality counters; dedicated compiled pg-erd acceptance sends non-vacuous URI/query/Host/Authorization/Cookie/product-context sentinels and requires none in the canonical shared log target | Distributed tracing is not yet implemented; unrelated consumer or third-party logger policy remains outside this bounded context |
| Container privilege/persistence | Digest-pinned distroless image, uid/gid 65532, read-only root, capabilities dropped, `no-new-privileges`, read-only configuration; both composition roots have exact OCI source acceptance | Current exact-head hosted OCI execution and immutable published digest/provenance are still required |
| Supply-chain substitution / known vulnerable graph | `Cargo.lock` committed; `--locked` build/test policy; exact Pingora revision; SBOM/image-vulnerability/provenance gates exist | `derivative 2.2.0` / `RUSTSEC-2024-0388`, exact-release versus patched-`lru` policy, and public non-fork Dependency Review HTTP 403 remain fail-closed release blockers |
| Unsafe rollout | Draft/pre-traffic stack, process health, source acceptance for routed graceful drain, explicit parity -> shadow/canary -> cutover -> rollback sequence | No immutable protected release, consumer shadow/canary, cutover, rehearsed rollback or legacy-removal evidence yet |

## Abuse cases explicitly out of scope

Open forward proxying, arbitrary user-provided destinations, certificate issuance, consumer authentication/business authorization, product route decisions, Wardnet/EgressWeave verdict duplication, and Keyverse identity ownership are rejected as ownership violations rather than implemented generically.
