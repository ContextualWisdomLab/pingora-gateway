# Product / Technical Gap Baseline

Updated: 2026-09-01.

## Shared runtime

Bootstrap branch contains a real Rust/Pingora production path, versioned fail-closed configuration, DDD boundary, HTTP/HTTPS upstreams, health/metrics, bounded requests/timeouts, TLS verification, forwarded-header distrust, non-root OCI packaging and local integration fixtures. It is **not migrated/released** until protected `main` contains the implementation and a real immutable image digest is published.

Current upstream basis: Pingora latest release observed is 0.8.1 (2026-06-04). Critical request-smuggling/cache issues affecting <=0.7.0 are fixed there. Upstream PR #977 (`09696b51bc59315353d96686355861604d0bb48c`, merged 2026-08) subsequently moved Pingora's in-tree `lru` to 0.18.2 after RUSTSEC-2026-0253; this bootstrap pins that exact commit. Revalidate before release.

Open shared-runtime gaps: compilation/CI evidence pending; no committed Cargo.lock; no coverage threshold/branch evidence; no fuzz/concurrency/drain recovery test; Docker base tags are not digest-pinned; no signed SBOM/provenance handoff; no published OCI digest; readiness does not probe upstreams; trusted-proxy forwarding is intentionally unsupported; static/WebSocket/TLS termination/Gateway API remain separate future increments.

## Organization inventory — first live sweep

Actionable evidence includes `scopeweave/Dockerfile` plus `infra/nginx/default.conf` (active static runtime), `LineageWeave/frontend/Dockerfile` plus `frontend/nginx.conf` (active frontend runtime), `inkspan/Dockerfile` (active Nginx static runtime), `naruon` live-E2E/operator material referencing Nginx, and `linux-cluster-ops` scripts/runbooks/inventory for operating Nginx and certificate backup. Repositories with enabled dedicated writers are read-only from this loop; their owner paths need the same characterization/RED -> Pingora or correct managed-owner replacement -> GREEN evidence. `scopeweave` currently has no more-specific enabled dedicated writer and is a candidate consumer after the shared runtime is gate-clean.

Do not delete historical references or negative policy fixtures. Certbot/certificate issuance must be separated from the edge runtime rather than moved into Pingora by default.
