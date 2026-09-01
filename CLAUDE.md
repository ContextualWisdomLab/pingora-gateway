# Repository Working Notes

Use `AGENTS.md` as the engineering contract. The central design rule is that admitted edge contracts own domain invariants while Pingora remains a delivery adapter. `edge_routing` and `http_policy` may contain executable consumer-derived characterization only when the responsibility is genuinely shared edge behavior; they do not own product authorization, business routing, domain response semantics, Wardnet/EgressWeave verdicts, or Keyverse identity.

Before a material change, compare the proposed module/path/API/config name with `PRD.md`, `TRD.md`, `ARCHITECTURE.md`, `CONTEXT_MAP.md`, and `UBIQUITOUS_LANGUAGE.md`. After the change, update tests and the gap baseline. Do not claim release, migration, performance, SBOM/provenance, graceful-drain verification, or image digest evidence unless the exact protected revision proves it.

Current active v1 intentionally supports one upstream per process. The `pg-erd-cloud` route table and response-security headers are characterized but not active. Adding multiple upstreams, route-table activation, response-policy activation, load balancing, WebSocket behavior, downstream TLS, static serving, dynamic reload, or Kubernetes Gateway API is a versioned product increment rather than an opportunistic helper.
