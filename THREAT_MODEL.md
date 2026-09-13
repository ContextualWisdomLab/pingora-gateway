# Threat Model

## Assets

Network authority, upstream identity, availability, operator configuration integrity, request metadata, release artifacts, and consumer trust in the shared runtime.

## Principal threats and controls

| Threat | Current control | Residual gap |
| --- | --- | --- |
| Request-controlled SSRF | Upstream socket is startup config only; v1 has one admitted upstream | Config write compromise remains privileged |
| TLS MITM/upstream impersonation | Explicit SNI plus certificate/hostname verification | Downstream TLS is out of scope |
| Forwarded-header spoofing | Strip `Forwarded`, `X-Forwarded-*`, `X-Real-IP`; emit only `proto=http` | No trusted-proxy/client-IP feature yet |
| Request-body exhaustion | Explicit body limit; Content-Length rejected pre-upstream; streaming bytes counted | Header-specific configurable limit and broader concurrency budgets pending |
| Slow/dead upstream | Explicit connect/total-connect/read/write/idle budgets | Retry policy and consumer-specific streaming semantics require characterization |
| Hop-by-hop/request smuggling ambiguity | Pingora standard policy strips hop-by-hop/connection-nominated headers; pinned line includes 0.8-era smuggling fixes | Exact dependency audit and current advisory revalidation still required |
| Credential leakage in telemetry | Policy forbids sensitive headers/bodies | Operational logging/metrics implementation still absent |
| Container privilege/persistence | Numeric non-root user; no intended writes; read-only-root-compatible layout | Hosted OCI test not yet GREEN |
| Supply-chain substitution | Exact Pingora Git revision | No committed lock, SBOM, provenance, signing, immutable digest |
| Unsafe rollout | Draft PR, health paths, documented rollback | Graceful-drain/rolling rollback test and published artifact pending |

## Abuse cases explicitly out of scope

Open forward proxying, arbitrary user-provided destinations, certificate issuance, consumer authentication/business authorization, and product route decisions are rejected as ownership violations rather than implemented generically.
