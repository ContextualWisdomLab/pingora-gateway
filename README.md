# Pingora Gateway

**Shared Rust edge-gateway boundary for high-throughput ContextualWisdomLab services.**

Pingora Gateway is the dedicated repository for a reusable edge runtime built around Cloudflare's Pingora ecosystem. Its purpose is to centralize gateway responsibilities that should not be reimplemented independently by every product: connection handling, upstream routing boundaries, transport policy, observability hooks, and other edge concerns that belong below product-specific application logic.

## What this repository owns

This repository owns the **shared edge-gateway runtime boundary** for ContextualWisdomLab products that need a Rust gateway based on Pingora.

Product authorization, business-domain policy, identity truth, model orchestration, and application data remain with their owning services. A shared gateway may enforce transport and edge policy, but it must not become a hidden monolith that absorbs those product responsibilities.

## Why it exists

A common gateway can reduce duplicated infrastructure while preserving independent product deployment. The intended value is a reviewed place to evolve capabilities such as:

- HTTP/TLS connection and upstream lifecycle management;
- bounded routing and destination policy;
- reusable timeout, resource, and failure behavior;
- edge observability and operational evidence;
- stable integration contracts for product services.

Those are target responsibilities, not claims that the current repository already implements them.

## Current status

This repository is currently a **governance-only bootstrap**. Protected `main` contains no Rust crate, Pingora dependency, executable gateway, configuration schema, deployment artifact, benchmark, release, or published integration API.

There is therefore nothing to install or run yet. Consumers should not treat the repository name or this product boundary as evidence of a deployed gateway.

## Planned integration boundary

```text
Client / service traffic
          │
          ▼
┌───────────────────────────┐
│      Pingora Gateway      │
│ shared Rust edge boundary │
└─────────────┬─────────────┘
              │
       reviewed upstream
        service contract
              │
      ┌───────┼────────┐
      ▼       ▼        ▼
   product  product   shared
   service  service   service
```

When implementation begins, gateway APIs and policy must be explicit and versioned. Product services remain independently authoritative for their own business and security decisions.

## Pingora dependency boundary

Cloudflare Pingora is separately licensed under the Apache License 2.0. The current Pingora Gateway repository does **not** contain copied Pingora source or a dependency manifest yet. If Pingora code or packages are introduced later, the change must preserve the applicable upstream license and attribution obligations rather than treating this repository's license as a substitute for third-party provenance.

## Quality and governance

Because the current tree is only a bootstrap, it does not claim performance numbers, production traffic, security certification, availability targets, test coverage, or deployment readiness.

Future runtime work should add its build/test/security/operability evidence together with the implementation so this README can describe measured current behavior instead of planned capability.

## Contributing

Keep edge-runtime concerns here and product-specific policy in its owning repository. New third-party software must be commercially usable under the intended distribution model and must retain its required notices and provenance.

Substantive changes should arrive through reviewed pull requests with the architecture, security, tests, and integration documentation needed to make the new boundary understandable to downstream services.

## License

ContextualWisdomLab's original work in this repository is licensed under the [Apache License 2.0](LICENSE). Third-party components retain their own applicable license and attribution terms.
