# Downstream TLS / HTTP/2 traceability

Last verified: 2026-09-11

This note connects the versioned downstream TLS/H2 increment to supplier source, Internet standards, executable evidence and the remaining promotion boundaries. It is evidence documentation, not a release or cutover claim.

## Authority boundary

`pingora-gateway` owns listener transport, TLS materialization, ALPN admission and reusable edge HTTP behavior. It consumes certificate/private-key references read-only. Certificate issuance, renewal, revocation, ACME, backup and private-key custody remain external. Product authentication/authorization/business routing, Keyverse identity and Wardnet/EgressWeave verdicts are not duplicated here.

Generic configuration version 1 and pg-erd versions 1/2 remain cleartext. Generic version 2 and pg-erd version 3 require downstream TLS. Pg-erd version 3 retains version-2 response-body lifetime semantics. Earlier versions reject the new TLS field.

## Supplier mapping

Current released dependency: `pingora = 0.9.0`, `pingora-prometheus = 0.9.0` from the registry and committed Cargo lock.

Current Cloudflare protected source checked for listener semantics: `702f69015e53f7244d6ad2e743de571d859a70a4`.

At that source:

- `TlsSettings::intermediate(cert_path, key_path)` constructs the server acceptor and loads the PEM private key and certificate chain.
- `TlsSettings::intermediate` does not call `check_private_key()`; the CWL delivery adapter therefore performs that explicit pairing check before listener activation.
- `TlsSettings::enable_h2()` installs `ALPN::H2H1`; its `prefer_h2` callback returns `AlpnError::NOACK` when there is no common advertised protocol.
- Pingora exposes the underlying OpenSSL-compatible `SslAcceptorBuilder` through the public TLS settings API, so CWL can install a stricter ALPN selector without vendoring or copying supplier source.
- `add_tls_with_settings` is used only after the versioned transport-neutral Admin Config has been admitted and materialized.

Supplier source: https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs

## Standards mapping

RFC 9113 defines HTTP/2. For HTTPS, the HTTP/2 protocol identifier is `h2`; h2c is a distinct non-TLS mode and is not admitted by this increment.

RFC 7301 defines ALPN negotiation. It requires the server to choose a common advertised protocol and, when an ALPN-bearing client offers no protocol supported by the server, terminate with fatal `no_application_protocol`. The CWL `h2_http1` selector therefore validates the complete client protocol-list vector before selection, prefers `h2`, falls back to `http/1.1` only when offered, and returns `AlpnError::ALERT_FATAL` for no overlap or malformed vectors. This is intentionally stricter than Pingora 0.9.0's convenience H2/H1 selector.

OpenSSL documents ALPN protocol lists as vectors of non-empty 8-bit length-prefixed byte strings; zero-length and truncated entries are invalid. Its server ALPN callback is not invoked when ClientHello contains no ALPN extension. Consequently an ALPN-bearing no-overlap client and a client that omits ALPN are distinct compatibility cases: the former is rejected, while the latter may complete verified TLS with no negotiated application protocol and continue on Pingora's ordinary HTTP/1 path.

RFC 9846 is the current TLS 1.3 specification and obsoletes RFC 8446. RFC 9525 supplies current service-identity verification guidance for TLS applications. The real-wire tests use a controlled CA and a certificate for `gateway.test` to exercise identity validation rather than disabling peer verification.

RFC 9000 and RFC 9114 show why HTTP/3 is not a TLS-listener checkbox: HTTP/3 is mapped over QUIC and therefore introduces a UDP/QUIC transport surface. H3 remains fail-closed until a release-qualified server capability and a separate operational/security contract exist.

## RED / repair chain

PR #75 was created from exact #73 base `625cae4f156366bc39d6782161a4a5f336d58624`. Its original RED established that the production roots had no versioned downstream TLS authority. The implementation then introduced transport-neutral downstream TLS configuration, generic v2 / pg-erd v3 admission, the Pingora delivery adapter, TLS listener composition, and real-wire H2/H1 fallback/lifecycle tests. Subsequent ordinary-forward repairs resolved stale fixture, coverage, formatting, Clippy and child-process lifecycle findings without weakening gates.

Exact predecessor `a7f12c8ee67bfe06a802aab66f7f038ec11bfb6f` completed CI `34530278038`, Supply Chain `34530277984` and PgErd bounded-origin capacity `34530278009` successfully. That remains historical exact evidence only.

