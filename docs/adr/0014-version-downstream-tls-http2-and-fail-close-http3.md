# ADR 0014: Version downstream TLS/HTTP/2 and keep HTTP/3 fail-closed

- Status: Proposed
- Date: 2026-09-11
- Owners: Ingress / TLS / HTTP Policy / Admin Config
- Related: #51, #58, PR #75, PR #76, supplier `cloudflare/pingora#901`, `cloudflare/pingora#936`, `cloudflare/pingora#976`

## Problem

The shared gateway and bounded pg-erd migration profile were characterized with cleartext downstream HTTP/1. Migrating an HTTPS edge therefore requires a new transport contract rather than silently changing version-1 semantics. TLS termination, HTTP/2 admission over TLS, complete HTTP/2-to-HTTP/1 translation parity and HTTP/3/QUIC are separate claims and must remain separately evidenced.

A second problem appears once TLS is admitted: `TlsSettings::intermediate` is a useful supplier construction helper but its protocol-version and cipher defaults are not CWL release authority. Leaving those properties implicit would allow an upstream library/profile change to alter the buyer-visible security posture without a versioned CWL decision.

## Constraints

`pingora-gateway` owns reusable edge transport/runtime authority only. Certificate issuance, renewal, ACME, revocation workflow, private-key backup/custody and identity lifecycle remain outside this repository. Product authentication, authorization and business routing remain in the product domain. Keyverse, Wardnet and EgressWeave keep their existing authorities.

Configuration admission remains transport-neutral. Pingora types appear only in delivery/composition. Certificate and key references are read-only inputs; invalid configuration or unusable material fails before listener activation. Existing version-1 generic and version-1/version-2 pg-erd semantics do not acquire TLS implicitly.

HTTP/2 over TLS uses ALPN `h2`; h2c is not admitted. RFC 7301 requires fatal `no_application_protocol` when an ALPN-bearing client offers no protocol supported by the server. HTTP/3 requires QUIC/UDP and its own operational/security contract; current released Pingora source does not expose a first-class H3 server-session path used by this gateway.

This migration serves existing HTTPS/HTTP/2 consumers rather than defining a new TLS application protocol. TLS 1.2 is therefore retained as an explicit compatibility floor while TLS 1.0/1.1 are rejected. RFC 10015's obsolete TLS 1.2 key exchanges are not admitted: no static RSA, static ECDH or finite-field DH suites. TLS 1.3 is the explicit ceiling.

Two supplier correctness roots remain material for HTTP/2 downstream to HTTP/1 upstream translation: `cloudflare/pingora#901` for Cookie coalescing and the zero-length body termination work represented by open contributor candidates #936/#976. Mutable contributor branches are not dependency authority.

## Alternatives considered

### Keep Nginx/OpenResty in front for TLS/H2

Rejected as the target architecture. It can be a temporary migration state, but retaining a legacy proxy and then claiming Pingora edge parity would hide the responsibility this migration is intended to move and leave two independently configured edge policy surfaces.

### Add TLS fields to existing configuration versions

Rejected. Existing versions have already been characterized as cleartext. Reinterpreting them would create a non-obvious transport-authority change and make rollback/config provenance ambiguous.

### Implement certificate lifecycle in the gateway

Rejected. The gateway needs only read-only materialization of already provisioned identity material. Absorbing issuance, renewal or key custody would duplicate canonical identity/secret ownership and expand blast radius.

### Inherit the complete `TlsSettings::intermediate` security profile

Rejected as release authority. Supplier helper defaults may be a sensible starting point, but CWL needs an auditable protocol/cipher contract that changes only through reviewed source and release provenance. The delivery adapter therefore applies its own bounded profile after materialization.

### Make minimum version and cipher strings arbitrary Admin Config

Rejected for this increment. Free-form cipher policy would create a large, hard-to-test configuration state space and make consumer/security posture dependent on mutable operator text. The current profile is code-owned. A future operator-selectable policy would require a separately versioned domain contract, validation grammar and compatibility evidence.

### Require TLS 1.3 only immediately

Rejected for this migration increment. RFC 9852/BCP 195 requires new protocols using TLS to require TLS 1.3, but this project is moving an existing HTTPS/H2 edge. Existing-consumer compatibility is a material migration constraint. TLS 1.2 remains explicit rather than ambient, while obsolete TLS 1.2 key exchanges are removed in accordance with RFC 10015.

### Use Pingora 0.9.0 `enable_h2()` unchanged

Rejected. At protected supplier source `702f69015e53f7244d6ad2e743de571d859a70a4`, the H2/H1 selector returns `AlpnError::NOACK` when no advertised protocol overlaps. RFC 7301 requires fatal failure in that case. CWL uses the public `SslAcceptorBuilder` callback surface to enforce the narrower contract without copying supplier source.

### Enable h2c or HTTP/3 together with TLS/H2

Rejected. h2c has distinct discovery/trust semantics. HTTP/3 adds QUIC, UDP exposure, QPACK, migration, amplification, congestion/loss and 0-RTT replay considerations and therefore needs separate supplier capability and acceptance evidence.

## Decision

Introduce opt-in versioned downstream TLS configuration and a code-owned security profile:

