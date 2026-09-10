# Downstream TLS / HTTP/2 traceability

Last verified: 2026-09-11

This note connects the versioned downstream TLS/H2 increment to supplier source, current Internet standards, executable evidence and the remaining promotion boundaries. It is evidence documentation, not a release or cutover claim.

## Authority boundary

`pingora-gateway` owns listener transport, TLS materialization, TLS protocol/cipher admission, ALPN admission and reusable edge HTTP behavior. It consumes certificate/private-key references read-only. Certificate issuance, renewal, revocation, ACME, backup and private-key custody remain external. Product authentication/authorization/business routing, Keyverse identity and Wardnet/EgressWeave verdicts are not duplicated here.

Generic configuration version 1 and pg-erd versions 1/2 remain cleartext. Generic version 2 and pg-erd version 3 require downstream TLS. Pg-erd version 3 retains version-2 response-body lifetime semantics. Earlier versions reject the new TLS field.

The TLS security profile is code-owned by the versioned edge contract rather than exposed as an arbitrary operator cipher string. Changing its version floor/ceiling or cipher sets is therefore a reviewed release decision.

## Supplier mapping

Current released dependency: `pingora = 0.9.0`, `pingora-prometheus = 0.9.0` from the registry and committed Cargo lock.

Current Cloudflare protected source checked for listener semantics: `702f69015e53f7244d6ad2e743de571d859a70a4`.

At that source:

- `TlsSettings::intermediate(cert_path, key_path)` constructs the server acceptor from the backend `mozilla_intermediate_v5` helper and loads PEM private key/certificate material. CWL deliberately applies its own explicit version/cipher profile afterward rather than treating that helper as policy authority.
- `TlsSettings` exposes the underlying OpenSSL-compatible `SslAcceptorBuilder`, including minimum/maximum protocol-version and cipher configuration APIs.
- `TlsSettings::intermediate` does not call `check_private_key()`; the CWL delivery adapter performs that pairing check before listener activation.
- `TlsSettings::enable_h2()` installs `ALPN::H2H1`; its no-overlap path returns `AlpnError::NOACK`. CWL therefore installs its stricter RFC 7301 selector through the public builder callback instead of copying supplier source.
- Pingora exposes optional downstream TLS handshake-offload pools through server configuration. No offload knob is added here; #51 keeps that as an evidence-driven performance decision.
- `add_tls_with_settings` is used only after versioned transport-neutral Admin Config admission and TLS materialization.

Supplier TLS source: https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs

## Standards mapping

RFC 9113 defines HTTP/2. For HTTPS, the HTTP/2 protocol identifier is `h2`; h2c is a distinct cleartext mode and is not admitted by this increment.

RFC 7301 defines ALPN. An ALPN-bearing ClientHello with no common protocol must fail with fatal `no_application_protocol`. The CWL selector validates the complete length-prefixed vector, prefers `h2`, falls back to `http/1.1` only when actually offered, and returns `AlpnError::ALERT_FATAL` for no overlap or malformed vectors. A client that omits ALPN is a different compatibility case: verified TLS may complete with no selected application protocol and continue on Pingora's HTTP/1 path.

RFC 9846, published July 2026, is the current TLS 1.3 specification and obsoletes RFC 8446. Its mandatory-to-implement TLS 1.3 suite is `TLS_AES_128_GCM_SHA256`; `TLS_AES_256_GCM_SHA384` and `TLS_CHACHA20_POLY1305_SHA256` are also recommended. The CWL TLS 1.3 profile admits exactly those three AEAD suites.

RFC 9852 / BCP 195 requires new protocols that use TLS to require TLS 1.3. This migration deploys existing HTTPS/HTTP/2 rather than defining a new application protocol, so that rule does not by itself justify silently breaking existing TLS 1.2 consumers. The current compatibility profile therefore sets TLS 1.2 as an explicit minimum and TLS 1.3 as the explicit maximum; TLS 1.0 and TLS 1.1 are below the admitted floor.

RFC 10015, published July 2026, updates TLS 1.2 guidance: servers must not select non-ephemeral finite-field DH, FFDHE or static RSA cipher suites and should not select static ECDH. The CWL TLS 1.2 profile therefore admits only ECDHE with AES-GCM or ChaCha20-Poly1305:

- `ECDHE-ECDSA-AES128-GCM-SHA256`
- `ECDHE-RSA-AES128-GCM-SHA256`
- `ECDHE-ECDSA-AES256-GCM-SHA384`
- `ECDHE-RSA-AES256-GCM-SHA384`
- `ECDHE-ECDSA-CHACHA20-POLY1305`
- `ECDHE-RSA-CHACHA20-POLY1305`

RFC 9525 remains the service-identity reference. Real-wire tests use a controlled CA and a certificate for `gateway.test`; peer verification is not disabled.

RFC 9000 and RFC 9114 show why HTTP/3 is not a TLS-listener checkbox. HTTP/3 is mapped over QUIC/UDP, so H3 remains fail-closed until a release-qualified server capability and separate operational/security contract exist.

## RED / repair chain

PR #75 was created from exact #73 base `625cae4f156366bc39d6782161a4a5f336d58624`. Its original RED established that the production roots had no versioned downstream TLS authority. Subsequent ordinary-forward work introduced generic v2 / pg-erd v3 admission, transport-neutral TLS config, Pingora delivery, strict ALPN, production listener composition, certificate lifecycle failure handling and real-wire H2/H1 tests.

Exact predecessor `a7f12c8ee67bfe06a802aab66f7f038ec11bfb6f` completed CI `34530278038`, Supply Chain `34530277984`, and PgErd bounded-origin capacity `34530278009` successfully. That evidence is historical only.

