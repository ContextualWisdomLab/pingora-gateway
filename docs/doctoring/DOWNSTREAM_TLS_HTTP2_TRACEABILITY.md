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
- `TlsSettings::enable_h2()` maps to `ALPN::H2H1`, preferring HTTP/2 while allowing HTTP/1.1 fallback.
- `add_tls_with_settings` is used only after the versioned transport-neutral Admin Config has been admitted and materialized.

Supplier source: https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs

## Standards mapping

RFC 9113 defines HTTP/2. For HTTPS, the HTTP/2 protocol identifier is `h2`; h2c is a distinct non-TLS mode and is not admitted by this increment. RFC 7301 defines ALPN negotiation. The current `h2_http1` contract deliberately permits only the Pingora H2-preferred/H1-allowed policy rather than accepting arbitrary operator protocol lists.

RFC 9846 is the current TLS 1.3 specification and obsoletes RFC 8446. RFC 9525 supplies current service-identity verification guidance for TLS applications. The real-wire tests use a controlled CA and a certificate for `gateway.test` to exercise identity validation rather than disabling peer verification.

RFC 9000 and RFC 9114 show why HTTP/3 is not a TLS-listener checkbox: HTTP/3 is mapped over QUIC and therefore introduces a UDP/QUIC transport surface. H3 remains fail-closed until a release-qualified server capability and a separate operational/security contract exist.

## RED to implementation to GREEN chain

PR #75 was created from exact #73 base `625cae4f156366bc39d6782161a4a5f336d58624`. Its original tests established that the production roots had no versioned downstream TLS authority. The implementation then introduced:

- `DownstreamTlsConfig` and `DownstreamAlpnPolicy` in a transport-neutral bounded context;
- generic config version 2 and pg-erd config version 3 with fail-closed version transitions;
- `tls_delivery` as the Pingora adapter;
- conditional `add_tls_with_settings` listener composition in both production roots;
- real-wire H2, HTTP/1.1 fallback, pg-erd TLS and lifecycle/failure tests.

CodeRabbit review later found process-lifecycle defects in the real-wire fixtures. Ordinary forward repairs moved child ownership into cleanup guards before handshake attempts and replaced an unbounded `Command::output()` negative-startup path with deadline-bounded `try_wait()` plus kill/reap. All three review threads are resolved on the current lineage.

Exact predecessor `a7f12c8ee67bfe06a802aab66f7f038ec11bfb6f` completed:

- CI `34530278038`: formatting, locked compile/tests, Clippy, warnings-denied rustdoc, pinned coverage tooling, owned-production coverage enforcement, resolved-lock verification, rootless OCI runtime and both load contracts — success;
- Supply Chain `34530277984` — success;
- PgErd bounded-origin capacity `34530278009` — success.

The documentation/ADR repairs after that SHA create a new exact head. No predecessor GREEN transfers to the new head; the branch remains Draft until the new identity completes the required checks.

## What this increment proves

The current increment can prove, on an exact head with terminal checks:

- explicit versioned TLS listener authority rather than implicit transport mutation;
- read-only certificate/key materialization and mismatch failure before listener activation;
- verified TLS service identity in the real-wire fixture;
- negotiated `h2` over TLS;
- actual H2 request/response flow through the generic production root;
- deliberate HTTP/1.1 fallback over the same TLS policy;
- pg-erd TLS listener activation without converting its bounded route contract into a generic router;
- cleanup of child processes on success, panic, timeout and negative-startup paths.

## What remains RED / not release-qualified

This increment must not be promoted to complete H2 production parity by inference. Issue #51 still requires realistic acceptance for concurrent streams, reset/cancellation, GOAWAY/graceful drain, header/body admission, flow control/backpressure, origin failure/recovery, readiness, forwarding trust, new-handshake versus reused-connection timing and rollback/cutover observability.

Two supplier roots remain especially important for H2 downstream to H1 upstream translation:

- `cloudflare/pingora#901`: Cookie field coalescing semantics;
- `cloudflare/pingora#936`: zero-length DATA/body-termination semantics.

Open contributor heads are not released dependency authority. The gateway must consume a maintainer-integrated/release-qualified supplier identity or enforce a versioned deployment protocol path that makes the affected downgrade unreachable before claiming parity.

HTTP/3/QUIC remains unsupported. No H3 credit is earned from TLS, ALPN or H2 evidence.

## Promotion boundary

After exact-head source/test/documentation checks and independent review, this branch may participate in ordinary non-force stack integration. Release still requires the foundation/compiler and supplier roots identified by canonical issue #58, protected integration, version/CHANGELOG/tag/package, immutable artifact digest, SBOM/provenance/reproducibility, rollback and concrete consumer parity. Shadow/canary, observed rollback, cutover and verified legacy Nginx/OpenResty removal remain deployment evidence, not source evidence.

## Primary references (APA 7th)

Bishop, M. (2022). *HTTP/3* (RFC 9114). Internet Engineering Task Force. https://doi.org/10.17487/RFC9114

Friedl, S., Popov, A., Langley, A., & Stephan, E. (2014). *Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension* (RFC 7301). Internet Engineering Task Force. https://doi.org/10.17487/RFC7301

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). Internet Engineering Task Force. https://doi.org/10.17487/RFC9000

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). Internet Engineering Task Force. https://doi.org/10.17487/RFC9846

Saint-Andre, P., & Salz, R. (2023). *Service identity in TLS* (RFC 9525). Internet Engineering Task Force. https://doi.org/10.17487/RFC9525

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113

Cloudflare, Inc. (2026). *Pingora TLS listener implementation* (source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs
