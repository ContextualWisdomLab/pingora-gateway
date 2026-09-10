# Threat Model

## Assets

Network authority, upstream identity, availability, operator configuration integrity, request metadata, release artifacts, and consumer trust in the shared runtime.

## Principal threats and controls

| Threat | Current control | Residual gap |
| --- | --- | --- |
| Request-controlled SSRF | Upstream socket is startup config only; v1 has one admitted upstream and no per-request destination | Config write compromise remains privileged |
| TLS MITM/upstream impersonation | Explicit SNI plus certificate/hostname verification; optional absolute trust bundle is loaded fail-closed before listeners open | Downstream TLS is out of scope; certificate issuance/rotation/revocation stays with its canonical owner |
| Forwarded-header spoofing | Strip `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto`, `X-Real-IP`; emit only gateway-owned `proto=http` | No trusted-proxy/client-IP feature yet |
| Request/resource exhaustion | Explicit body limit; declared length rejected pre-upstream; streamed bytes counted; `max_in_flight_requests` fails excess application traffic fast with 503; health bypasses the admission budget; keepalive pool is explicitly bounded | Operator-controlled smaller HTTP/1 header byte/count budget and representative consumer queue/origin-capacity study remain open |
| Slow/dead upstream or accidental replay | Explicit connect/total-connect/read/write/idle budgets; one total upstream attempt and zero generic automatic retries | Reset/slow-stream/partial-response behavior and any product retry/failover require separate characterization |
| Hop-by-hop/request-smuggling ambiguity | Pingora standard policy strips hop-by-hop/connection-nominated headers; pinned line includes the 0.8-era smuggling fixes | H2→H1 Cookie reconstruction and zero-length chunked-body supplier repairs are separately gated; exact dependency/advisory revalidation remains required |
| Credential or PII leakage in telemetry | Implemented coarse access logging and low-cardinality request/error/body-byte/backpressure counters omit credentials, cookies, bodies and unbounded request labels | Distributed tracing and representative production observability evidence remain open |
| Container privilege/persistence | Digest-pinned distroless nonroot runtime, uid/gid 65532, no intentional writes, read-only-root/capability-free/`no-new-privileges` executable acceptance | Every moved release candidate must reacquire exact-head OCI and image-scan GREEN |
| Supply-chain substitution or known-unmaintained dependency | Exact Pingora Git revision, committed `Cargo.lock`, locked builds, license/source/advisory policy, exact-source SBOM and final-image scan | `derivative 2.2.0` / RUSTSEC-2024-0388 requires immutable maintainer-integrated supplier repair; protected provenance/reproducibility and published digest are still release gates |
| Compiler miscompilation | Current branch manifest is explicit; separately gated #56 moves release-producing paths to Rust 1.98.1 after the 1.98.0 vtable-generation miscompilation disclosure | #56 exact-head GREEN and ordinary ancestry adoption are required before release; documentation cannot substitute for compiler evidence |
| Unsafe rollout | Health paths, bounded drain policy and executable graceful-shutdown test exist; cutover policy requires immutable artifact identity and rollback | No protected gateway release/digest exists yet, and no consumer shadow/canary/cutover/legacy-removal evidence exists |
| Responsibility/authority collapse | DDD contract keeps product auth/business policy, Keyverse identity, Wardnet/EgressWeave decisions, certificate lifecycle and consumer FastCGI/application semantics outside the shared edge | Consumer migration must prove the responsibility boundary from live deployment evidence before replacing a legacy proxy |

## Abuse cases explicitly out of scope

Open forward proxying, arbitrary user-provided destinations, certificate issuance, consumer authentication/business authorization, product route decisions, cross-service application-table access, and a generic OJS/PHP FastCGI server are rejected as ownership violations rather than implemented in the shared runtime.

## Evidence rule

A control is not promoted because it exists in source or passed on a predecessor head. Release and migration claims require the exact protected candidate to reacquire the applicable compile/test/rustdoc/coverage/load/OCI/security/supply-chain/review evidence, then bind the immutable release artifact to consumer deployment, shadow/canary, rollback and cutover evidence.