# syntax=docker/dockerfile:1

FROM rust:1.89-bookworm AS builder
WORKDIR /app

# Build dependencies first when Cargo manifests change, then build the locked release binary.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin airgap

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --shell /usr/sbin/nologin airgap

COPY --from=builder /app/target/release/airgap /usr/local/bin/airgap

USER airgap
WORKDIR /home/airgap

EXPOSE 8080

# Default CMD serves stdio MCP for turnkey registries (Glama, NodeFlare).
# Override CMD with `serve --host 0.0.0.0 --port 8080` for Streamable HTTP hosting (Fly.io, Railway).
ENTRYPOINT ["airgap"]
CMD ["--mcp"]
