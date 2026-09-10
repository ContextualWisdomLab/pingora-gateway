# ADR 0014: Version downstream TLS/HTTP/2 and keep HTTP/3 fail-closed

- Status: Proposed
- Date: 2026-09-11
- Owners: Ingress / TLS / HTTP Policy / Admin Config
- Related: #51, #58, PR #75, supplier `cloudflare/pingora#901`, `cloudflare/pingora#936`

## Problem

The shared gateway and the bounded pg-erd migration profile were characterized with cleartext downstream HTTP/1. Migrating an HTTPS edge therefore requires a new transport contract rather than silently changing version-1 semantics. The gateway also needs to distinguish three different claims: TLS termination, HTTP/2 admission over TLS, and complete HTTP/2-to-HTTP/1 translation parity. HTTP/3/QUIC is a fourth protocol surface and must not be inferred from TLS support.

## Constraints

`pingora-gateway` owns reusable edge transport/runtime authority only. Certificate issuance, renewal, ACME, revocation workflow, private-key backup/custody and identity lifecycle remain outside this repository. Product authentication, authorization and business routing remain in the product domain. Keyverse, Wardnet and EgressWeave keep their existing authorities.

Configuration admission must remain transport-neutral. Pingora types may appear only in delivery/composition. Certificate and key references are read-only inputs; invalid configuration or unusable material must fail before listener activation. Existing version-1 generic and version-1/version-2 pg-erd configuration semantics must not acquire TLS implicitly.

HTTP/2 over TLS uses ALPN `h2`; h2c is a distinct cleartext mode and is not admitted by this increment. RFC 7301 requires a fatal `no_application_protocol` outcome when a client sends ALPN but offers no protocol supported by the server. HTTP/3 requires QUIC/UDP and its own operational/security contract; current released Pingora source does not expose a first-class HTTP/3 server-session path used by this gateway.

Two supplier correctness roots remain material for HTTP/2 downstream to HTTP/1 upstream translation: `cloudflare/pingora#901` for Cookie coalescing and `cloudflare/pingora#936` for zero-length DATA/body-termination behavior. Mutable contributor branches are not dependency authority.

## Alternatives considered

### Keep Nginx/OpenResty in front for TLS/H2

Rejected as the target architecture. It can be a temporary migration state, but retaining a legacy proxy and then claiming Pingora edge parity would hide the responsibility that this migration is intended to move. It also leaves two independently configured edge policy surfaces.

### Add TLS fields to existing configuration versions

Rejected. Existing versions have already been characterized as cleartext. Reinterpreting them would create a non-obvious transport-authority change and make rollback/config provenance ambiguous.

### Implement certificate lifecycle in the gateway

Rejected. The gateway needs only read-only materialization of already provisioned identity material. Absorbing issuance, renewal or key custody would duplicate canonical identity/secret ownership and expand the blast radius.

### Use Pingora 0.9.0 `enable_h2()` unchanged

Rejected for the versioned CWL protocol contract. At protected supplier source `702f69015e53f7244d6ad2e743de571d859a70a4`, `enable_h2()` installs the `H2H1` selector, whose no-overlap branch returns `AlpnError::NOACK`. That allows a TLS handshake to continue without selecting an application protocol when the client supplied only unsupported ALPN identifiers. RFC 7301 instead requires a fatal `no_application_protocol` alert when there is no common advertised protocol. CWL therefore uses Pingora's public `SslAcceptorBuilder` callback surface to enforce the narrower contract without copying supplier source.

### Enable h2c or HTTP/3 together with TLS/H2

Rejected. Neither is necessary for the current HTTPS migration increment. h2c has different discovery and trust semantics. HTTP/3 adds QUIC, UDP exposure, QPACK, migration, amplification, congestion/loss and 0-RTT replay considerations that require separate supplier capability and acceptance evidence.

## Decision

Introduce opt-in versioned downstream TLS configuration:

