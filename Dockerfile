# syntax=docker/dockerfile:1

# ---- 1. Build the Solid frontend ----
FROM node:24-alpine AS web
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# ---- 2. Build the Rust server ----
FROM rust:1-slim-trixie AS server
WORKDIR /app/server
COPY server/ ./
# Cache mounts keep downloaded crates and compiled deps between builds.
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
ENTRYPOINT ["/app/server"]
