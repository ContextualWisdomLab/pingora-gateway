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

Docker documents bind mounts as host paths mounted into a container and describes read-only bind mounts and their host coupling. Docker Compose also specifies that when a published host port is omitted the container runtime allocates an unassigned host port, and `--project-directory` can preserve the consumer project base when a generated Compose file is used. These contracts let the characterization isolate host-port allocation from the edge semantics without a bind-then-release race.

## Executable characterization

`.github/workflows/pg-erd-traefik-reload-characterization.yml` is manual-only and protected-main-only. It pins the pg-erd-cloud source SHA above, checks out both source identities without persisted credentials, and executes `tests/load/characterize_pg_erd_traefik_reload.sh` against an ephemeral checkout. The consumer repository is never pushed or otherwise mutated.

The harness verifies the exact production-style Compose image, `filename`, `watch`, and single-file bind-mount shape before startup. For CI-only host publication it derives a temporary runtime Compose file from that pinned source, changing only the two loopback host-port mappings to Docker-managed ephemeral ports; relative build, env, secret, and bind-mount paths continue to resolve from the exact consumer project directory. The generated file is runner-local, its digest is recorded, and the consumer's tracked Compose is left byte-identical. It then records:

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

Current-range review also identified a timing false-GREEN in the invalid-input observations: malformed YAML and the semantically invalid service reference were each followed by a fixed two-second sleep and one `/healthz` sample. A slow file-provider callback could leave the prior generation visible at that instant and be misclassified as the settled outcome. Source-level RED `1f5424d5f600fdc51eaf57f781dd42705053e5eb` rejects fixed-delay classification and requires bounded transition windows plus explicit observation duration. Causal repair `e784afc2c497f9e828e90d7c8ea8775706a3b82e` waits up to 15 seconds for three consecutive non-baseline observations, records the actual malformed and semantic-invalid observation windows, and gives the semantic-invalid candidate its own generation marker before replacing the health-route service. If no stable external transition appears in that window, the receipt records the baseline generation as the bounded black-box observation; it does not claim that Traefik's parser callback definitely ran. The subsequent valid recovery still has to be observed or controlled recreation is recorded.

The subsequent exact-range review found a separate stale-generation false-GREEN in that recovery step. After the semantic-invalid candidate, the harness rewrote the same earlier `recovery` generation and immediately waited for generation `recovery`. If the invalid candidate never became externally visible, the still-visible pre-invalid `recovery` marker could satisfy that predicate without proving any new valid configuration was consumed. Source-level RED `bc98ca4bf1d51c2bbb15fc402fc3bbedb973a9bc` requires a distinct post-invalid recovery marker. Causal repair `a64808ae79e2f4e39b850559485c846bd6cb3c7c` renders `post-invalid-recovery` into a new candidate and records `post_invalid_recovery_requires_recreate=false` only when that new marker is observed; otherwise the existing controlled recreation path is taken and the same fresh marker must then appear. The repair changes only characterization evidence integrity, not consumer configuration or gateway runtime semantics.

A further harness review found a non-deterministic startup RED unrelated to Traefik semantics. The harness selected two ephemeral host ports by binding `127.0.0.1:0`, reading the assigned numbers, closing those sockets, and only later starting Compose. Another process could claim either released port in that interval, turning a valid characterization into an environmental bind failure. Source-level RED `9d02664f04a013c18096109cd33b76ce005671d3` rejects this bind-then-close TOCTOU. Causal repair `7f99b9ba3547aac137813ac5da284c9d81c8a79b` derives a runner-local Compose file whose two host publications omit the host port while retaining loopback binding, lets Docker allocate the unassigned ports when containers are created, discovers Traefik's actual mapping with `docker compose port`, and refreshes an atomic endpoint file after every controlled Traefik recreation so the concurrent probe follows the current container mapping. Contract follow-up `9d5516004fcc3cfa6ff4cdcd0a2dbcf94cf6f09b` binds the executable checks to that runtime endpoint model. This changes only test-harness host publication; the pinned consumer Compose, product routes, file-provider shape, and gateway runtime remain unchanged.

The always-run artifact upload was then found to rely on the GitHub artifact name and surrounding run metadata for the gateway source identity. If characterization fails after exact checkout verification but before the harness initializes its receipt/log files, the evidence package could lack an internal statement of which gateway and consumer source pair was actually verified; renaming or exporting the artifact would sever that source binding. Source-level RED `fb9a57ec881dac26d0f129f786f389811eb26c05` requires the uploaded set to carry its own exact gateway and consumer source marker before characterization starts. Causal repair `00dd41c81274095b1b631b5060a503b01d2ad079` writes `pg-erd-traefik-source-identity.txt` immediately after both exact checkout verifications, records `gateway_source_sha=$EXPECTED_SHA` and `consumer_source_sha=$PG_ERD_SOURCE_SHA`, and includes that marker in the always-run artifact. This does not change Traefik behavior, the consumer checkout, characterization traffic, or Admin Config semantics; it makes post-verification failures source-attributable even when the harness exits before producing its normal receipt.

