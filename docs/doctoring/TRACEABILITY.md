# Primary-Source and APA-7 Traceability

This file links material technical/security claims to primary standards or upstream sources. Revalidate version/advisory claims immediately before release.

| Claim | Source |
| --- | --- |
| Pingora server/proxy composition and graceful server lifecycle | Cloudflare Pingora source at pinned commit `09696b51bc59315353d96686355861604d0bb48c`, still the protected upstream `main` head observed on 2026-09-06 |
| Pingora's server default `max_retries` is 16, while the proxy loop copies that field and loops while its attempt counter is below the value | `pingora-core/src/server/configuration/mod.rs` and `pingora-proxy/src/lib.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; CWL v1 therefore sets the field to `1` for one total attempt |
| Graceful SIGTERM uses `grace_period_seconds` and `graceful_shutdown_timeout_seconds`, with framework fallbacks when unset | `pingora-core/src/server/mod.rs` and `pingora-core/src/server/configuration/mod.rs` at the pinned commit; CWL v1 sets 5 s grace and 10 s per-runtime graceful timeout explicitly inside a 30 s external termination budget |
| Standard upstream request policy supports hop-by-hop/connection-nominated stripping and normalized WebSocket-only HTTP/1 upgrade forwarding | Cloudflare Pingora `HttpUpstreamRequestPolicy` / peer implementation at pinned commit `09696b51bc59315353d96686355861604d0bb48c` |
| Pingora OpenSSL peers support a per-peer CA store; when configured it replaces the verification store for that peer while certificate and hostname verification remain separately enabled | `pingora-core/src/upstreams/peer.rs`, `pingora-core/src/connectors/tls/boringssl_openssl/mod.rs`, and `pingora-core/src/protocols/tls/boringssl_openssl/mod.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c` |
| Forwarded-header grammar and trust semantics | RFC 7239 |
| HTTP semantics | RFC 9110 |
| HTTP/1.1 message framing/hop-by-hop requirements | RFC 9112 |
| HTTP/2 framing and connection semantics, including H2 Cookie-field reconstruction before a non-H2 hop | RFC 9113 |
| HTTP/3 semantics over QUIC | RFC 9114; HTTP/3 is not claimed implemented by this v1 candidate until executable listener/interoperability evidence exists |
| Current TLS 1.3 protocol semantics and application identity-verification responsibility | RFC 9846, published July 2026, which obsoletes RFC 8446 and points applications to RFC 9525 for identity verification |
| New protocols using TLS must require TLS 1.3 | RFC 9852, BCP 195, July 2026; this gateway is not claiming a new application protocol and still requires explicit migration-time protocol compatibility evidence |
| GitHub Actions concurrency permits `cancel-in-progress` to be a conditional expression; group names should include workflow identity to prevent cross-workflow cancellation; event-specific properties may use `github.run_id` as a guaranteed unique fallback | GitHub Docs, *Control the concurrency of workflows and jobs*, revalidated 2026-09-05 |
| With the default single pending slot, a newly queued run in the same concurrency group replaces an existing pending run even when `cancel-in-progress` is false | GitHub Docs, *Control the concurrency of workflows and jobs*; CWL rerun isolation therefore avoids placing historical reruns in the same first-attempt PR group |
| Pull-request workflow activity can explicitly include both `converted_to_draft` and `ready_for_review`, and job-level `if` conditions are evaluated before a job is routed to a runner | GitHub Docs, *Events that trigger workflows* and *Contexts reference*, revalidated 2026-09-05; CWL uses the draft-conversion event in the same PR concurrency group to retract superseded Ready work while direct jobs skip, then uses Ready re-admission to restore normal checks |
| Re-running a workflow uses the same original `GITHUB_SHA` and `GITHUB_REF` | GitHub Docs, *Re-running workflows and jobs*; rerun evidence is historical execution of the same triggered revision, not a new source revision |
| Cargo can select a compiler executable independently of a prior standalone `rustc` verification through `RUSTC` or the `CARGO_BUILD_RUSTC` configuration environment variable (`build.rustc`) | The Cargo Book, *Environment Variables*; release-path acceptance therefore rejects those secondary compiler authorities after selecting/verifying Rust 1.98.1 |
| Bash declaration commands give assignment arguments assignment-statement semantics; `declare` and `typeset` can therefore persist a Cargo executable variable in the current workflow shell just as `export`/`readonly` can persist assignment authority | GNU Bash Reference Manual, *Shell Parameters* / *Shell Builtin Commands*, revalidated 2026-09-06. The current oracle explicitly covers observed release-path `export`, `readonly`, `declare`, and `typeset` forms; no `local CARGO` release-path use was found, so function-local parsing is not claimed complete |
| GNU Coreutils `env -S` / `--split-string=STRING` performs a second argument-splitting pass; short options can be combined (for example `-vS`); `--` delimits only the option list; `NAME=VALUE` operands still precede the first non-assignment command operand, and all later words are child-command arguments | GNU Coreutils 9.11 manual, *Common options* and *env invocation*; the gateway release-path oracle therefore fails closed on split-string/unmodelled option grammar, continues compiler-assignment inspection after `env --`, and stops only at the first child command so child arguments such as `/usr/bin/printf -S` or assignment-looking strings are not misclassified |
| March 2026 Pingora request-smuggling/cache-key advisories are patched in 0.8.0 | GitHub Security Advisories GHSA-xq2h-p299-vjwv, GHSA-hj7x-879w-vrp7, GHSA-f93w-pcj3-rggc |
| Pingora 0.8.1 is the latest public GitHub Release observed on 2026-09-05 and bounds default HTTP/2 server limits | Cloudflare Pingora GitHub Releases, 0.8.1, 2026-06-04; the observed GitHub release object is not marked immutable |
| The pinned upstream head is seven commits after the prior security-resolution pin `6463ad6407a1d3fe256f1951dd0ecb054477e3f6`; the relevant retry/grace configuration remains unchanged at the new head | GitHub compare `6463ad6...09696b5` plus the exact `ServerConf` source at `09696b5` |
| Rust 1.98.1 repairs a Rust 1.98.0 vtable-generation miscompilation that can emit a null pointer where a trait-object function pointer should be, causing undefined behavior | Rust Release Team, Rust 1.98.1 announcement, 2026-09-03. Draft #56 is the separately gated release-path repair and must not be treated as inherited before integration |
| OCI runtime-spec 1.3.0 is the latest released runtime specification observed on 2026-09-05 | Open Container Initiative runtime-spec v1.3.0 release notice and release list; runtime hardening claims still require executable container evidence |
| `lru` versions before 0.18.2 are affected by RUSTSEC-2026-0253 | RustSec advisory RUSTSEC-2026-0253; the upstream pin includes the first-fixed `lru` dependency change, but release must use a committed audited lock |
| `derivative` is unmaintained and RUSTSEC-2024-0388 has no patched versions | RustSec advisory RUSTSEC-2024-0388. The downstream policy therefore requires a maintainer-integrated supplier repair/removal rather than a generic audit ignore |

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

