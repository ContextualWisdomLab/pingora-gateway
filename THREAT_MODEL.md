# Threat Model

## Assets

Network authority, upstream identity, availability, operator configuration integrity, request metadata, release artifacts, and consumer trust in the shared runtime.

## Principal threats and controls

| Threat | Current control | Residual gap |
| --- | --- | --- |
| Request-controlled SSRF | Generic v1 admits one startup-configured upstream; the bounded pg-erd profile admits only prevalidated `backend` and `frontend` transport identities. Neither accepts a request-controlled destination | Configuration write compromise remains privileged |
| TLS MITM/upstream impersonation | Explicit SNI plus certificate/hostname verification and optional fail-closed custom trust-bundle consumption before listener activation | Downstream TLS is out of scope |
| Forwarded-header spoofing | Generic v1 strips request-controlled forwarding identity; pg-erd strips the characterized `Forwarded`/`X-Forwarded-*`/`X-Real-IP` surface and rebuilds only bounded compatibility fields from accepted transport/request authority | No generic trusted-proxy/client-IP feature; a future TLS listener needs TLS-derived scheme evidence |
| Request-body/concurrency exhaustion | Explicit body and process-wide in-flight limits; Content-Length rejected pre-upstream; streaming bytes counted; saturation fails fast with 503 while health remains observable | Operator-controlled HTTP header budgets, per-route/origin capacity and representative pg-erd saturation measurements remain gaps |
| Slow/dead upstream | Explicit connect/total-connect/read/write/idle budgets and one total upstream attempt | Dedicated pg-erd timeout/reset/partial-response/streaming recovery still requires executable characterization |
| Hop-by-hop/request smuggling ambiguity | Pingora standard policy strips hop-by-hop/connection-nominated headers; pinned line includes the 0.8-era smuggling fixes | Exact dependency audit and current advisory revalidation remain release gates; HTTP/2/3 and WebSocket parity are not inferred |
| Credential/customer-data leakage in telemetry | Shared observability emits only coarse status/outcome/body-byte facts and low-cardinality counters; headers, cookies, tokens, payloads, paths and product identifiers are excluded | Dedicated payload-free runtime assertions and tracing evidence remain gaps |
| Container privilege/persistence | Digest-pinned distroless nonroot image, uid/gid 65532, no intended writes; source-defined OCI acceptance requires read-only root, all capabilities dropped and `no-new-privileges` for both admitted generic and pg-erd image profiles | Current exact-head hosted OCI execution must reach terminal GREEN; routed pg-erd traffic is a separate acceptance lane |
| Supply-chain substitution | Exact Pingora Git revision, committed `Cargo.lock`, `--locked` CI/builds, fail-closed dependency policy, source SBOM, and candidate vulnerability scans for both admitted image profiles | `derivative 2.2.0` disposition, release-bound SBOM/provenance/reproducibility, signing/attestation policy and immutable registry digests remain blockers |
| Unsafe rollout | Draft governance, process health, generic graceful-drain test, explicit immutable-artifact/rollback requirements | Dedicated routed drain, shadow/canary, protected cutover and rehearsed rollback against a released digest remain pending |

## Abuse cases explicitly out of scope

Open forward proxying, arbitrary user-provided destinations, certificate issuance, consumer authentication/business authorization, and product route decisions are rejected as ownership violations rather than implemented generically.