- generic `GatewayConfig` version 1 remains cleartext; version 2 requires `downstream_tls`;
- bounded `PgErdMigrationConfig` versions 1 and 2 remain cleartext; version 3 retains the version-2 response-body lifetime and additionally requires `downstream_tls`;
- `DownstreamTlsConfig` owns certificate-chain path, private-key path and explicit ALPN policy only;
- filesystem references must be absolute and are materialized once immediately before listener construction;
- the Pingora delivery adapter uses `TlsSettings::intermediate`, explicitly verifies the certificate/private-key pair with `check_private_key()`, then overrides supplier defaults with the CWL security profile;
- TLS minimum is 1.2 and maximum is 1.3;
- TLS 1.2 admits only ephemeral ECDHE + AEAD suites: `ECDHE-ECDSA-AES128-GCM-SHA256`, `ECDHE-RSA-AES128-GCM-SHA256`, `ECDHE-ECDSA-AES256-GCM-SHA384`, `ECDHE-RSA-AES256-GCM-SHA384`, `ECDHE-ECDSA-CHACHA20-POLY1305`, and `ECDHE-RSA-CHACHA20-POLY1305`;
- TLS 1.3 admits `TLS_AES_128_GCM_SHA256`, `TLS_AES_256_GCM_SHA384`, and `TLS_CHACHA20_POLY1305_SHA256`;
- `h2_http1` validates the complete ALPN vector, prefers `h2`, falls back to `http/1.1` only when offered and returns a fatal alert for malformed/no-overlap advertised vectors;
- a client that omits ALPN may retain verified HTTP/1 compatibility; no synthetic protocol is assigned;
- production composition uses `add_tls_with_settings` only when the new versioned contract is active and preserves the existing TCP listener otherwise;
- h2c and HTTP/3 remain absent;
- supplier #901 and release-qualified zero-length-body disposition continue to gate complete mixed-protocol parity.

This is source/runtime capability, not deployment authority. Protected release, consumer pin, shadow/canary, rollback rehearsal, cutover and legacy removal remain separate promotion stages.

## Acceptance and evidence

The increment must preserve strict YAML version admission and fail before listener activation for relative/empty material references, unreadable material and certificate/private-key mismatch. Real-wire tests must use an ephemeral CA, verify service identity, assert negotiated ALPN, exercise an actual H2 request through the production composition boundary, retain deliberate HTTP/1.1 fallback and fail an ALPN-bearing unsupported client.

The security-profile child additionally must force exact TLS 1.2 and TLS 1.3 clients through the compiled generic production root, assert that each negotiated cipher is in the code-owned allowlist, and require a TLS-1.1-only client to fail the handshake. Structural regressions must prevent accidental removal of the explicit version/cipher setters or reintroduction of deprecated TLS 1.2 key exchange.

PR #75 predecessor exact `4b4aa3bf6e2a2ae1b5823fa435d9eba7edb1856b` compiled and ran all tests in CI `34536286899` but failed the Rust 1.98.0 Clippy gate for an elidable lifetime on `select_h2_http1`; ordinary-forward exact `723ec4fbc3f58f42408caca87cc986a94354164b` is the minimal lifetime-only repair and must reacquire exact-head gates.

PR #76 test-only exact `2a0f2c2575ed13225b54a808c0e94e69a561641e` defines the explicit TLS security-profile RED. Exact `7bda5a356eb0aad9912345bf7ad076b3fab807c2` applies the protocol/cipher policy. Exact `d2aabfe6d281312b4fcb7f50002228544d410324` adds real-wire TLS 1.2/TLS 1.3 acceptance and TLS 1.1 rejection. Exact `7ba84028d25e5f511860af4579e46145766aeafd` adopts the parent's lifetime-only Clippy repair. Descendant documentation commits do not transfer predecessor GREEN; every new exact head must reacquire its own checks.

## Risks and follow-up

TLS 1.2 remains a deliberate compatibility exposure. Consumer telemetry and security requirements may justify a later TLS-1.3-only contract, but that must be a versioned decision with concrete compatibility evidence rather than an implicit library upgrade.

The current cipher profile intentionally favors a small interoperable AEAD set. It does not yet prove relative handshake CPU cost, resumed-session behavior or a need for Pingora's downstream TLS offload thread pools. Issue #51 remains authoritative for new-handshake versus reused-connection latency/CPU, representative CPU/NUMA profiling, concurrent H2 streams, reset/cancellation, GOAWAY/drain, decoded-header/body limits, flow-control/backpressure, failure/recovery, forwarding trust and rollback/cutover observability.

Supplier #901 plus a maintainer-integrated/release-qualified zero-length-body repair must close, or deployment must make the affected downgrade path unreachable, before complete H2-downstream/H1-upstream parity is claimed.

HTTP/3 remains unsupported until a maintainer-supported release-qualified server integration exists and a separate decision covers UDP/Kubernetes exposure, QUIC transport, QPACK/header limits, connection/stream flow control, path migration, loss/congestion, amplification defenses, 0-RTT replay, drain and rollback.

## Primary references

Aviram, N. (2026). *Deprecating obsolete key exchange methods in TLS 1.2 and DTLS 1.2* (RFC 10015). Internet Engineering Task Force. https://doi.org/10.17487/RFC10015

Bishop, M. (2022). *HTTP/3* (RFC 9114). Internet Engineering Task Force. https://doi.org/10.17487/RFC9114

Cloudflare, Inc. (2026). *Pingora `TlsSettings` listener implementation* (protected source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs

Friedl, S., Popov, A., Langley, A., & Stephan, E. (2014). *Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension* (RFC 7301). Internet Engineering Task Force. https://doi.org/10.17487/RFC7301

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). Internet Engineering Task Force. https://doi.org/10.17487/RFC9000

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). Internet Engineering Task Force. https://doi.org/10.17487/RFC9846

Saint-Andre, P., & Salz, R. (2023). *Service identity in TLS* (RFC 9525). Internet Engineering Task Force. https://doi.org/10.17487/RFC9525

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). Internet Engineering Task Force. https://doi.org/10.17487/RFC9852

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113
