# Primary-Source and APA-7 Traceability

This file links material technical/security claims to primary standards or upstream sources. Revalidate version/advisory claims immediately before release.

| Claim | Source |
| --- | --- |
| Pingora server/proxy composition and graceful server lifecycle | Cloudflare Pingora source at pinned commit `09696b51bc59315353d96686355861604d0bb48c`, still the protected upstream `main` head observed on 2026-09-04 |
| Pingora's server default `max_retries` is 16, while the proxy loop copies that field and loops while its attempt counter is below the value | `pingora-core/src/server/configuration/mod.rs` and `pingora-proxy/src/lib.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; CWL v1 therefore sets the field to `1` for one total attempt |
| Graceful SIGTERM uses `grace_period_seconds` and `graceful_shutdown_timeout_seconds`, with framework fallbacks when unset | `pingora-core/src/server/mod.rs` and `pingora-core/src/server/configuration/mod.rs` at the pinned commit; CWL v1 sets 5 s grace and 10 s per-runtime graceful timeout explicitly inside a 30 s external termination budget |
| Standard upstream request policy supports hop-by-hop/connection-nominated stripping and normalized WebSocket-only HTTP/1 upgrade forwarding | Cloudflare Pingora `HttpUpstreamRequestPolicy` / peer implementation at pinned commit `09696b51bc59315353d96686355861604d0bb48c` |
| Pingora OpenSSL peers support a per-peer CA store; when configured it replaces the verification store for that peer while certificate and hostname verification remain separately enabled | `pingora-core/src/upstreams/peer.rs`, `pingora-core/src/connectors/tls/boringssl_openssl/mod.rs`, and `pingora-core/src/protocols/tls/boringssl_openssl/mod.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c` |
| Forwarded-header grammar and trust semantics | RFC 7239 |
| HTTP semantics | RFC 9110 |
| HTTP/1.1 message framing/hop-by-hop requirements | RFC 9112 |
| HTTP/2 framing and connection semantics | RFC 9113 |
| HTTP/3 semantics over QUIC | RFC 9114; HTTP/3 is not claimed implemented by this v1 candidate until executable listener/interoperability evidence exists |
| Current TLS 1.3 protocol semantics and application identity-verification responsibility | RFC 9846, published July 2026, which obsoletes RFC 8446 and points applications to RFC 9525 for identity verification |
| New protocols using TLS must require TLS 1.3 | RFC 9852, BCP 195, July 2026; this gateway is not claiming a new application protocol and still requires explicit migration-time protocol compatibility evidence |
| March 2026 Pingora request-smuggling/cache-key advisories are patched in 0.8.0 | GitHub Security Advisories GHSA-xq2h-p299-vjwv, GHSA-hj7x-879w-vrp7, GHSA-f93w-pcj3-rggc |
| Pingora 0.8.1 is the latest release observed on 2026-09-04 and bounds default HTTP/2 server limits | Cloudflare Pingora GitHub Releases, 0.8.1, 2026-06-04 |
| The pinned upstream head is seven commits after the prior security-resolution pin `6463ad6407a1d3fe256f1951dd0ecb054477e3f6`; the relevant retry/grace configuration remains unchanged at the new head | GitHub compare `6463ad6...09696b5` plus the exact `ServerConf` source at `09696b5` |
| Rust 1.98.1 is the latest stable toolchain observed on 2026-09-04; it fixes a vtable-generation miscompilation introduced in 1.98.0 that could emit null function pointers in trait-object vtables and cause undefined behavior | Rust Release Team, Rust 1.98.1 announcement, 2026-09-03; release-producing gateway paths therefore select and verify 1.98.1 rather than compiling with 1.98.0 |
| OCI runtime-spec 1.3.0 is the latest released runtime specification observed on 2026-09-04 | Open Container Initiative runtime-spec v1.3.0 release notice, 2025-11-04; runtime hardening claims still require executable container evidence |
| `lru` versions before 0.18.2 are affected by RUSTSEC-2026-0253 | RustSec advisory RUSTSEC-2026-0253; the upstream pin includes the first-fixed `lru` dependency change, but release must use a committed audited lock |
| `derivative` is classified as INFO Unmaintained and has no patched versions; the current committed Pingora graph resolves `derivative 2.2.0` | RustSec advisory RUSTSEC-2024-0388, issued 2024-11-10, plus exact committed `Cargo.lock`; upstream ownership is tracked in `cloudflare/pingora#889` |

## References

Cloudflare. (2026, June 4). *Pingora 0.8.1*. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.8.1

Cloudflare. (n.d.). *Pingora upstream peer options* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/upstreams/peer.rs

Cloudflare. (n.d.). *Pingora OpenSSL upstream TLS connector* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/connectors/tls/boringssl_openssl/mod.rs

Cloudflare. (n.d.). *Pingora server configuration* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/server/configuration/mod.rs

Cloudflare. (n.d.). *Pingora server lifecycle* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/server/mod.rs

Cloudflare. (n.d.). *Pingora proxy implementation* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/lib.rs

Cloudflare. (2026). *HTTP request smuggling via premature upgrade* (GHSA-xq2h-p299-vjwv). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-xq2h-p299-vjwv

Cloudflare. (2026). *HTTP request smuggling via HTTP/1.0 and Transfer-Encoding misparsing* (GHSA-hj7x-879w-vrp7). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-hj7x-879w-vrp7

Cloudflare. (2026). *Cache key poisoning advisory* (GHSA-f93w-pcj3-rggc). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-f93w-pcj3-rggc

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). RFC Editor. https://www.rfc-editor.org/rfc/rfc9110

Nottingham, M. (2022). *HTTP/1.1* (RFC 9112). RFC Editor. https://www.rfc-editor.org/rfc/rfc9112

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113

Bishop, M. (2022). *HTTP/3* (RFC 9114). RFC Editor. https://www.rfc-editor.org/rfc/rfc9114

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). RFC Editor. https://www.rfc-editor.org/rfc/rfc9846

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). RFC Editor. https://www.rfc-editor.org/rfc/rfc9852

Petersson, A., & Nilsson, M. (2014). *Forwarded HTTP extension* (RFC 7239). RFC Editor. https://www.rfc-editor.org/rfc/rfc7239

Open Container Initiative. (2025, November 4). *OCI runtime-spec v1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/

Rust Secure Code Working Group. (2024, November 10). *RUSTSEC-2024-0388: derivative—`derivative` is unmaintained; consider using an alternative*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2024-0388.html

Rust Secure Code Working Group. (2026, August 11). *RUSTSEC-2026-0253: lru—memory safety issue under panic*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2026-0253.html
