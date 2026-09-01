# Primary-Source and APA-7 Traceability

This file links material technical/security claims to primary standards or upstream sources. Revalidate version/advisory claims immediately before release.

| Claim | Source |
| --- | --- |
| Pingora server/proxy composition and graceful server lifecycle | Cloudflare Pingora source at pinned commit `6463ad6407a1d3fe256f1951dd0ecb054477e3f6` |
| Pingora's server default `max_retries` is 16, while the proxy loop copies that field and loops while its attempt counter is below the value | `pingora-core/src/server/configuration/mod.rs` and `pingora-proxy/src/lib.rs` at pinned commit `6463ad6407a1d3fe256f1951dd0ecb054477e3f6`; CWL v1 therefore sets the field to `1` for one total attempt |
| Graceful SIGTERM uses `grace_period_seconds` and `graceful_shutdown_timeout_seconds`, with framework fallbacks when unset | `pingora-core/src/server/mod.rs` and `pingora-core/src/server/configuration/mod.rs` at the pinned commit; CWL v1 sets 5 s and 30 s explicitly |
| Standard upstream request policy supports hop-by-hop/connection-nominated stripping | Cloudflare Pingora `HttpUpstreamRequestPolicy` / peer implementation at the same pinned commit |
| Forwarded-header grammar and trust semantics | RFC 7239 |
| HTTP/1.1 message framing/hop-by-hop requirements | RFC 9112 |
| HTTP semantics | RFC 9110 |
| March 2026 Pingora request-smuggling/cache-key advisories are patched in 0.8.0 | GitHub Security Advisories GHSA-xq2h-p299-vjwv, GHSA-hj7x-879w-vrp7, GHSA-f93w-pcj3-rggc |
| Pingora 0.8.1 is the latest release observed on 2026-09-01 | Cloudflare Pingora GitHub Releases, 0.8.1, 2026-06-04 |
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

Petersson, A., & Nilsson, M. (2014). *Forwarded HTTP extension* (RFC 7239). RFC Editor. https://www.rfc-editor.org/rfc/rfc7239

Rust Secure Code Working Group. (2026, August 11). *RUSTSEC-2026-0253: lru—memory safety issue under panic*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2026-0253.html
