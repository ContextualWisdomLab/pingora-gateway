# ADR 0005: Characterize pg-erd-cloud response security headers before Pingora activation

- Status: Accepted for characterization; runtime activation remains blocked
- Date: 2026-09-02

## Context

The live `ContextualWisdomLab/pg-erd-cloud` default branch carries a production-style Traefik edge configuration at source commit `8dc746920c12988f082e914879d95e13c9693535`. `deploy/traefik/dynamic.yaml` has blob `656d18fdfedb19b2556312db4102740044531719` and attaches one `security-headers` middleware to the `/healthz`, `/api` and frontend routers.

The middleware emits these edge-owned response fields:

| Traefik setting | HTTP response field | Exact value |
| --- | --- | --- |
| `contentTypeNosniff: true` | `X-Content-Type-Options` | `nosniff` |
| `customFrameOptionsValue: DENY` | `X-Frame-Options` | `DENY` |
| `referrerPolicy: no-referrer` | `Referrer-Policy` | `no-referrer` |
| `permissionsPolicy: geolocation=(), microphone=(), camera=()` | `Permissions-Policy` | `geolocation=(), microphone=(), camera=()` |

PR #5 already characterizes route precedence in the Edge Routing bounded context. Folding response mutation into that context would conflate two independently testable responsibilities. Product authentication, authorization and business response semantics also remain outside this runtime, as do Wardnet/EgressWeave security verdicts and Keyverse identity.

## Decision

Introduce a transport-neutral HTTP Policy bounded context that can characterize explicit edge-owned response headers before any Pingora callback activates them.

- Header names are compared ASCII case-insensitively.
- A characterized field name may occur only once; duplicate authority fails closed.
- The current migration profile accepts only non-empty alphanumeric/hyphen field names. This deliberately narrow subset covers the captured consumer contract; broader legal HTTP field-name syntax is not added without evidence that a migrated consumer needs it.
- Empty response values fail closed for this explicit policy profile.
- CR or LF in configured values fails closed to prevent response-splitting/header-injection semantics.
- Values are otherwise preserved exactly; the gateway does not normalize product policy into a different value.

The policy remains independent of Pingora types and is not wired into `GatewayConfig` or `GatewayProxy` in this slice. Runtime activation must follow separate exact-head evidence and versioned configuration/runtime work.

## Test-first evidence

RED commit `f0e32d3630e676c494995e9c4cc94082372a3287` added the executable `pg-erd-cloud` contract before `http_policy` existed. GREEN implementation begins at `4182bb629749e9c2acc3e9d575e7598aebfa7e66`; public bounded-context exposure follows at `a0af9544850ed578248f069966295d77207c21e1`. Validation/profile repair `f48fd423a37f1c6a579375da9e7471d2543f588e` removes syntax ambiguity and makes the deliberately narrow header-name profile explicit. Coverage-focused contract expansion at `acd0e4adeb8916dad6d853cf71e06ce983da1bb4` exercises absent lookup, non-empty collection semantics, empty/invalid names, duplicate case variants, empty values and both CR/LF injection paths.

Every later source or documentation head must reacquire the repository's 100% owned production line/region coverage, public rustdoc and then-applicable CI/supply-chain/security/review evidence. Predecessor results do not transfer.

## Consequences

This closes one characterization gap only. It does not make multi-upstream Pingora routing active, does not mutate consumer application responses yet, and does not prove shadow, canary, cutover or rollback. The next runtime increment may bind a validated HTTP policy to explicitly selected routes only after the parent route contract and release/security dependencies are coherent. Product-domain authorization/business logic and canonical security/identity owners remain separate bounded contexts.
