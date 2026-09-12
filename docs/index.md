---
title: Pingora Gateway
---

# Pingora Gateway

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ContextualWisdomLab/pingora-gateway)

**Pingora Gateway is the shared Rust edge-runtime boundary for ContextualWisdomLab services that need explicit, bounded reverse-proxy behavior without moving product authority into infrastructure.**

## Current status

Protected `main` remains shipped authority. The active dependency-ordered candidate stack contains the generic Rust/Pingora runtime plus later TLS/H2, migration-characterization, supply-chain and protected-release-evidence increments, but it is not yet protected-integrated, published, deployed or cut over.

The current candidate owns one explicit generic upstream per process, bounded transport and request budgets, fail-fast in-flight admission, forwarding-header distrust, `/livez` and `/readyz`, low-cardinality metrics, coarse credential-safe logging, verified upstream TLS, opt-in downstream TLS/H2 for the admitted configuration versions, graceful drain, OCI hardening, and locked supply-chain/release-evidence controls. Product authentication, business routing, certificate issuance/lifecycle, identity, workflow state, Wardnet/EgressWeave verdicts, Keyverse authority, and domain-specific retry/failover remain external responsibilities.

The branch dependency contract uses exact registry `pingora = 0.9.0` and `pingora-prometheus = 0.9.0`. That candidate graph still carries the separately tracked `derivative 2.2.0 / RUSTSEC-2024-0388` supplier root, so current successor GREEN evidence is not a release-ready dependency claim. H2-downstream to H1-upstream parity also remains gated by release-qualified supplier disposition of the tracked Cookie and zero-length body-framing roots; mutable upstream contributor branches are evidence only.

## Start here

- [README](https://github.com/ContextualWisdomLab/pingora-gateway#readme) — product value, quickstart, boundaries, quality and source-license status.
- [Architecture](https://github.com/ContextualWisdomLab/pingora-gateway/blob/main/ARCHITECTURE.md) — edge-runtime structure and responsibility boundary after protected integration.
- [Configuration contract](https://github.com/ContextualWisdomLab/pingora-gateway/blob/main/API_CONFIG_CONTRACT.md) — versioned public configuration surface after protected integration.
- [Security](https://github.com/ContextualWisdomLab/pingora-gateway/blob/main/SECURITY.md) — security reporting and runtime security boundary after protected integration.
- [Releases](https://github.com/ContextualWisdomLab/pingora-gateway/releases) — immutable release evidence when one is published.
- [Ask DeepWiki](https://deepwiki.com/ContextualWisdomLab/pingora-gateway) — repository-grounded navigation and questions.

Links to candidate-only files may not resolve from `main` until the dependency-ordered stack integrates; the pull requests remain the review authority for those bytes meanwhile.

## License and dependency boundary

Pingora Gateway original source and documentation are Apache-2.0 on the candidate stack. Cloudflare Pingora is separately Apache-2.0 licensed, and all third-party crates, container bases, copied material, data, and assets retain their own licenses and attribution obligations. The repository license does not replace dependency provenance.

Current security/supply-chain findings are not waived by the permissive source license. Promotion remains blocked wherever exact-head security, dependency, independent-review, protected-integration or release policy is unsatisfied.

## Publication truth

This file is a GitHub Pages source prerequisite only. Source presence, a green documentation check, or an open pull request is not evidence that Pages is published. Publication is complete only after protected integration, repository-settings reconciliation, successful deployment, and live HTTPS verification.
