# Downstream TLS / HTTP/2 traceability

Last verified: 2026-09-11

This note binds the downstream TLS/H2 migration to released supplier identities, current Internet standards, executable real-wire evidence, and the remaining promotion boundary. It is evidence documentation, not a protected-release or cutover claim. `docs/product-technical-gap-baseline.md` remains owned by the dedicated documentation lane #61 and is not modified by this protocol writer.

## Authority boundary

`pingora-gateway` owns listener transport, TLS materialization, ALPN admission, reusable HTTP edge behavior, load/runtime isolation, and transport-lifecycle acceptance. Certificate issuance, renewal, revocation, ACME, backup, and private-key custody remain external. Product authentication/authorization/business routing, Keyverse identity, and Wardnet/EgressWeave verdict authority are not duplicated here.

Generic configuration version 1 and pg-erd versions 1/2 remain cleartext. Generic version 2 and pg-erd version 3 require downstream TLS. Earlier versions reject the new TLS field. The gateway consumes released/versioned supplier behavior and does not copy mutable supplier source.

## Supplier mapping

Released dependency authority remains `pingora = 0.9.0` and `pingora-prometheus = 0.9.0` with the committed Cargo lock. Pingora tag `0.9.0` resolves exactly to `702f69015e53f7244d6ad2e743de571d859a70a4`. The resolved HTTP/2 codec is `h2 = 0.4.19`; tag `v0.4.19` resolves to `d57d1b852fec9dda6d42d3454502006d52104da8`.

`TlsSettings::intermediate(cert_path, key_path)` constructs the server acceptor and loads PEM material. CWL performs explicit private-key/certificate pairing before listener activation and installs the stricter ALPN callback through Pingora's exposed OpenSSL-compatible acceptor builder. Optional downstream TLS handshake offload is not enabled as a default; representative handshake/reuse CPU and NUMA evidence must justify any later tuning authority.

Pingora 0.9.0 broadcasts service shutdown before the process-level `grace_period_seconds` wait. HTTP/2 graceful GOAWAY can therefore begin while process grace is active. The resolved h2 0.4.19 graceful path sends initial `GOAWAY(NO_ERROR, 2^31-1)` and a shutdown PING before later GOAWAY; a raw acceptance client must ACK a non-ACK PING with the identical eight-octet payload.

For H2 body transmission, released Pingora 0.9.0 `pingora-core/src/protocols/http/v2/mod.rs` calls `SendStream::reserve_capacity(remaining.len())`, awaits `poll_capacity`, sends only the capacity granted, and repeats until the body is exhausted. The downstream acceptance therefore verifies the observable end-to-end consequence—connection-credit exhaustion must propagate backpressure to the H1 origin before an intentionally large body can be handed to the gateway in full—rather than assuming that supplier source structure alone proves bounded memory.

Primary supplier source:

- https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/listeners/tls/boringssl_openssl/mod.rs
- https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/server/configuration/mod.rs
- https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/server/mod.rs
- https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v2/mod.rs
- https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v2/server.rs
- https://github.com/hyperium/h2/blob/d57d1b852fec9dda6d42d3454502006d52104da8/src/proto/connection.rs

## Standards mapping

RFC 7301 governs ALPN. The `h2_http1` selector validates the entire length-prefixed protocol vector, prefers `h2`, falls back to `http/1.1` only when offered, and fails closed on malformed/no-overlap vectors. No-ALPN TLS remains a separately tested HTTP/1 compatibility path.

RFC 9113 governs HTTP/2. `RST_STREAM` is stream-scoped termination; GOAWAY is connection lifecycle; SETTINGS and PING have explicit acknowledgement rules. SETTINGS_MAX_HEADER_LIST_SIZE is an advisory maximum field-section size after decompression, so the decoded-header fixture uses legal HEADERS/CONTINUATION frame sizes and cannot manufacture success through frame-size rejection. DATA is subject to both per-stream and connection windows; WINDOW_UPDATE changes only the corresponding flow-control credit. SETTINGS_INITIAL_WINDOW_SIZE changes stream windows and does not replace connection-level WINDOW_UPDATE.