A later standards review found a distinct protocol defect before release: Pingora 0.9.0's H2/H1 convenience selector returns `NOACK` on ALPN no-overlap, whereas RFC 7301 requires fatal `no_application_protocol` when the client supplied ALPN but no protocol overlaps. Test-only exact `5aebba832692debdf99a6866b22be25deea9cdb2` added a real-wire unsupported-ALPN handshake oracle. Its hosted CI was cancelled by the subsequent ordinary-forward repair before the test job executed, so that SHA is a realistic RED definition, not a terminal hosted RED receipt.

Ordinary-forward source exact `2d97b944e855cc04346b4fbd85838c5abb4ce506` replaced `enable_h2()` with a small CWL-owned selector installed through Pingora/OpenSSL's public callback surface. It preserves H2 preference, permits H1 only when offered and returns fatal on no overlap.

Fresh review of the selector then found that an early return on a valid `h2` prefix could accept a syntactically malformed trailing protocol-list entry, contradicting the documented malformed-vector fail-closed contract. Exact `70bc745e31ae52d3e1c681cccba208f9c8a9e690` minimally repairs this by scanning and validating the complete vector before choosing H2 over H1. Unit regressions now cover malformed suffixes after both a valid `h2` and a valid `http/1.1` prefix.

Real-wire compatibility review also found that source-level OpenSSL semantics alone did not prove the legacy TLS client case most relevant to Nginx/OpenResty parity. Exact `d751be56fb043b95084f3e810435ca76de05dfc6` adds a certificate-verified client that deliberately sends no ALPN extension, asserts that no synthetic protocol is negotiated, sends a real HTTP/1.1 request through the generic production root and requires a successful origin round trip. Exact `9d00693d4142035b5559851c6c7688a5ac2bcf3b` updates the test strategy to make both complete-vector validation and no-ALPN compatibility explicit. Later exact heads must run these unchanged contracts GREEN; no predecessor result transfers.

## What this increment can prove

On an unchanged exact head with terminal checks, the increment can prove explicit versioned TLS listener authority, read-only certificate/key materialization, certificate/key mismatch failure before listener activation, verified TLS service identity, negotiated H2, deliberate H1 fallback with explicit HTTP/1.1 ALPN, verified HTTP/1.1 compatibility when ALPN is omitted, fatal unsupported-ALPN behavior, actual H2 request/response flow through the generic production root, bounded pg-erd TLS activation and child-process cleanup.

## What remains RED / not release-qualified

This increment must not be promoted to complete H2 production parity by inference. Issue #51 still requires realistic acceptance for concurrent streams, reset/cancellation, GOAWAY/graceful drain, header/body admission, flow control/backpressure, origin failure/recovery, readiness, forwarding trust, new-handshake versus reused-connection timing and rollback/cutover observability.

Two supplier roots remain especially important for H2 downstream to H1 upstream translation: `cloudflare/pingora#901` for Cookie coalescing and `cloudflare/pingora#936` for zero-length DATA/body termination. Open contributor heads are not released dependency authority. The gateway must consume a maintainer-integrated/release-qualified supplier identity or enforce a versioned deployment protocol path that makes the affected downgrade unreachable before claiming parity.

HTTP/3/QUIC remains unsupported. No H3 credit is earned from TLS, ALPN or H2 evidence.

## Promotion boundary

After exact-head source/test/documentation checks and independent review, this branch may participate in ordinary non-force stack integration. Release still requires the foundation/compiler and supplier roots identified by canonical issue #58, protected integration, version/CHANGELOG/tag/package, immutable artifact digest, SBOM/provenance/reproducibility, rollback and concrete consumer parity. Shadow/canary, observed rollback, cutover and verified legacy Nginx/OpenResty removal remain deployment evidence, not source evidence.

## Primary references (APA 7th)

Bishop, M. (2022). *HTTP/3* (RFC 9114). Internet Engineering Task Force. https://doi.org/10.17487/RFC9114

Cloudflare, Inc. (2026). *Pingora TLS listener implementation* (source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs

Friedl, S., Popov, A., Langley, A., & Stephan, E. (2014). *Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension* (RFC 7301). Internet Engineering Task Force. https://doi.org/10.17487/RFC7301

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). Internet Engineering Task Force. https://doi.org/10.17487/RFC9000

OpenSSL Project Authors. (2024). *SSL_CTX_set_alpn_select_cb* (OpenSSL 3.1 documentation). https://docs.openssl.org/3.1/man3/SSL_CTX_set_alpn_select_cb/

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). Internet Engineering Task Force. https://doi.org/10.17487/RFC9846

Saint-Andre, P., & Salz, R. (2023). *Service identity in TLS* (RFC 9525). Internet Engineering Task Force. https://doi.org/10.17487/RFC9525

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113
