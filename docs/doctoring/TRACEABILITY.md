# Primary-Source and APA-7 Traceability

This file links material technical/security claims to primary standards or upstream sources. Revalidate version/advisory claims immediately before release.

| Claim | Source |
| --- | --- |
| Pingora server/proxy composition and graceful server lifecycle | Cloudflare Pingora source at pinned commit `6463ad6407a1d3fe256f1951dd0ecb054477e3f6` |
| Pingora's server default `max_retries` is 16, while the proxy loop copies that field and loops while its attempt counter is below the value | `pingora-core/src/server/configuration/mod.rs` and `pingora-proxy/src/lib.rs` at pinned commit `6463ad6407a1d3fe256f1951dd0ecb054477e3f6`; CWL v1 therefore sets the field to `1` for one total attempt |
| Graceful SIGTERM uses `grace_period_seconds` and `graceful_shutdown_timeout_seconds`, with framework fallbacks when unset | `pingora-core/src/server/mod.rs` and `pingora-core/src/server/configuration/mod.rs` at the pinned commit; CWL v1 sets 5 s grace and 10 s per-runtime graceful timeout explicitly inside a 30 s external termination budget |
| Standard upstream request policy supports hop-by-hop/connection-nominated stripping | Cloudflare Pingora `HttpUpstreamRequestPolicy` / peer implementation at the same pinned commit |
| Forwarded-header grammar and trust semantics | RFC 7239 |
| HTTP semantics | RFC 9110 |
| HTTP/1.1 message framing/hop-by-hop requirements | RFC 9112 |
| HTTP/2 framing and connection semantics | RFC 9113 |
| HTTP/3 semantics over QUIC | RFC 9114; HTTP/3 is not claimed implemented by this v1 candidate until executable listener/interoperability evidence exists |
| TLS 1.3 protocol semantics | RFC 8446; the current candidate proves verified upstream TLS mapping but still lacks a local-CA hostname-failure integration fixture |
| March 2026 Pingora request-smuggling/cache-key advisories are patched in 0.8.0 | GitHub Security Advisories GHSA-xq2h-p299-vjwv, GHSA-hj7x-879w-vrp7, GHSA-f93w-pcj3-rggc |
| Pingora 0.8.1 is the latest release observed on 2026-09-01 and bounds default HTTP/2 server limits | Cloudflare Pingora GitHub Releases, 0.8.1, 2026-06-04 |
| Rust 1.98.0 is the latest stable toolchain observed on 2026-09-01 | Rust Release Team, Rust 1.98.0 announcement, 2026-08-20 |
| OCI runtime-spec 1.2.1 is the latest released runtime specification observed on 2026-09-01 | Open Container Initiative runtime-spec v1.2.1 release notice, 2025-02-27; runtime hardening claims still require executable container evidence |
| `lru` versions before 0.18.2 are affected by RUSTSEC-2026-0253 | RustSec advisory RUSTSEC-2026-0253; hosted resolution for this branch selected 0.18.3, but release must use a committed audited lock |

## References

Cloudflare. (2026, June 4). *Pingora 0.8.1*. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.8.1

Cloudflare. (n.d.). *Pingora server configuration* [Source code, commit 6463ad6407a1d3fe256f1951dd0ecb054477e3f6]. GitHub. https://github.com/cloudflare/pingora/blob/6463ad6407a1d3fe256f1951dd0ecb054477e3f6/pingora-core/src/server/configuration/mod.rs

Cloudflare. (n.d.). *Pingora server lifecycle* [Source code, commit 6463ad6407a1d3fe256f1951dd0ecb054477e3f6]. GitHub. https://github.com/cloudflare/pingora/blob/6463ad6407a1d3fe256f1951dd0ecb054477e3f6/pingora-core/src/server/mod.rs

Cloudflare. (n.d.). *Pingora proxy implementation* [Source code, commit 6463ad6407a1d3fe256f1951dd0ecb054477e3f6]. GitHub. https://github.com/cloudflare/pingora/blob/6463ad6407a1d3fe256f1951dd0ecb054477e3f6/pingora-proxy/src/lib.rs

Cloudflare. (2026). *HTTP request smuggling via premature upgrade* (GHSA-xq2h-p299-vjwv). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-xq2h-p299-vjwv

Cloudflare. (2026). *HTTP request smuggling via HTTP/1.0 and Transfer-Encoding misparsing* (GHSA-hj7x-879w-vrp7). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-hj7x-879w-vrp7

Cloudflare. (2026). *Cache key poisoning advisory* (GHSA-f93w-pcj3-rggc). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-f93w-pcj3-rggc

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). RFC Editor. https://www.rfc-editor.org/rfc/rfc9110

Nottingham, M. (2022). *HTTP/1.1* (RFC 9112). RFC Editor. https://www.rfc-editor.org/rfc/rfc9112

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113

Bishop, M. (2022). *HTTP/3* (RFC 9114). RFC Editor. https://www.rfc-editor.org/rfc/rfc9114

Rescorla, E. (2018). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 8446). RFC Editor. https://www.rfc-editor.org/rfc/rfc8446

Petersson, A., & Nilsson, M. (2014). *Forwarded HTTP extension* (RFC 7239). RFC Editor. https://www.rfc-editor.org/rfc/rfc7239

Open Container Initiative. (2025, February 27). *OCI runtime-spec v. 1.2.1 release notice*. https://opencontainers.org/release-notices/v1-2-1-runtime-spec/

Rust Release Team. (2026, August 20). *Announcing Rust 1.98.0*. Rust Blog. https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/

Rust Secure Code Working Group. (2026, August 11). *RUSTSEC-2026-0253: lru—memory safety issue under panic*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2026-0253.html
