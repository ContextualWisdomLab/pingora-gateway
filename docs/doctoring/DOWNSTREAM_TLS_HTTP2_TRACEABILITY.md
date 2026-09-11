# Downstream TLS / HTTP/2 traceability

Last verified: 2026-09-11

This note binds the versioned downstream TLS/H2 migration to released Pingora source, Internet standards, executable real-wire evidence, and the remaining promotion boundary. It is evidence documentation, not a release or cutover claim.

## Authority boundary

`pingora-gateway` owns listener transport, TLS materialization, ALPN admission, reusable HTTP edge behavior, and transport-lifecycle acceptance. It consumes certificate/private-key references read-only. Certificate issuance, renewal, revocation, ACME, backup, and private-key custody remain external. Product authentication/authorization/business routing, Keyverse identity, and Wardnet/EgressWeave verdict authority are not duplicated here.

Generic configuration version 1 and pg-erd versions 1/2 remain cleartext. Generic version 2 and pg-erd version 3 require downstream TLS. Pg-erd version 3 retains version-2 response-body lifetime semantics. Earlier versions reject the new TLS field.

## Supplier mapping

Released dependency authority remains `pingora = 0.9.0` and `pingora-prometheus = 0.9.0` from the registry with the committed Cargo lock. The protected Cloudflare source rechecked for this slice remains `702f69015e53f7244d6ad2e743de571d859a70a4`.

At that source, `TlsSettings::intermediate(cert_path, key_path)` constructs the server acceptor and loads PEM material, while CWL performs an explicit private-key/certificate pairing check before listener activation. Pingora exposes its OpenSSL-compatible acceptor builder, allowing CWL to install the stricter ALPN callback without copying supplier source. `add_tls_with_settings` is used only after versioned transport-neutral configuration admission and materialization.

Pingora also exposes optional downstream TLS handshake offload pools. This migration does not enable or expose those knobs by default; representative handshake/reuse CPU and NUMA evidence must justify any later tuning authority.

Supplier sources:

- https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs
- https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/server/configuration/mod.rs

## Standards mapping

RFC 9113 defines HTTP/2. For HTTPS the application protocol identifier is `h2`; h2c is a separate cleartext mode and is not admitted here. Section 6.4 defines `RST_STREAM` as immediate termination of the referenced stream, and Section 7 assigns error code `CANCEL` (`0x8`) when a stream is no longer needed. A cancellation acceptance therefore has two distinct obligations: the cancelled stream must stop consuming its dedicated upstream work within a bounded interval, and an unrelated live sibling stream on the same HTTP/2 connection must remain usable.

RFC 7301 requires the server to select a common ALPN protocol and to fail an ALPN-bearing handshake with fatal `no_application_protocol` when there is no overlap. The CWL `h2_http1` selector validates the entire length-prefixed protocol vector before selection, prefers `h2`, falls back to `http/1.1` only when offered, and fails closed on malformed/no-overlap vectors. A client that omits ALPN is a separate compatibility case: the callback is not invoked, verified TLS may complete without a negotiated protocol, and Pingora can continue on the ordinary HTTP/1 path.

RFC 9112 defines the HTTP/1.1 header section through the terminating empty line. Real-origin fixtures therefore read through `\r\n\r\n` with a 64 KiB evidence cap instead of treating one TCP `read()` as a message boundary. One monotonic `Instant` deadline is used across those fixture reads; a socket read timeout is not credited as an end-to-end HTTP-header lifetime. The production whole-header lifetime gap remains separate under #45 / upstream #447.

RFC 9846 is the current TLS 1.3 specification. RFC 9852 updates BCP 195 so new protocols using TLS require TLS 1.3. This migration serves existing HTTPS/H2 consumers, so the selected code-owned policy remains an explicit TLS 1.2 floor / TLS 1.3 ceiling with tested cipher constraints rather than silently converting an existing protocol surface to TLS-1.3-only. RFC 9525 supplies the service-identity verification guidance used by the controlled-CA fixtures.

RFC 9000 and RFC 9114 keep HTTP/3 outside this increment: HTTP/3 is mapped over QUIC and introduces a UDP/QUIC transport surface, not a TLS-listener flag. H3 remains fail-closed until a maintainer-supported, release-qualified server integration and separate operational/security contract exist.

## Executable evidence chain

PR #75, based on exact #73 `625cae4f156366bc39d6782161a4a5f336d58624`, introduced the versioned downstream TLS/H2 listener. Its ordinary-forward repair chain added strict ALPN no-overlap failure, complete-vector validation, verified no-ALPN HTTP/1.1 compatibility, bounded complete-header origin reads, a monotonic fixture deadline, and coverage-compatible in-process ALPN evidence without changing production routing or authority boundaries. Current #75 exact `df70de9cc0c77cfc1dabc51de039ac46af47df08` is terminal GREEN and Ready for independent governance.

PR #76 adds the explicit downstream TLS security profile. Current exact `2c4433b99c539a43bdecc74c7d167b2446dd4fdc` keeps TLS 1.2 minimum / TLS 1.3 maximum, code-owned TLS 1.2 ECDHE+AEAD suites, selected TLS 1.3 AEAD suites, real-wire TLS 1.2/TLS 1.3 acceptance, and TLS 1.1 rejection. Its exact CI `34558856760`, Supply Chain `34558856761`, and PgErd bounded-origin capacity `34558856758` are terminal GREEN. It is Ready but has no ruleset-valid independent `APPROVED` review.

