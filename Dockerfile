# syntax=docker/dockerfile:1.7
FROM rust:1.98.0-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release --bin cwl-pingora-gateway

# Debian 12 distroless reaches the end of its supported image line in September 2026. The
# Debian 13 base carries only the glibc/libssl/CA runtime needed by this dynamically linked Rust
# binary, without the package manager and unrelated utilities that caused the previous candidate
# image to fail the HIGH/CRITICAL vulnerability gate. The digest is intentionally immutable;
# dependency-update automation must move it through review rather than silently changing runtime
# bytes underneath a source revision.
FROM gcr.io/distroless/base-debian13:nonroot@sha256:b78832f41c8128046807c24840ebee4f1c18ba7870eed423d8750c272c15e147 AS runtime

COPY --from=builder /src/target/release/cwl-pingora-gateway /usr/local/bin/cwl-pingora-gateway

# The process requires no writable application directory. Operators can run the image with a
# read-only root filesystem and mount only the versioned configuration as a read-only file.
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/cwl-pingora-gateway"]