- generic `GatewayConfig` version 1 remains cleartext; version 2 requires `downstream_tls`;
- bounded `PgErdMigrationConfig` versions 1 and 2 remain cleartext; version 3 retains the version-2 response-body lifetime and additionally requires `downstream_tls`;
- `DownstreamTlsConfig` owns only certificate-chain path, private-key path and an explicit ALPN policy;
- filesystem references must be absolute and are materialized once immediately before listener construction;
- the Pingora delivery adapter uses `TlsSettings::intermediate` and explicitly verifies the certificate/private-key pairing with `check_private_key()`;
- `h2_http1` uses Pingora/OpenSSL's public ALPN selection callback surface to prefer `h2`, fall back to `http/1.1` only when actually offered, reject malformed protocol vectors and return a fatal alert when there is no admitted protocol overlap;
- production composition uses `add_tls_with_settings` only when the new versioned contract is active, otherwise it preserves the existing TCP listener;
- h2c and HTTP/3 are absent;
- supplier #901 and #936 continue to gate a claim of complete mixed-protocol H2-downstream/H1-upstream parity.

This is a source/runtime capability decision, not deployment authority. A protected release, consumer pin, shadow/canary, rollback rehearsal, cutover and legacy removal remain separate promotion stages.

## Acceptance and evidence

The increment must preserve strict YAML version admission and fail before listener activation for relative/empty material references, unreadable material and certificate/private-key mismatch. Real-wire tests must use an ephemeral test CA, verify the intended service identity, assert negotiated ALPN, exercise an actual H2 request through the relevant production composition boundary, retain deliberate HTTP/1.1 fallback, and require the TLS handshake to fail when an ALPN-bearing client offers only unsupported protocols. Child gateway processes must be bounded and reaped on every panic/timeout path.

PR #75 predecessor exact `a7f12c8ee67bfe06a802aab66f7f038ec11bfb6f` completed CI run `34530278038`, Supply Chain `34530277984`, and PgErd bounded-origin capacity `34530278009` successfully before the stricter ALPN finding. A later test-only exact `5aebba832692debdf99a6866b22be25deea9cdb2` added the unsupported-ALPN wire oracle; its CI was cancelled by an ordinary-forward head update before hosted execution could establish RED. The finding itself is independently grounded in RFC 7301 plus the exact supplier `NOACK` branch. Ordinary-forward `2d97b944e855cc04346b4fbd85838c5abb4ce506` adds the causal adapter repair and unit coverage. Every descendant must acquire its own exact-head GREEN; predecessor receipts are not transferable.

At Cloudflare Pingora protected source `702f69015e53f7244d6ad2e743de571d859a70a4`, `TlsSettings::intermediate` loads the private key and certificate chain and does not perform `check_private_key()` itself. Its `enable_h2()` helper maps to `ALPN::H2H1`; the CWL adapter deliberately does not use that helper because its no-overlap `NOACK` behavior is weaker than the versioned RFC 7301 fail-closed contract.

## Risks and follow-up

The current increment proves TLS materialization, service identity/ALPN and basic real-wire H2/H1 fallback only after the unchanged current head passes its checks. It does not by itself close the broader HTTP/2 operational matrix. Issue #51 remains authoritative for concurrent streams, reset/cancellation, GOAWAY/drain, header/body limits, flow control/backpressure, origin failure/recovery and cutover observability. Supplier #901/#936 must become maintainer-integrated and release-qualified, or the deployment must enforce an alternate protocol path that makes those downgrade defects unreachable before mixed-protocol parity can be claimed.

HTTP/3 remains explicitly unsupported until a maintainer-supported release-qualified server integration exists and a separate decision covers UDP/Kubernetes exposure, QUIC transport, QPACK/header limits, connection and stream flow control, path migration, loss/congestion, amplification defenses, 0-RTT replay policy, drain and rollback.

## Primary references

Bishop, M. (2022). *HTTP/3* (RFC 9114). Internet Engineering Task Force. https://doi.org/10.17487/RFC9114

Friedl, S., Popov, A., Langley, A., & Stephan, E. (2014). *Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension* (RFC 7301). Internet Engineering Task Force. https://doi.org/10.17487/RFC7301

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). Internet Engineering Task Force. https://doi.org/10.17487/RFC9000

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). Internet Engineering Task Force. https://doi.org/10.17487/RFC9846

Saint-Andre, P., & Salz, R. (2023). *Service identity in TLS* (RFC 9525). Internet Engineering Task Force. https://doi.org/10.17487/RFC9525

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113

Cloudflare, Inc. (2026). *Pingora `TlsSettings` listener implementation*, protected source `702f69015e53f7244d6ad2e743de571d859a70a4`. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs
