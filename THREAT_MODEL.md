# Threat Model

## Assets

Network authority, upstream identity, availability, operator configuration integrity, request metadata, release artifacts, and consumer trust in the shared runtime.

## Principal threats and controls

| Threat | Current control | Residual gap |
| --- | --- | --- |
| Request-controlled SSRF | Upstream socket is startup config only; v1 has one admitted upstream | Config write compromise remains privileged |
| TLS MITM/upstream impersonation | Explicit SNI plus certificate/hostname verification | Downstream TLS is out of scope |
| Forwarded-header spoofing | Strip `Forwarded`, `X-Forwarded-*`, `X-Real-IP`; emit only `proto=http` | No trusted-proxy/client-IP feature yet |
| Request/body resource exhaustion | Explicit body limit; Content-Length rejected pre-upstream; streaming bytes counted; `max_in_flight_requests` bounds process-wide admitted proxy work | Smaller configurable header budget plus explicit connection/queue admission budgets remain pending |
| Slow/dead upstream | Explicit connect/total-connect/read/write/idle budgets | Retry policy and consumer-specific streaming semantics require characterization |
| Hop-by-hop/request smuggling ambiguity | Pingora standard policy strips hop-by-hop/connection-nominated headers; pinned line includes 0.8-era smuggling fixes | Exact dependency audit and current advisory revalidation still required |
| Credential leakage in telemetry | CWL access logs are coarse/payload-free; Prometheus labels are low-cardinality; process logger redacts Pingora-family dependency message bodies before formatting even under broad `RUST_LOG`, with a compiled trace-level secret-sentinel regression | Product/consumer loggers stay outside this process boundary; distributed tracing still needs a bounded payload contract |
| Container privilege/persistence | Numeric non-root user; no intended writes; read-only-root-compatible layout; OCI runtime contract exists | Protected-head release artifact evidence is still pending |
| Supply-chain substitution | Exact Pingora Git revision; committed `Cargo.lock`; locked CI/build paths reject resolver drift; implicit package-root `build.rs` discovery is disabled with `[package] build = false` | Current advisory gate, SBOM, provenance, signing and immutable artifact digest remain release requirements |
| Unsafe rollout | Health/readiness contracts and graceful-drain/shutdown tests exist | Published artifact, representative canary/shadow evidence and observed rollback remain pending |

## Abuse cases explicitly out of scope

Open forward proxying, arbitrary user-provided destinations, certificate issuance, consumer authentication/business authorization, and product route decisions are rejected as ownership violations rather than implemented generically.
