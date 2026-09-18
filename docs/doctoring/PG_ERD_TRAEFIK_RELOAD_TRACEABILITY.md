# pg-erd Traefik reload characterization traceability

Status: Proposed characterization evidence. This document is scoped to issue #109 and is not repository-wide `TRACEABILITY.md` or `docs/product-technical-gap-baseline.md` authority.

## Decision boundary

`ContextualWisdomLab/pg-erd-cloud@8dc746920c12988f082e914879d95e13c9693535` is the consumer evidence identity for this characterization. Its production-style Compose pins `traefik:v3.5.4@sha256:4df0a50fcf71b454c0d7ad17675776dc8d37359deae3291895bdaa008c1b9972`, configures `--providers.file.filename=/etc/traefik/dynamic.yaml` with `--providers.file.watch=true`, and mounts `./deploy/traefik/dynamic.yaml:/etc/traefik/dynamic.yaml:ro` as a single-file bind mount.

The checked-in dynamic file owns the consumer's current `/healthz`, `/api`, fallback `/`, backend/frontend peer, and response-header choices. Those product route choices are evidence inputs only: product route policy remains consumer-owned. This repository may own reusable edge transport/configuration mechanics but must not turn pg-erd route policy into a shared gateway business language.

No Pingora hot-reload implementation is selected by the presence of `watch=true`. The first question is whether runtime reload is supported as an operator contract at all. If the pg-erd-cloud owner determines that it is not supported, the acceptable migration disposition is a controlled restart/redeployment contract with readiness, drain, rollback, and exact release identity. If it is supported, issue #109 requires a bounded Admin Config contract before implementation.

## Live repository evidence

At the source identity above:

- `compose.prod.yaml` contains the exact file-provider `filename` + `watch` configuration and single-file read-only bind mount.
- `deploy/traefik/dynamic.yaml` has one protected-history introduction commit, `28bed7a871c5e7b47249793f7192a8523d404f2c` (`Replace production nginx edge with Traefik`, 2026-06-20), and no later protected-history change was returned for that path through 2026-09-18.
- repository search finds `providers.file.watch` only in `compose.prod.yaml`.
- the documented production operation is `docker compose -f compose.prod.yaml up -d --build`; no checked-in runtime file-edit procedure was found.
- `CLAUDE.md` says the production Compose has “No bind mounts or reload” while the exact Compose does bind-mount `dynamic.yaml` and enables file watching. That wording is therefore insufficient as operational authority and is left for the pg-erd-cloud owner to reconcile.

This evidence makes an accidentally enabled or unused watcher plausible, but it does not prove that operators never edit the file outside Git history. Owner disposition remains required; this lane does not edit pg-erd-cloud source, docs, refs, or PR state.

## Primary-source constraint

Traefik's File provider documentation states that `providers.file.watch=true` watches for filesystem changes and that `filename` and `directory` are mutually exclusive. It recommends `directory`. The same documentation warns that orchestrator bind mounts can miss filesystem notifications when the host file is renamed/replaced; for Docker specifically, renaming a host file can break the link to an individually mounted file, so Traefik recommends binding the parent directory instead.

Docker documents bind mounts as host paths mounted into a container and describes read-only bind mounts and their host coupling. Together these sources make the current consumer's atomic rename/replace behavior an executable question rather than an assumption.

## Executable characterization

`.github/workflows/pg-erd-traefik-reload-characterization.yml` is manual-only and protected-main-only. It pins the pg-erd-cloud source SHA above, checks out both source identities without persisted credentials, and executes `tests/load/characterize_pg_erd_traefik_reload.sh` against an ephemeral checkout. The consumer repository is never pushed or otherwise mutated.

The harness uses the exact production-style Compose and verifies the exact Traefik image, `filename`, `watch`, and single-file bind-mount shape before startup. It then records:

- baseline `/healthz` availability;
- whether an in-place write is observed and its observed reload latency;
- whether malformed YAML preserves the prior last-known-good generation;
- whether a valid generation recovers after malformed input;
- the observed status/generation after a semantically invalid service reference;
- whether atomic rename/replace is observed without container recreation;
- whether controlled `--force-recreate traefik` consumes the replaced inode;
- concurrent probe sample/failure counts across the transition sequence;
- source/config/log/probe evidence digests.

