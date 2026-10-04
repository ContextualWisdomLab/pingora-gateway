# Configuration-only preflight — bounded feature traceability

## Scope and alignment

This addition serves platform operators in the existing shared edge-runtime bounded context. It supports the PRD requirement to parse the strict versioned configuration before granting network authority and the gap baseline's fail-closed configuration / operability areas. It does not replace product routing policy, certificate issuance/rotation, health semantics, migration parity, or release governance. PRD, TRD, the shared gap baseline and shared TRACEABILITY remain unchanged.

The coordinating agent reviewed a live inventory of 44 open PRs and 31 issue-only entries before assigning this isolated addition on `feat-mixed-agent-prd`, based on foundation `0cf3b4377745d8a59f40f16e60a164e443595bec`. Those observations are planning context, not permanent current-state or integration evidence. Existing protocol/security/consumer-migration lanes retain their owners. No PR, commit, push, merge, deployment or publication is part of this implementation.

## Acceptance boundaries

| Requirement | Owned implementation / executable evidence |
| --- | --- |
| Both `--check --config PATH` and `--config PATH --check` | `startup::GatewayCommand`; compiled-binary `tests/config_preflight.rs` |
| Same strict YAML and trust material as serving | Shared `GatewayCommand::load_config` and `GatewayProxy::try_from_config` before mode selection; invalid inputs compared against serving |
| No server constructor/bootstrap or listener activation | Binary early return before `Server::new_with_opt_and_conf`; occupied traffic and metrics listener positive |
| Local-only, no origin probing | Positive config uses an unreachable loopback origin; no network-validation branch |
| Static success and fail-closed arguments | Exact stdout/exit assertions; unknown, duplicate and missing option/path acceptance cases |
| Credential-safe diagnostics | Invalid YAML/trust sentinels stay absent from binary output; public config/error APIs retained |
| Normal serving preserved | Existing startup, production-path, refused-origin, health, TLS and graceful-shutdown regressions |

## Limits and gates

Success proves only local configuration and configured custom PEM loading at that instant. It does not prove upstream reachability, free ports, issuer identity, TLS handshake success, deployment readiness, or release readiness. Normal startup rereads and validates inputs because files, permissions and environment may change after preflight.

Local test receipts are held separately in the task's scratch evidence directory. A changed candidate still needs exact-current-head hosted CI, 100% owned-production line/region coverage, warning-denied rustdoc, k6, OCI non-root/read-only-root, security/supply-chain and independent review evidence under live policy. Supplier advisory / derivative disposition, protected-main integration, immutable artifact/provenance and consumer rollback gates are not resolved or waived here. No consumer migration or production readiness is claimed.
