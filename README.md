# Pingora Gateway

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ContextualWisdomLab/pingora-gateway)

**Shared edge-runtime boundary for ContextualWisdomLab-managed reverse-proxy traffic.**

Pingora Gateway is the organization-owned home for a Rust edge runtime built on Cloudflare Pingora. Consumer products should integrate through versioned edge configuration and contracts owned here rather than depending directly on Pingora implementation types.

## Current status

The protected `main` branch is currently a repository scaffold: it does not yet contain the production runtime, a published release, or a deployable edge configuration. This README describes the intended repository responsibility without promoting future implementation to shipped capability.

## Responsibility

This repository is intended to own the shared reverse-proxy edge boundary, including the runtime and versioned configuration needed to apply common edge behavior consistently across ContextualWisdomLab products. Product-specific business policy remains with the consuming product; upstream Pingora remains authoritative for its own framework behavior.

## Integration model

The architectural direction is deliberately contract-first:

```text
consumer product
      |
      v
versioned edge configuration / contract
      |
      v
Pingora Gateway
      |
      v
Cloudflare Pingora
```

Consumers should not need to import or reason about Pingora-specific types merely to request organization-standard edge behavior.

## Documentation

- [Public documentation landing](docs/index.md)
- [Repository releases](https://github.com/ContextualWisdomLab/pingora-gateway/releases)
- [Ask DeepWiki](https://deepwiki.com/ContextualWisdomLab/pingora-gateway)

Architecture, onboarding, release, and operational documents should be added here as implementation lands and reviewed evidence exists.

## Release and licensing truth

There is currently no published GitHub release and no root source license file on protected `main`. Do not infer production readiness, redistribution terms, customer deployment, or support status from the repository name or scaffold alone. Those claims require explicit source, license, release, and deployment evidence.
