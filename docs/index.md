---
title: Pingora Gateway
---

# Pingora Gateway

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ContextualWisdomLab/pingora-gateway)

**Pingora Gateway is the shared Rust edge-runtime boundary for ContextualWisdomLab services that need explicit, bounded reverse-proxy behavior without moving product authority into infrastructure.**

## Current status

Protected `main` remains shipped authority. The active foundation candidate implements the first executable v1 runtime, but it is still Draft and is not a published release or production cutover.

The current candidate owns one explicit upstream, bounded transport and request budgets, fail-fast in-flight admission, forwarding-header distrust, `/livez` and `/readyz`, low-cardinality metrics, coarse credential-safe logging, upstream TLS verification, graceful drain, OCI hardening, and locked supply-chain evidence. Product authentication, business routing, certificate issuance, identity, workflow state, and domain-specific retry/failover remain external responsibilities.

## Start here

- [README](https://github.com/ContextualWisdomLab/pingora-gateway#readme) — product value, quickstart, boundaries, quality and source-license status.
- [Architecture](https://github.com/ContextualWisdomLab/pingora-gateway/blob/main/ARCHITECTURE.md) — edge-runtime structure and responsibility boundary after protected integration.
- [Configuration contract](https://github.com/ContextualWisdomLab/pingora-gateway/blob/main/API_CONFIG_CONTRACT.md) — versioned public configuration surface after protected integration.
- [Security](https://github.com/ContextualWisdomLab/pingora-gateway/blob/main/SECURITY.md) — security reporting and runtime security boundary after protected integration.
- [Releases](https://github.com/ContextualWisdomLab/pingora-gateway/releases) — immutable release evidence when one is published.
- [Ask DeepWiki](https://deepwiki.com/ContextualWisdomLab/pingora-gateway) — repository-grounded navigation and questions.

Links to candidate-only files may not resolve from `main` until the foundation integrates; the pull request remains the review authority for those bytes meanwhile.

## License and dependency boundary

Pingora Gateway original source and documentation are Apache-2.0 on the foundation candidate. Cloudflare Pingora is separately Apache-2.0 licensed, and all third-party crates, container bases, copied material, data, and assets retain their own licenses and attribution obligations. The repository license does not replace dependency provenance.

Current security/supply-chain findings are not waived by the permissive source license. The foundation remains blocked wherever exact-head security or dependency policy fails.

## Publication truth

This file is a GitHub Pages source prerequisite only. Source presence, a green documentation check, or an open pull request is not evidence that Pages is published. Publication is complete only after protected integration, repository-settings reconciliation, successful deployment, and live HTTPS verification.