A follow-up evidence-set review found that the source marker and the normal characterization receipt were still independent sibling files. The receipt carried only the consumer SHA, so an exported or accidentally mixed artifact set could pair a valid receipt/log/probe set with a different gateway source marker without an internal cross-link. The same review also noted that `if-no-files-found: error` rejects an entirely empty upload but does not prove that every required file exists. Source-level RED `f613b8daa8b10099063645c930f90636690508cd`, refined without weakening the invariant by `dd1aa387cd323577a0e3cf988306f4cc0e77571c`, requires the evidence boundary to verify the marker, receipt, Traefik log, and concurrent-probe log individually and to bind the receipt to the exact marker before upload. Causal workflow repair `08d43d4d623060781515fd9239af172e433fe933` runs that check under `always()`, verifies the marker contains the exact gateway and consumer SHAs, hashes the marker, and appends `gateway_source_sha`, `consumer_source_sha`, and `source_identity_sha256` to the receipt before the artifact step. Contract alignment `60cfb356e018dfc8f1df36687581368e07a61703` reflects the actual `printf` implementation. Missing individual evidence now keeps the run fail-closed even when another artifact file exists, while successful detached evidence can prove which source marker its receipt consumed. No Traefik behavior, consumer source, route/header policy, restart choice, or gateway Admin Config semantics changed.

The generated `X-CWL-Reload-Generation` header exists only in the ephemeral checked-out fixture and is a transport-neutral observation marker. It is not added to pg-erd-cloud or to the shared gateway API.

The first harness intentionally does not claim long-lived WebSocket/H2 stream generation semantics or a production Pingora reload design. If the pg-erd-cloud owner confirms that live mutation is supported, a successor RED must add in-flight/long-lived connection generation behavior before issue #109 can close. If the owner selects controlled deployment/restart, those live-reload semantics become non-requirements and restart/drain/rollback acceptance becomes authoritative instead.

## Privacy and evidence handling

Only `/healthz` status and the test-only generation header are sampled. Evidence must not capture an `Authorization` header, `Cookie`, request body, DSN, APP secret, user/project identity, schema payload, or other product data. The application secret and database password created by the harness are disposable local fixture values, are not printed, and are removed during cleanup. Traefik logging is retained only for the isolated characterization runtime.

A GREEN workflow means the characterization executed and produced source-bound evidence; it does not mean every observed behavior is desirable. The receipt fields such as `in_place_reload_detected`, `malformed_last_known_good`, and `atomic_replace_detected` are observations that feed the owner decision. Recovery to a valid generation and controlled recreation consuming the replaced path are fixture-integrity requirements.

## Promotion boundary

This lane is a Draft child of the stale #12 Admin Config stack and cannot transfer evidence across parent reconciliation. The manual workflow cannot run as promotion evidence until its definition is integrated on protected `main`. Issue #109 remains open until the pg-erd-cloud owner records whether runtime reload is supported and the selected disposition is proved through the appropriate release-qualified consumer path.

No mutable consumer branch, product-source copy, provider-specific business routing, self-approval, security suppression, immutable-release claim, canary/cutover claim, or legacy-Traefik removal follows from this characterization.

## References

Docker, Inc. (2026a). *Bind mounts*. Docker Docs. https://docs.docker.com/engine/storage/bind-mounts/

Docker, Inc. (2026b). *Define services in Docker Compose: Ports*. Docker Docs. https://docs.docker.com/reference/compose-file/services/#ports

Docker, Inc. (2026c). *docker compose*. Docker Docs. https://docs.docker.com/reference/cli/docker/compose/

Traefik Labs. (n.d.). *File provider*. Traefik Proxy documentation. Retrieved September 18, 2026, from https://doc.traefik.io/traefik/providers/file/

ContextualWisdomLab. (2026a). *pg-erd-cloud: compose.prod.yaml* (commit `8dc746920c12988f082e914879d95e13c9693535`). GitHub.

ContextualWisdomLab. (2026b). *pg-erd-cloud: deploy/traefik/dynamic.yaml* (commit `8dc746920c12988f082e914879d95e13c9693535`). GitHub.
