# syntax=docker/dockerfile:1.7
FROM rust:1.98.0-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release \
    --bin cwl-pingora-gateway \
    --bin cwl-pingora-pg-erd-migration

# Pingora's pinned `pingora-openssl` dependency enables the OpenSSL crate's `vendored` feature, so
# the gateway binaries do not require Debian's libssl at runtime. Keep both runtime targets on the
# same distroless `base-nossl` foundation rather than duplicating or widening packaging authority.
# The current Rust binaries do require libgcc_s for unwinding; copy that single runtime library from
# the already-pinned build environment. Both final targets inherit the same non-root identity and
# are exercised under read-only-root, capability-free, no-new-privileges constraints in CI.
# The base-nossl digest is the Debian 13 nonroot image also pinned by Envoy 1.39.1 (2026-08-27).
FROM gcr.io/distroless/base-nossl-debian13:nonroot@sha256:5cab74e7f8a5e7c5f1c8a9e6268b1f352f053c36c656f493308340bcecbc636c AS runtime-common

COPY --from=builder /usr/lib/x86_64-linux-gnu/libgcc_s.so.1 /usr/lib/x86_64-linux-gnu/libgcc_s.so.1

# Neither process requires a writable application directory. Operators can run either target with a
# read-only root filesystem and mount only the corresponding versioned configuration read-only.
USER 65532:65532

FROM runtime-common AS pg-erd-migration
COPY --from=builder /src/target/release/cwl-pingora-pg-erd-migration /usr/local/bin/cwl-pingora-pg-erd-migration
ENTRYPOINT ["/usr/local/bin/cwl-pingora-pg-erd-migration"]

# Keep the generic gateway as the final/default target so existing `docker build .` consumers retain
# the same binary and entrypoint. The dedicated migration image must be selected explicitly with
# `--target pg-erd-migration` and therefore cannot silently replace generic v1 packaging.
FROM runtime-common AS gateway
COPY --from=builder /src/target/release/cwl-pingora-gateway /usr/local/bin/cwl-pingora-gateway
ENTRYPOINT ["/usr/local/bin/cwl-pingora-gateway"]
