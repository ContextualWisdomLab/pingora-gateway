# Changelog

## Unreleased

- Bootstrap versioned fail-closed configuration, deterministic Edge Routing domain, Pingora HTTP/HTTPS proxy adapter, liveness/readiness/metrics endpoints, bounded requests/timeouts, verified upstream TLS, non-root OCI packaging and production-path integration fixtures.
- Pin Pingora to upstream commit `09696b51bc59315353d96686355861604d0bb48c` because the latest release (0.8.1, 2026-06-04) predates the upstream 2026-08 dependency-security sync that moved Pingora's in-tree `lru` use to 0.18.2.
