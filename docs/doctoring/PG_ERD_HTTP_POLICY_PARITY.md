# pg-erd-cloud HTTP Policy parity evidence

This note is evidence for characterization only. It is not deployment, canary, cutover, release, or Enterprise Architecture admission evidence.

## Live consumer source

- Repository: `ContextualWisdomLab/pg-erd-cloud`
- Default-branch source commit observed for this slice: `8dc746920c12988f082e914879d95e13c9693535`
- Edge config: `deploy/traefik/dynamic.yaml`
- Exact file blob: `656d18fdfedb19b2556312db4102740044531719`

The Traefik `security-headers` middleware is attached to the exact `/healthz` router, raw-prefix `/api` router, and frontend fallback router. It configures `contentTypeNosniff: true`, `customFrameOptionsValue: DENY`, `referrerPolicy: no-referrer`, and `permissionsPolicy: geolocation=(), microphone=(), camera=()`.

The executable parity contract maps these to exact HTTP response values: `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and `Permissions-Policy: geolocation=(), microphone=(), camera=()`.

## Standards boundary

RFC 9110 defines HTTP field names as case-insensitive and field names using the HTTP token grammar. The candidate policy therefore compares names ASCII case-insensitively. Its accepted field-name syntax is intentionally a stricter alphanumeric/hyphen subset because the current migration evidence does not require the full token character set. Broadening the policy without a consumer contract is not necessary for parity.

Configured values reject CR/LF before activation. This is a local fail-closed configuration invariant; it does not claim that the transport-neutral model is already connected to Pingora's response callbacks.

## Executable source trail

- RED characterization: `f0e32d3630e676c494995e9c4cc94082372a3287`
- Initial GREEN policy: `4182bb629749e9c2acc3e9d575e7598aebfa7e66`
- Public module exposure: `a0af9544850ed578248f069966295d77207c21e1`
- Validation-profile repair: `f48fd423a37f1c6a579375da9e7471d2543f588e`
- Coverage-contract expansion: `acd0e4adeb8916dad6d853cf71e06ce983da1bb4`

Every subsequent head must reacquire exact-head CI, owned-production 100% line/region coverage, public rustdoc and applicable supply-chain/security/review evidence. Predecessor checks do not transfer.

## Authority exclusions

This evidence does not move product authentication, authorization, business response semantics, Wardnet/EgressWeave security verdicts, Keyverse identity, certificate lifecycle, runtime request/log/customer data, or Context Graph/EA authority into `pingora-gateway`.

## Reference

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). RFC Editor. https://www.rfc-editor.org/rfc/rfc9110