The #82 fixture isolates per-stream pressure by withholding one stream's WINDOW_UPDATE while replenishing shared connection credit. #83 isolates shared connection pressure by making stream windows non-limiting, consuming exactly the initial 65,535-byte connection DATA window, requiring sibling origin dispatch and PING liveness while shared DATA credit is zero, then recovering with stream-0 WINDOW_UPDATE only. Draft #84 adds the next causal layer: after the same shared-window exhaustion, a 64 MiB H1 origin body is written through a nonblocking socket and must encounter `WouldBlock` before full-body handoff; after connection credit is restored, the response must finish byte-for-byte. That oracle demonstrates bounded upstream read-ahead under sustained downstream pressure without treating any host-specific RSS number as a product limit.

RFC 9113 also permits a server to complete a response before the request body finishes and then stop remaining request transmission with protocol-appropriate stream cancellation. Request-body admission therefore uses completed HTTP response plus same-connection sibling recovery as the transport semantic boundary; a later stream-level reset is not reclassified as a connection failure.

RFC 9110 §15.5.14 defines `413 Content Too Large`. CWL owns one versioned `max_request_body_bytes` policy across downstream HTTP versions: declared `Content-Length` is checked before upstream selection and streamed bytes are observed incrementally. HTTP/1 real-wire evidence proves the stable 413 mapping. The H2 body fixture proves transport parity without independently HPACK-decoding the status.

RFC 9112 defines the HTTP/1.1 header section through the terminating empty line. Origin fixtures therefore read through `\r\n\r\n` under one monotonic deadline instead of treating one TCP `read()` as a message boundary. The production whole-header lifetime gap remains distinct under upstream #447.

RFC 9846 is the current TLS 1.3 specification. RFC 9852 updates BCP 195 so new protocols using TLS require TLS 1.3. This migration serves existing HTTPS/H2 consumers, so the code-owned compatibility policy remains an explicit TLS 1.2 floor / TLS 1.3 ceiling with tested cipher constraints; it is not silently converted to TLS-1.3-only. RFC 9525 supplies service-identity guidance for the controlled-CA fixtures.

RFC 9000 and RFC 9114 keep HTTP/3 outside this increment. HTTP/3 is a QUIC/UDP transport surface, not a TLS-listener flag. H3 remains fail closed until a maintainer-supported, release-qualified server integration and separate operational/security acceptance exist.

## Executable evidence chain

PR #75 exact `df70de9cc0c77cfc1dabc51de039ac46af47df08` proves versioned downstream TLS activation, certificate/key pairing, strict ALPN selection, verified HTTP/1.1 fallback and no-ALPN compatibility. It is exact-head GREEN/Ready and awaits independent governance.

PR #76 exact `2c4433b99c539a43bdecc74c7d167b2446dd4fdc` proves the explicit TLS 1.2–1.3 profile, selected TLS 1.2 ECDHE+AEAD and TLS 1.3 AEAD suites, real-wire TLS 1.2/TLS 1.3 acceptance, and TLS 1.1 rejection. It is exact-head GREEN/Ready and awaits independent governance.

PR #77 exact `0d583374607f32876fa17ef0501a8a548fb074f1` proves concurrent H2 multiplexing: two streams on one verified H2 connection must create two live H1 origin requests before either response can manufacture success. It is exact-head GREEN/Ready and awaits independent governance.

PR #78 exact `b4c54e22c252aa99a19b2b90881343653c6aa995` proves one-stream cancellation releases its dedicated origin while a sibling on the same H2 connection completes with exact body `sibling-ok`. It is exact-head GREEN/Ready and awaits independent governance.

PR #79 exact `c0406907aea4c2a13d76d219cfacd9c508a2a467` proves graceful H2 GOAWAY/drain with released Pingora/h2 shutdown ordering and protocol-correct PING ACK behavior. It is exact-head GREEN/Ready and awaits independent governance.

PR #80 exact `f98b359cf01e6bc46eb5f50abd1d7cf5adcd7141` proves decoded-header admission without letting frame-size rejection substitute for decompressed field-section admission. It is exact-head GREEN/Ready and awaits independent governance.

PR #81 exact `6ce632d6ff23469d22d7e6b0746ce42c9bdc98e2` proves negotiated-H2 request-body admission parity for declared and streamed over-limit bodies while a compliant same-connection sibling remains usable. It is exact-head GREEN/Ready and awaits independent governance.

PR #82 exact `03c20c08ee40466037c4e7bc2dcf143c83fde23b` proves stream-window isolation and sibling fairness under a withheld 16 KiB stream window while connection credit is replenished. It is exact-head GREEN/Ready and awaits independent governance.

