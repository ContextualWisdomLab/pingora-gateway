# Downstream TLS / HTTP/2 primary-source traceability

This companion traceability note scopes only the Proposed downstream TLS / HTTP/2 contract in ADR 0012. `docs/doctoring/TRACEABILITY.md` remains the repository-wide source ledger. Live supplier state must be revalidated before any release or protocol promotion.

## Claim-to-source map

| Claim / decision | Primary authority | Consequence for `pingora-gateway` |
| --- | --- | --- |
| TLS 1.3 is the current TLS 1.3 specification and RFC 9846 obsoletes RFC 8446 | Rescorla (2026), RFC 9846 | A future downstream TLS listener must be evaluated against current TLS 1.3 semantics rather than freezing an obsolete RFC 8446 citation. This does not imply that every migrated legacy consumer can drop TLS 1.2 without characterization. |
| Application service certificate identity verification is specified by RFC 9525 | Saint-Andre & Salz (2023), RFC 9525 | SNI/certificate acceptance must prove the configured reference identity. Certificate issuance/ACME/private-key custody remains outside the gateway. |
| HTTP/2 permits multiple Cookie field lines but requires them to be concatenated with ASCII `; ` before a non-HTTP/2 context | Thomson & Benfield (2022), RFC 9113 §8.2.3 | Current H1 upstream authority means any admitted downstream H2 must prove this normalization on the wire. A product-layer Cookie rewrite is not the preferred shared-runtime closure. |
| HTTP/2 has stream-level multiplexing, header-block rules, reset/GOAWAY semantics and protocol-specific decoded header accounting | Thomson & Benfield (2022), RFC 9113 | H2 admission needs concurrent-stream, malformed/header-limit, cancellation/reset and drain evidence; HTTP/1 parser-byte acceptance cannot be transferred. |
| HTTP/3 maps HTTP over QUIC and uses QPACK and QUIC streams | Bishop (2022), RFC 9114; Iyengar & Thomson (2021), RFC 9000 | TLS/H2 completion cannot be credited as H3/QUIC. UDP exposure, QPACK, QUIC flow/congestion, amplification, migration, 0-RTT, drain and deployment evidence remain a separate contract. |
| QUIC uses TLS 1.3 with QUIC-specific handshake mapping | Thomson & Turner (2021), RFC 9001 | A future H3 candidate cannot be implemented or certified by assigning an ALPN label to the existing TCP listener. |
| New protocols using TLS must require TLS 1.3 by default | Salz & Aviram (2026), RFC 9852 / BCP 195 | Relevant only if CWL defines a new protocol; enabling HTTPS for an existing HTTP migration is not treated as inventing a new application protocol. |
| Public Pingora protected `main` consumed for current characterization is `09696b51bc59315353d96686355861604d0bb48c` | Cloudflare Pingora protected branch, revalidated 2026-09-09 | Mutable contributor branches are evidence, not dependency authority. |
| H2 downstream → H1 upstream can duplicate the chunk terminator when the H2 body ends with empty DATA+END_STREAM on the reported supplier path | songhieu (2026), cloudflare/pingora#935 | H2 activation requires exact-supplier RED/GREEN with raw H1 bytes and connection reuse. |
| Public PR #936 changes zero-length chunked writes to no-ops and remains open/unmerged on 2026-09-09 | songhieu (2026), cloudflare/pingora#936 | #936 is a repair candidate and test oracle source, not an immutable production dependency. |
| Pingora issue #892 reports missing RFC 9113 multiple-Cookie concatenation on H2→H1 | MyLittleLuckyDog (2026), cloudflare/pingora#892 | H2 admission must prove exact single H1 Cookie field identity and `; ` joining on the consumed supplier. |
| Public PR #901 implements the reported Cookie concatenation and remains open/unmerged on 2026-09-09 | MyLittleLuckyDog (2026), cloudflare/pingora#901 | #901 is characterization/repair evidence only until maintainer integration produces immutable supplier authority. |

## Required executable linkage

ADR 0012 stays Proposed until a versioned downstream listener exists and the exact consumed supplier passes real TLS/H2 traffic. RED/GREEN must keep the current H1 upstream contract unchanged: valid SNI/certificate and admitted ALPN succeed; invalid identity or policy fails closed before useful application traffic; HTTP/1.1 fallback is deliberate; H2 concurrent streams, header/body limits, cancellation/reset, GOAWAY/drain and post-failure recovery remain usable.

The mixed-protocol fixture must capture raw H1 origin bytes. A streamed H2 request ending in empty DATA+END_STREAM must yield exactly one chunked terminator and the same H1 connection must successfully carry a later unrelated request. A request containing multiple decompressed H2 Cookie fields must arrive in the H1 context as one Cookie field joined with the two-octet delimiter `0x3b 0x20` (`; `). The fixture must also check pseudo-header and hop-by-hop translation rather than assuming that closing #935 and #892 proves every translation edge.

No evidence from h2c, an upstream-H2 switch, a retained Nginx/Traefik terminator, a mutable Pingora fork, or a CWL-local supplier-source copy is accepted as canonical TLS/H2 parity. HTTP/3/QUIC receives no GREEN from this lane.

## References

Bishop, M. (2022). *HTTP/3* (RFC 9114). RFC Editor. https://www.rfc-editor.org/rfc/rfc9114.html

Cloudflare. (n.d.). *Pingora* [Source code, commit `09696b51bc59315353d96686355861604d0bb48c`]. GitHub. https://github.com/cloudflare/pingora/tree/09696b51bc59315353d96686355861604d0bb48c

Iyengar, J., & Thomson, M. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). RFC Editor. https://www.rfc-editor.org/rfc/rfc9000.html

MyLittleLuckyDog. (2026, May 29). *HTTP/2 multiple Cookie headers not concatenated when proxied to HTTP/1.1 upstream (RFC 9113 §8.2.3)* [GitHub issue #892]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/892

MyLittleLuckyDog. (2026, June 2). *fix(proxy): concatenate HTTP/2 Cookie headers when proxying to HTTP/1.1 (RFC 9113 §8.2.3)* [GitHub pull request #901]. Cloudflare Pingora. https://github.com/cloudflare/pingora/pull/901

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). RFC Editor. https://www.rfc-editor.org/rfc/rfc9846.html

Saint-Andre, P., & Salz, R. (2023). *Service identity in TLS* (RFC 9525). RFC Editor. https://www.rfc-editor.org/rfc/rfc9525.html

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). RFC Editor. https://www.rfc-editor.org/rfc/rfc9852.html

songhieu. (2026, July 17). *H1 upstream: chunked terminator 0\\r\\n\\r\\n written twice when an H2 downstream ends the request body with an empty DATA frame (END_STREAM), poisoning upstream keep-alive connections* [GitHub issue #935]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/935

songhieu. (2026, July 17). *fix: don't emit the chunked terminator for zero-length body writes* [GitHub pull request #936]. Cloudflare Pingora. https://github.com/cloudflare/pingora/pull/936

Thomson, M., & Benfield, R. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113.html

Thomson, M., & Turner, S. (2021). *Using TLS to secure QUIC* (RFC 9001). RFC Editor. https://www.rfc-editor.org/rfc/rfc9001.html