Fresh evidence-integrity review found that the first harness captured `docker compose logs traefik` only after the transition sequence and wrote with `>` rather than append. Every controlled `--force-recreate traefik` can destroy the outgoing container whose log contains the file-provider parse/reload event that caused the fallback, so a GREEN characterization could retain only the final container's logs and lose the causal evidence for malformed, recovery, or atomic-replace observations. Source-level RED `2efce0a5bf63349809ee5c915c00e147f5fa60d2` requires append-only snapshots and a capture immediately before recreation. Causal repair `fb315ea666b77985ab591564dcf657967a1dff11` appends phase/timestamp/container-identity snapshots, captures the outgoing Traefik log before every force recreation, captures the replacement container afterward, and retains the final/cleanup snapshots. Observation semantics, consumer source, Traefik configuration, restart decision boundary, and product authority are unchanged.

A second evidence-integrity review found that the receipt computed `traefik_log_sha256` before process exit while the EXIT cleanup trap appended one more `cleanup` log snapshot afterward. The uploaded log could therefore be byte-different from the digest recorded in the receipt even on a successful run. Source-level RED `95e2413cbf52b5c9128c51daba4213e383eac0c8` freezes the invariant that the cleanup snapshot must precede the one final log digest and that no later snapshot may mutate the artifact. Causal repair `a10a76a29bbaa1592c70a813a3e2cd37cf82672a` moves digest finalization into cleanup after the last snapshot; `result=characterization-complete` is also emitted there only when the script's preserved exit status is zero. The uploaded log digest now describes the final artifact bytes rather than a pre-cleanup prefix.

The generated `X-CWL-Reload-Generation` header exists only in the ephemeral checked-out fixture and is a transport-neutral observation marker. It is not added to pg-erd-cloud or to the shared gateway API.

The first harness intentionally does not claim long-lived WebSocket/H2 stream generation semantics or a production Pingora reload design. If the pg-erd-cloud owner confirms that live mutation is supported, a successor RED must add in-flight/long-lived connection generation behavior before issue #109 can close. If the owner selects controlled deployment/restart, those live-reload semantics become non-requirements and restart/drain/rollback acceptance becomes authoritative instead.

## Privacy and evidence handling

Only `/healthz` status and the test-only generation header are sampled. Evidence must not capture an `Authorization` header, `Cookie`, request body, DSN, APP secret, user/project identity, schema payload, or other product data. The application secret and database password created by the harness are disposable local fixture values, are not printed, and are removed during cleanup. Traefik logging is retained only for the isolated characterization runtime.

A GREEN workflow means the characterization executed and produced source-bound evidence; it does not mean every observed behavior is desirable. The receipt fields such as `in_place_reload_detected`, `malformed_last_known_good`, and `atomic_replace_detected` are observations that feed the owner decision. Recovery to a valid generation and controlled recreation consuming the replaced path are fixture-integrity requirements.

## Promotion boundary

This lane is a Draft child of the stale #12 Admin Config stack and cannot transfer evidence across parent reconciliation. The manual workflow cannot run as promotion evidence until its definition is integrated on protected `main`. Issue #109 remains open until the pg-erd-cloud owner records whether runtime reload is supported and the selected disposition is proved through the appropriate release-qualified consumer path.

No mutable consumer branch, product-source copy, provider-specific business routing, self-approval, security suppression, immutable-release claim, canary/cutover claim, or legacy-Traefik removal follows from this characterization.

## References

Docker, Inc. (2026). *Bind mounts*. Docker Docs. https://docs.docker.com/engine/storage/bind-mounts/

Traefik Labs. (n.d.). *File provider*. Traefik Proxy documentation. Retrieved September 18, 2026, from https://doc.traefik.io/traefik/providers/file/

ContextualWisdomLab. (2026a). *pg-erd-cloud: compose.prod.yaml* (commit `8dc746920c12988f082e914879d95e13c9693535`). GitHub.

ContextualWisdomLab. (2026b). *pg-erd-cloud: deploy/traefik/dynamic.yaml* (commit `8dc746920c12988f082e914879d95e13c9693535`). GitHub.