PR #83 exact `6d0bf05bae5bff0e3b4360024567716e59350236` proves connection-window exhaustion/recovery. CI `34584075862`, Supply Chain `34584075751`, and PgErd bounded-origin capacity `34584076080` are terminal GREEN; review threads are empty. It is Ready for independent governance.

PR #84 is Draft and currently owns only the bounded upstream read-ahead acceptance plus this focused traceability update. Its initial test-only exact `96faf62b856fc94221783bb28cc0dffced48e089` deliberately does not inherit #83 GREEN. The current branch must independently pass formatting, compile/test, Clippy, rustdoc, owned-production coverage, Supply Chain and capacity gates after this documentation movement before any Ready or promotion claim.

## Supplier protocol roots that remain open

Full H2-downstream to H1-upstream parity is not inferred from the TLS/H2 listener tests.

- `cloudflare/pingora#901` remains an open mutable contributor path for RFC 9113 multiple-Cookie coalescing before H1 translation.
- `cloudflare/pingora#976` remains an open mutable candidate for zero-length HTTP/1 upstream-body handling.
- `cloudflare/pingora#1000` remains open mutable contributor evidence for configurable HTTP/1 request-header parser admission.
- `cloudflare/pingora#889` remains the `derivative 2.2.0` / RUSTSEC-2024-0388 commercial dependency root.
- `cloudflare/pingora#447` remains the downstream HTTP/1 connection/header-lifetime root.

The gateway must consume a maintainer-integrated, release-qualified supplier identity or a separately versioned owner contract that makes the affected path unreachable. Mutable contributor heads are evidence, not dependency authority.

## Remaining acceptance and promotion boundary

After #84 bounded-read-ahead acceptance is exact-current GREEN and governed, issue #51 still owns partial-body upstream cancellation semantics, pre/post-commit H2 origin failure and recovery, forwarding/client-IP trust, release-qualified mixed-protocol Cookie/body framing, handshake versus reused-connection timing and representative CPU/NUMA profiling, rollback/cutover observability, and any later HTTP/3 admission.

Promotion remains dependency ordered: unchanged exact-head checks → independent ruleset-valid approval/governance → protected integration without bypass → version/CHANGELOG/tag/package and immutable gateway artifact identity → SBOM/provenance/reproducibility/rollback → remaining parity → shadow/canary → observed rollback → cutover → verified Nginx/OpenResty removal. Source capability, owner/bot comments, predecessor GREEN, mutable supplier PRs, or loopback-only performance are not release/cutover evidence.

## Primary references (APA 7th)

Bishop, M. (2022). *HTTP/3* (RFC 9114). Internet Engineering Task Force. https://doi.org/10.17487/RFC9114

Cloudflare, Inc. (2026). *Pingora HTTP/2 body-write implementation* (release 0.9.0, source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v2/mod.rs

Cloudflare, Inc. (2026). *Pingora HTTP/2 server implementation* (release 0.9.0, source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v2/server.rs

Cloudflare, Inc. (2026). *Pingora server runtime and graceful-shutdown implementation* (release 0.9.0, source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/server/mod.rs

Fielding, R. T., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). Internet Engineering Task Force. https://doi.org/10.17487/RFC9110

Friedl, S., Popov, A., Langley, A., & Stephan, E. (2014). *Transport Layer Security (TLS) Application-Layer Protocol Negotiation Extension* (RFC 7301). Internet Engineering Task Force. https://doi.org/10.17487/RFC7301

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). Internet Engineering Task Force. https://doi.org/10.17487/RFC9000

Nottingham, M., Fielding, R. T., & Reschke, J. (2022). *HTTP/1.1* (RFC 9112). Internet Engineering Task Force. https://doi.org/10.17487/RFC9112

Rescorla, E. (2018). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 8446; superseded by RFC 9846). Internet Engineering Task Force.

Saint-Andre, P., & Valin, M. (2023). *Service identity in TLS* (RFC 9525). Internet Engineering Task Force. https://doi.org/10.17487/RFC9525

Thomson, M., & Benfield, B. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113

Thomson, M., & Turner, S. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). Internet Engineering Task Force. https://doi.org/10.17487/RFC9846

Thomson, M., & Turner, S. (2026). *Recommendations for secure use of Transport Layer Security (TLS) and Datagram Transport Layer Security (DTLS)* (RFC 9852 / BCP 195). Internet Engineering Task Force. https://doi.org/10.17487/RFC9852
