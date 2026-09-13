# syntax=docker/dockerfile:1.7
FROM rust:1.98.0-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release --bin cwl-pingora-gateway

# Pingora's pinned `pingora-openssl` dependency enables the OpenSSL crate's `vendored` feature, so
# the gateway binary does not require Debian's libssl at runtime. Keep the runtime on distroless
# `base-nossl` rather than carrying an unused libssl package and its unrelated QUIC-server attack
# surface. The current Rust binary does require libgcc_s for unwinding; copy that single runtime
# library from the already-pinned build environment instead of widening the final image to the
# distroless `cc` package set. CI starts the exact image under read-only/rootless restrictions and
# the supply-chain workflow scans the final image fail-closed for HIGH/CRITICAL vulnerabilities.
# The base-nossl digest is the Debian 13 nonroot image also pinned by Envoy 1.39.1 (2026-08-27).
FROM gcr.io/distroless/base-nossl-debian13:nonroot@sha256:5cab74e7f8a5e7c5f1c8a9e6268b1f352f053c36c656f493308340bcecbc636c AS runtime

COPY --from=builder /usr/lib/x86_64-linux-gnu/libgcc_s.so.1 /usr/lib/x86_64-linux-gnu/libgcc_s.so.1
COPY --from=builder /src/target/release/cwl-pingora-gateway /usr/local/bin/cwl-pingora-gateway

# The process requires no writable application directory. Operators can run the image with a
# read-only root filesystem and mount only the versioned configuration as a read-only file.
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/cwl-pingora-gateway"]