PR #77 adds the concurrent-stream H2 oracle. Current exact `0d583374607f32876fa17ef0501a8a548fb074f1` sends two streams on one certificate-verified H2 connection and requires two distinct HTTP/1 origin requests to be live before either response can manufacture success. Its exact CI `34558985477`, Supply Chain `34558985542`, and capacity `34558985485` are terminal GREEN. It is Ready but likewise awaits independent governance.

PR #78 is the next lifecycle child. Initial exact `b965eceacbd1c00163ac380d9fe406a3bff901f2` stopped at `cargo fmt --all -- --check` before compilation; the runner diff only wrapped two final assertions. Ordinary-forward exact `13f2520249169f18644094e244c7133ae375c35a` applied precisely that formatting repair. On that exact head, CI `34562439644`, Supply Chain `34562439646`, and PgErd bounded-origin capacity `34562439638` are terminal GREEN.

The #78 real-wire fixture sends stream 1 (`/`) and stream 3 (`/index.html`) on one certificate-verified H2 connection. The origin refuses to expose the sibling response until both dedicated HTTP/1 origin requests have been observed, so a serialized gateway cannot manufacture GREEN. Once stream 3 has begun responding, the client emits `RST_STREAM(CANCEL)` for stream 1. Acceptance requires stream 3 to avoid `RST_STREAM`, reach `END_STREAM`, and return exact body `sibling-ok`, while the cancelled stream's dedicated origin connection reaches EOF/reset within three seconds. This closes the branch-local executable reset/cancellation-and-sibling-survival acceptance; it does not by itself promote the branch or prove GOAWAY/drain, flow-control, supplier mixed-protocol correctness, or cutover readiness.

This documentation update is an ordinary forward movement after that exact execution. Its new exact head must independently reacquire all applicable checks; the GREEN receipts above remain evidence for `13f252...` only and do not transfer to the documentation-moving head.

## Supplier protocol roots that remain open

Full H2-downstream to H1-upstream parity is not inferred from the TLS/H2 listener tests.

- `cloudflare/pingora#901` remains an open mutable contributor path for RFC 9113 multiple-Cookie coalescing before H1 translation.
- Zero-length body termination remains represented by open mutable #936/#976 candidates; a maintainer-integrated, release-qualified disposition is required before parity credit.
- `cloudflare/pingora#1000` remains mutable contributor evidence for parser admission rather than released dependency authority.
- `cloudflare/pingora#889` remains the `derivative 2.2.0` / RUSTSEC-2024-0388 commercial dependency root.
- `cloudflare/pingora#447` remains the downstream H1 whole-header lifetime root.

The gateway must consume a maintainer-integrated, release-qualified supplier identity or enforce a versioned deployment path that makes an affected downgrade unreachable. Mutable contributor heads are evidence, not dependency authority.

## Remaining acceptance and promotion boundary

Branch-local TLS/H2 evidence now covers versioned TLS activation, material pairing, verified service identity, strict ALPN admission, H1/no-ALPN compatibility, explicit TLS version/cipher policy, real H2 request/response flow, concurrent streams, and one-stream `RST_STREAM(CANCEL)` with sibling survival plus bounded upstream release.

Issue #51 still owns GOAWAY/drain/new-stream behavior, decoded header-list/body admission, connection/stream flow-control and backpressure, pre/post-commit origin failure and recovery, forwarding/client-IP trust, release-qualified mixed-protocol Cookie/body framing, handshake versus reused-connection timing and representative CPU/NUMA profiling, rollback/cutover observability, and any later HTTP/3 admission.

Promotion remains dependency ordered: unchanged exact-head checks → independent ruleset-valid approval/governance → protected integration without bypass → version/CHANGELOG/tag/package and immutable gateway artifact identity → SBOM/provenance/reproducibility/rollback → remaining parity → shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. Source capability, owner/bot comments, predecessor GREEN, or mutable supplier PRs are not release/cutover evidence.

## Primary references (APA 7th)

Bishop, M. (2022). *HTTP/3* (RFC 9114). Internet Engineering Task Force. https://doi.org/10.17487/RFC9114

Cloudflare, Inc. (2026). *Pingora server configuration* (source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/server/configuration/mod.rs

Cloudflare, Inc. (2026). *Pingora TLS listener implementation* (source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs

Fielding, R. T., Nottingham, M., & Reschke, J. (2022). *HTTP/1.1* (RFC 9112). Internet Engineering Task Force. https://doi.org/10.17487/RFC9112

Friedl, S., Popov, A., Langley, A., & Stephan, E. (2014). *Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension* (RFC 7301). Internet Engineering Task Force. https://doi.org/10.17487/RFC7301

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). Internet Engineering Task Force. https://doi.org/10.17487/RFC9000

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). Internet Engineering Task Force. https://doi.org/10.17487/RFC9846

Saint-Andre, P., & Salz, R. (2023). *Service identity in TLS* (RFC 9525). Internet Engineering Task Force. https://doi.org/10.17487/RFC9525

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). Internet Engineering Task Force. https://doi.org/10.17487/RFC9852

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113
