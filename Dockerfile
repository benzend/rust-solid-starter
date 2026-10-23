# syntax=docker/dockerfile:1

# ---- 1. Build the Solid frontend ----
FROM node:24-alpine AS web
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ---- 2. Build the Rust server ----
# Keep this tag in step with rust-toolchain.toml.
FROM rust:1.99-slim-trixie AS server
WORKDIR /app/server
COPY server/ ./
# SQL is checked against the committed .sqlx/ cache (SQLX_OFFLINE in .cargo/config.toml),
# so no database is needed here. Cache mounts keep downloaded crates and compiled deps between builds.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/server/target \
    cargo build --release && cp target/release/server /usr/local/bin/server

# ---- 3. Minimal runtime: no shell, no package manager, runs as non-root ----
FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=server /usr/local/bin/server /app/server
COPY --from=web /app/web/dist /app/web/dist
ENV PORT=3000 \
    STATIC_DIR=/app/web/dist
EXPOSE 3000
# The binary probes its own /api/health (distroless has no curl).
HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/app/server", "--health-check"]
ENTRYPOINT ["/app/server"]