A later RFC 7301 review found Pingora 0.9.0's H2/H1 convenience selector returns `NOACK` on ALPN no-overlap. Test-only exact `5aebba832692debdf99a6866b22be25deea9cdb2` defined the unsupported-ALPN wire RED. Ordinary-forward `2d97b944e855cc04346b4fbd85838c5abb4ce506` installed a CWL-owned selector. Exact `70bc745e31ae52d3e1c681cccba208f9c8a9e690` then repaired malformed suffix handling by validating the complete ALPN vector before selection. Exact `d751be56fb043b95084f3e810435ca76de05dfc6` added certificate-verified no-ALPN HTTP/1 compatibility traffic.

Exact #75 `4b4aa3bf6e2a2ae1b5823fa435d9eba7edb1856b` compiled and ran all tests successfully in CI `34536286899`, but Rust 1.98.0 Clippy failed `select_h2_http1` for needless explicit lifetimes. Ordinary-forward #75 exact `723ec4fbc3f58f42408caca87cc986a94354164b` applies only the compiler-suggested lifetime elision. No protocol behavior or authority is changed by that repair; it must reacquire exact-head gates.

A separate pre-cutover security review found that #75 still inherited protocol-version and cipher policy from `TlsSettings::intermediate`. PR #76 is the writer-safe child for that gap. Test-only exact `2a0f2c2575ed13225b54a808c0e94e69a561641e` defines the missing explicit-policy contract. Ordinary-forward `7bda5a356eb0aad9912345bf7ad076b3fab807c2` sets TLS 1.2–1.3 bounds and the code-owned TLS 1.2/TLS 1.3 cipher sets. Exact `d2aabfe6d281312b4fcb7f50002228544d410324` adds real-wire exact-version acceptance/rejection: TLS 1.2 and TLS 1.3 must complete verified production-root round trips with an admitted cipher, while a TLS-1.1-only client must fail the handshake. Exact `7ba84028d25e5f511860af4579e46145766aeafd` adopts the parent Clippy lifetime repair without weakening the new security profile.

Every descendant must acquire its own exact-head GREEN. A predecessor receipt is never transferred across a commit.

## What the current branch can prove after exact-head GREEN

The combined #75/#76 stack can prove:

- opt-in versioned TLS listener authority while legacy config versions remain cleartext;
- read-only certificate/key materialization and certificate/private-key mismatch failure before listener activation;
- an explicit TLS 1.2 minimum and TLS 1.3 maximum rather than ambient supplier defaults;
- TLS 1.2 ephemeral-ECDHE + AEAD cipher admission and the three selected TLS 1.3 AEAD suites;
- verified TLS service identity;
- negotiated H2, deliberate HTTP/1.1 ALPN fallback and verified no-ALPN HTTP/1 compatibility;
- fatal unsupported/malformed ALPN behavior;
- actual production-root TLS 1.2 and TLS 1.3 round trips plus TLS 1.1 rejection;
- actual H2 request/response flow through the generic production root;
- bounded pg-erd TLS activation and child-process cleanup.

This still does not prove complete H2 production parity, representative TLS handshake performance, or a need for dedicated TLS offload.

## What remains RED / not release-qualified

Issue #51 still requires realistic acceptance for concurrent H2 streams, reset/cancellation, GOAWAY/graceful drain, decoded-header/body limits, connection/stream flow control and backpressure, origin failure/recovery, readiness, forwarding trust, new-handshake versus reused-connection timing, rollback/cutover observability and representative CPU/NUMA profiling for the optional Pingora TLS offload path.

Supplier `cloudflare/pingora#901` remains the Cookie-coalescing gate for H2 downstream to H1 upstream translation. Zero-length body termination has open mutable repair candidates #936/#976; neither open contributor head is released dependency authority. The gateway must consume a maintainer-integrated/release-qualified supplier identity or enforce a versioned deployment protocol path that makes affected downgrade behavior unreachable before claiming complete mixed-protocol parity.

HTTP/3/QUIC remains unsupported. No H3 credit is earned from TLS or H2 evidence.

## Promotion boundary

After exact-head source/test/documentation checks and independent review, the stack may participate in ordinary non-force integration. Release still requires the foundation/compiler and supplier roots identified by canonical issue #58, protected integration, version/CHANGELOG/tag/package, immutable artifact digest, SBOM/provenance/reproducibility, rollback and concrete consumer parity. Shadow/canary, observed rollback, cutover and verified legacy Nginx/OpenResty removal remain deployment evidence, not source evidence.

## Primary references (APA 7th)

Aviram, N. (2026). *Deprecating obsolete key exchange methods in TLS 1.2 and DTLS 1.2* (RFC 10015). Internet Engineering Task Force. https://doi.org/10.17487/RFC10015

Bishop, M. (2022). *HTTP/3* (RFC 9114). Internet Engineering Task Force. https://doi.org/10.17487/RFC9114

Cloudflare, Inc. (2026). *Pingora TLS listener implementation* (source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs

Friedl, S., Popov, A., Langley, A., & Stephan, E. (2014). *Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension* (RFC 7301). Internet Engineering Task Force. https://doi.org/10.17487/RFC7301

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). Internet Engineering Task Force. https://doi.org/10.17487/RFC9000

OpenSSL Project Authors. (2026). *SSL/TLS context configuration API: protocol bounds and cipher lists*. https://docs.openssl.org/

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). Internet Engineering Task Force. https://doi.org/10.17487/RFC9846

Saint-Andre, P., & Salz, R. (2023). *Service identity in TLS* (RFC 9525). Internet Engineering Task Force. https://doi.org/10.17487/RFC9525

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). Internet Engineering Task Force. https://doi.org/10.17487/RFC9852

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113
