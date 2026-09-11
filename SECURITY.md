# Security

## Trust boundaries

Configuration files, downstream requests, forwarding headers, certificate/key filesystem references, upstream certificates and deployment inputs are untrusted. Operators must protect configuration and mounted TLS-material write access. TLS-enabled configuration stores filesystem references, not certificate lifecycle state or credentials. Certificate issuance, renewal, revocation workflow, backup and private-key custody remain outside this repository.

The generic gateway contacts only its startup-admitted upstream socket. The bounded pg-erd migration profile contacts only the explicit prevalidated `backend` and `frontend` authorities admitted by its compiled migration plan. Neither process accepts a request-controlled upstream URI, so the runtime does not become a generic forward-proxy or SSRF primitive.

Upstream TLS uses certificate and hostname verification with explicit SNI. Optional absolute `trust_bundle_file` material is read during peer activation before listeners open. Downstream TLS is opt-in only in generic configuration version 2 and pg-erd version 3. `DownstreamTlsConfig` accepts only absolute certificate-chain/private-key references plus the admitted ALPN policy; `tls_delivery` materializes them once immediately before listener construction and explicitly rejects unusable or mismatched certificate/key material. This is TLS termination and read-only identity-material consumption, not CA/ACME/key-management ownership.

Inbound forwarding identity is deleted before proxying. Request-controlled `Forwarded`, `X-Forwarded-*`, `X-Real-IP` and `X-Forwarded-Server` values are not trusted. Cleartext configurations derive `http`; TLS-enabled configurations derive `https` from accepted listener transport. Neither path turns these fields into user, tenant or authorization identity.

HTTP/1 protocol transition is denied at two gateway-owned boundaries. Request admission rejects an `Upgrade` field or exact comma-delimited `upgrade` connection token before application admission/origin selection, and every immutable Pingora upstream peer uses `HttpUpstreamRequestPolicy::deny_upgrades()`. This is defense in depth and explicit non-support, not WebSocket parity.

Downstream TLS/H2 admits only `h2_http1`: Pingora prefers `h2` and permits HTTP/1.1 fallback. h2c is not enabled. HTTP/3/QUIC remains fail-closed and must not be inferred from TLS support. Full H2-downstream/H1-upstream production parity remains gated by release-qualified supplier disposition of `cloudflare/pingora#901` and `#936`.

Request bodies, upstream I/O and process-wide concurrent application traffic are bounded. `max_in_flight_requests` fails fast with HTTP 503 rather than allowing unbounded queueing. Runtime worker count is explicitly bounded to 1–256 and is not derived from host CPU count.

## Parser and lifetime gaps

Downstream HTTP/1 request headers have finite supplier limits but still lack a CWL-configurable parser-phase admission budget. Callback-only rejection occurs after supplier parsing and therefore cannot be credited as pre-allocation resource admission. HTTP/2 decoded-header-list accounting is a separate protocol model and must not be conflated with HTTP/1 wire/parser bytes. The gateway must not vendor mutable Pingora parser source to hide this gap.

Downstream HTTP/1 header acquisition also has a separate whole-header lifetime gap: per-read inactivity is not a monotonic whole-header deadline. Proxy callbacks execute only after request-header parsing, so a callback watchdog cannot establish the required pre-complete-header lifetime. This root remains separate from parser byte/count admission and from upstream response-header lifetime.

Graceful shutdown is likewise a distinct availability boundary. Pingora 0.9.0 has passed the current stack's exact consumer correctness fixture for parked-read shutdown, while representative configured-worker/NUMA contention remains separately open under issue #46. Low-core hosted evidence cannot be promoted into many-core contention closure, and mutable supplier branches are not supply-chain authority.

Pg-erd version 2/3 adds a response-body progress lifetime. It starts at the first non-informational upstream response header and is evaluated on non-empty body progress. It complements per-read inactivity and is not an exact timer interrupt for a pending read; incomplete response-header slow delivery remains separate.

## TLS identity and protocol verification

Real-wire TLS acceptance must verify an intended service identity against a controlled CA and assert negotiated ALPN. Certificate/key mismatch, invalid material and missing material must fail before listener activation. HTTP/1.1 fallback is deliberate and tested separately from H2. The broader H2 matrix—parallel streams, reset/cancellation, GOAWAY/drain, header/body limits, flow control/backpressure, failure recovery and cutover observability—remains required by issue #51 before production parity can be claimed.

HTTP/3 requires a separate threat model covering UDP exposure, QUIC amplification and path behavior, QPACK/header limits, stream/connection flow control, congestion/loss handling and 0-RTT replay. Until a release-qualified server implementation and that contract exist, H3 remains unsupported.

## Logging and data minimization

The production path emits coarse status/outcome/request-body-byte access logs and bounded Prometheus counters. It does not log Authorization, Proxy-Authorization, Cookie, Set-Cookie, request/response bodies, access tokens, arbitrary headers, route values, trust-bundle contents, certificate private-key material, TLS material contents or configuration credentials. TLS filesystem paths must not become request-derived labels or payload logs.

The guarantee applies to the whole gateway process. Operator-selected `RUST_LOG` verbosity is parsed normally, but records from Pingora-family dependency targets pass through `logging_policy` before formatting so supplier debug/error output cannot bypass the shared data-minimization contract.

## Supply chain and runtime isolation

Pingora and `pingora-prometheus` are exact `0.9.0` registry dependencies under the committed Cargo lock and must be revalidated against current releases/advisories before release. CI uses `--locked` and rejects resolution drift. Mutable supplier PRs are evidence only, not dependencies.

The Docker process selector admits exactly `cwl-pingora-gateway` or `cwl-pingora-pg-erd-migration`. Candidate images run as uid/gid `65532` under a read-only root filesystem, all capabilities dropped and `no-new-privileges`. Exact-head OCI acceptance builds and starts both admitted profiles. TLS-enabled deployment additionally requires certificate/key material to be mounted read-only from the canonical secret/certificate owner; the image must not bake private-key material into layers.

A committed lock and PR-head scan are necessary but not sufficient release evidence. Protected release requires current vulnerability/license policy, registry-bound immutable image digest, SBOM/provenance/reproducibility evidence, signing/attestation policy and rollback rehearsal before a consumer cutover may rely on the runtime.

Report security issues privately through the organization security channel rather than publishing exploit details in a public issue.
