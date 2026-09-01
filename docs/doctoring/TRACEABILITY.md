# Doctoring / Traceability

Evidence reviewed 2026-09-01: Cloudflare Pingora release 0.8.1 (2026-06-04) states bounded default HTTP/2 server limits and rustls-related dependency updates. Pingora 0.8.0 release notes document request-framing, CONNECT default and range/304/416 hardening. Cloudflare Pingora PR #977, merge commit `09696b51bc59315353d96686355861604d0bb48c`, contains the 2026-08 dependency/security sync including `lru = 0.18.2`; PR #962 was closed in favor of #977.

Primary standards for consumer characterization: Fielding et al. (2022), *HTTP Semantics*, RFC 9110; Nottingham (2022), *HTTP Caching*, RFC 9111; Thomson & Benfield (2022), *HTTP/2*, RFC 9113. Kubernetes Gateway API or downstream TLS standards will be added only when those increments enter scope.

APA 7 web/software traceability: Cloudflare. (2026). *Pingora* [Computer software]. GitHub. Exact source revision is recorded in Cargo.toml and ADR 0001. Internet Engineering Task Force RFCs are cited by author/editor, year, title and RFC number above. This file records evidence provenance; it does not claim certification.
