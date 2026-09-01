# Repository working agreement

Read `AGENTS.md`, `ARCHITECTURE.md`, `docs/CONTEXT_MAP.md`, `docs/UBIQUITOUS_LANGUAGE.md`, `docs/SECURITY.md` and `docs/product-technical-gap-baseline.md` before editing. Domain logic must remain independent of Pingora transport types. Prefer deterministic tests through the real gateway process. Treat configuration, headers, upstream addresses and requests as untrusted. Do not add product-specific policy to this shared runtime. Update CHANGELOG and traceability when a contract changes.
