# Threat Model

## Assets

Network authority, downstream and upstream service identity, private-key confidentiality at the mount boundary, availability, operator configuration integrity, request metadata, release artifacts and consumer trust in the shared runtime.

## Principal threats and controls

| Threat | Current control | Residual gap |
| --- | --- | --- |
| Request-controlled SSRF | Generic runtime admits one startup-configured upstream; bounded pg-erd admits only characterized `backend`/`frontend` identities. No request-controlled destination or discovery. | Configuration write compromise remains privileged; any future discovery must preserve fail-closed authority. |
| Recursive self-proxy / metrics routing | Traffic and metrics listeners cannot overlap; admitted upstreams cannot alias either gateway-owned listener across exact/wildcard/mapped authority forms. | Deployment network ACLs remain separate; every exact head must rerun the contract. |
| Downstream TLS material substitution | TLS exists only in generic v2 and pg-erd v3; certificate/key paths must be absolute, are materialized immediately before listener construction and certificate/key mismatch fails activation. | Mount/config write authority is deployment-owned. Certificate issuance, renewal, revocation, backup and key custody are intentionally external. |
| TLS service-identity / ALPN confusion | Real-wire tests verify a controlled CA and intended DNS identity; `h2_http1` is the only admitted ALPN policy and maps to Pingora H2-preferred/H1-allowed behavior. | Production certificate/SNI inventory and concrete consumer cutover evidence remain deployment obligations. |
| Accidental h2c or HTTP/3 exposure | No h2c listener path is admitted. HTTP/3/QUIC is explicitly unsupported by current config and composition. | A future H3 increment requires separate UDP/QUIC/QPACK/0-RTT threat analysis and release-qualified supplier support. |
| H2→H1 translation ambiguity | Current branch proves basic real-wire H2 traffic and deliberate H1 fallback. | Supplier `cloudflare/pingora#901` Cookie coalescing and `#936` zero-length body-framing remain release blockers for complete mixed-protocol parity. |
| Forwarded-header spoofing | Request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP` and `X-Forwarded-Server` are discarded; compatibility fields are rebuilt from accepted transport/request authority. | Trusted-proxy/client-IP chain policy is not yet a generic feature and must be separately versioned if required. |
| Uncharacterized HTTP protocol transition | Request admission rejects HTTP/1 Upgrade before origin selection; every Pingora peer also denies upgrades. | WebSocket and HTTP/2 Extended CONNECT remain unsupported until separately versioned and exercised. |
| Body / header / concurrency exhaustion | Explicit request-body and process in-flight budgets; `service_threads` bounded to 1–256; health remains available under application saturation. | HTTP/1 parser-phase byte/count admission and whole-header lifetime remain supplier-owned pre-callback gaps. H2 header-list/flow-control accounting requires its own acceptance. |
| Parked-read shutdown race / many-core contention | Pingora 0.9.0 passed exact consumer shutdown correctness; worker topology is explicit and bounded; a representative NUMA harness exists on sibling #74. | Representative many-core/NUMA profile is still required by #46; low-core hosted evidence is not contention closure. |
| Slow/dead/aborting upstream | Explicit connect/total-connect/read/write/idle budgets; pg-erd v2/v3 adds monotonic response-body progress lifetime; deterministic failure traffic exists. | Progress lifetime cannot interrupt a pending read at an exact wall-clock instant; incomplete upstream response-header slow delivery remains separate. |
| Credential or product-data leakage | Low-cardinality payload-free shared telemetry; Pingora-family dependency logs pass through process logging policy. TLS/key contents and paths are not request labels. | Rich tracing remains release work and must preserve the same minimization boundary. |
| Container privilege / key persistence | Distroless non-root runtime, read-only root filesystem, capabilities dropped, `no-new-privileges`; TLS material is deployment-mounted read-only rather than baked into image layers. | Immutable registry digest, attestation and deployment mount policy remain release/deployment evidence. |
| Supply-chain substitution | Exact Pingora 0.9.0 registry dependency, committed lock, `--locked` CI/build, dual-image SBOM/scan/source binding. Mutable supplier PRs are not dependencies. | Current advisory roots and protected-release provenance/reproducibility/signing must be closed before release. |
| Unsafe rollout | Source capability is kept distinct from release and traffic authority; canonical order is protected integration → immutable artifact → parity → shadow/canary → observed rollback → cutover → legacy removal. | No source PR may claim deployment completion. |

## H2 operational residuals

Basic H2 request/response success is not enough to close issue #51. Production parity still requires concurrent streams, reset/cancellation, GOAWAY/graceful drain, header/body admission, flow control/backpressure, origin failure/recovery, readiness and forwarding trust, plus separate new-handshake and reused-connection performance evidence. The downgrade defects tracked by supplier #901/#936 remain explicit until release-qualified.

## HTTP/3 residuals

HTTP/3 runs over QUIC and introduces a different transport/security surface. Before admission, require explicit UDP/Kubernetes/network-policy exposure, QUIC TLS/ALPN, QPACK/header limits, stream and connection flow control, path migration, loss/congestion behavior, amplification defenses, 0-RTT replay policy, drain/connection close, observability and rollback. TLS/H2 enablement is not partial H3 evidence.

## Abuse cases outside gateway ownership

Open forward proxying, arbitrary user destinations, certificate issuance, consumer authentication/business authorization, product route decisions, Wardnet/EgressWeave verdict duplication and Keyverse identity ownership are rejected as boundary violations rather than implemented generically. WebSocket, h2c and HTTP/3 are not inherently abuse cases; they are simply outside the currently admitted protocol contract.