GitHub. (n.d.). *Control the concurrency of workflows and jobs*. GitHub Docs. https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency

GitHub. (n.d.). *Events that trigger workflows*. GitHub Docs. https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows

GitHub. (n.d.). *Contexts reference*. GitHub Docs. https://docs.github.com/en/actions/reference/workflows-and-actions/contexts

GitHub. (n.d.). *Re-running workflows and jobs*. GitHub Docs. https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs

Free Software Foundation. (n.d.). *Bash reference manual*. GNU Project. https://www.gnu.org/software/bash/manual/bash.html

Free Software Foundation. (2026). *Common options (GNU Coreutils 9.11)*. GNU Coreutils manual. https://www.gnu.org/software/coreutils/manual/html_node/Common-options.html

Free Software Foundation. (2026). *env invocation (GNU Coreutils 9.11)*. GNU Coreutils manual. https://www.gnu.org/software/coreutils/manual/html_node/env-invocation.html

Open Container Initiative. (2025, November 4). *OCI runtime-spec v1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Rust Project Developers. (n.d.). *Environment variables*. The Cargo Book. https://doc.rust-lang.org/cargo/reference/environment-variables.html

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/

Rust Secure Code Working Group. (2024, November 10). *RUSTSEC-2024-0388: derivative—`derivative` is unmaintained; consider using an alternative*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2024-0388.html

Rust Secure Code Working Group. (2026, August 11). *RUSTSEC-2026-0253: lru—memory safety issue under panic*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2026-0253.html
