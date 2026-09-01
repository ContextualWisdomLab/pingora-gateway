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

## Release truth

There is currently no published GitHub release or production deployment evidence. Do not infer production readiness, customer deployment, or support status from the repository name or scaffold alone; those claims require protected source, immutable release artifacts, deployment evidence, and live verification.

## License

Pingora Gateway original source and documentation are licensed under the [Apache License 2.0](LICENSE) on this branch. The repository history is an organization-owned initialization lineage with no imported runtime source, vendored code, package manifest, submodule, or third-party asset in protected `main` that imposes a conflicting outbound source license.

Cloudflare Pingora is an upstream dependency/framework with its own Apache-2.0 license and remains separately attributed under its upstream terms. Future crates, copied code, assets, datasets, rules, or other inbound components must independently satisfy ContextualWisdomLab's commercial-use and attribution policy before incorporation; their licenses are not replaced by this repository's source grant.
