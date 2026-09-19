# Repository Working Notes

Use `AGENTS.md` as the engineering contract. The central design rule is that the versioned edge contract owns domain invariants while Pingora remains a delivery adapter. Never place product-specific routing or policy in this repository.

Before a material change, compare the proposed module/path/API/config name with `PRD.md`, `TRD.md`, `ARCHITECTURE.md`, `CONTEXT_MAP.md`, and `UBIQUITOUS_LANGUAGE.md`. After the change, update tests and the gap baseline. Do not claim release, migration, performance, SBOM/provenance, graceful-drain verification, or image digest evidence unless the exact protected revision proves it.

Current v1 intentionally supports one upstream per process. Adding multiple upstreams, route tables, load balancing, WebSocket behavior, downstream TLS, static serving, dynamic reload, or Kubernetes Gateway API is a versioned product increment rather than an opportunistic helper.
