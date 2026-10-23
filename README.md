# Rust + Solid starter

[![CI](https://github.com/benzend/rust-solid-starter/actions/workflows/ci.yml/badge.svg)](https://github.com/benzend/rust-solid-starter/actions/workflows/ci.yml)

Axum API + SolidJS frontend + Postgres, with types shared from Rust to TypeScript.

- **Server:** Axum 0.8, sqlx 0.9 (Postgres, migrations, compile-time checked queries), Tokio
- **Frontend:** SolidJS 1.9, @solidjs/router, Vite 8, TypeScript 7
- **UI:** Tailwind v4 + shadcn/ui-style components (`web/src/components/ui`)
- **Tooling:** Docker Compose, Biome, rustfmt + clippy, Vitest, GitHub Actions

## Prerequisites

- [Rust](https://rustup.rs) (the pinned toolchain installs automatically from `rust-toolchain.toml`)
- [sqlx-cli](https://crates.io/crates/sqlx-cli), only needed when you change SQL: `cargo install sqlx-cli --no-default-features --features rustls,postgres`
- Node 22.12+ (24 recommended)
- Docker (or Podman) for Postgres

## Quick start

```sh
make setup   # creates .env and installs frontend deps
make dev     # Postgres in Docker + Rust API on :3000 + Vite on :5173
```

Open http://localhost:5173. Vite hot-reloads the UI and forwards `/api` to Axum. Rust changes need a restart of `make dev`, or run the server under a watcher such as `bacon` or `cargo watch` to rebuild on save.

Run `make` to list every command.

| Command | What it does |
|---|---|
| `make dev` | Postgres + API + UI with hot reload |
| `make test` | Rust tests (each gets its own throwaway database) and Vitest |
| `make prepare` | Refresh the `.sqlx/` query cache after changing SQL |
| `make types` | Regenerate TypeScript types from Rust structs |
| `make check` | fmt, clippy, `.sqlx/` cache check, Biome, and type-check (same as CI) |
| `make up` | Build and run the production image + Postgres on :3000 |

## How it fits together

```
server/
  migrations/        SQL migrations, compiled in and applied on startup
  .sqlx/             cached query metadata for offline builds (generated, commit it)
  src/lib.rs         router, middleware (compression, timeout, security headers)
  src/todos.rs       example resource: shared types + handlers
  src/error.rs       AppError -> JSON { message } responses
  tests/api.rs       API tests against a real database
web/
  src/types/         GENERATED from Rust by ts-rs; don't edit by hand
  src/lib/api.ts     typed fetch wrapper (+ api.test.ts)
  src/components/ui/ shadcn-style components (yours to edit)
  src/routes/        pages; non-home routes are lazy-loaded
```

In production, the single Rust binary serves both `/api/*` and the built Solid app. Unknown paths return `index.html`, so client-side routes work on refresh.

### Shared types

Structs marked `#[derive(TS)] #[ts(export)]` in Rust become TypeScript types in `web/src/types/` when you run `make types`. If you rename or retype a field in Rust, the frontend fails to type-check until you update it. CI also fails if you forget to regenerate.

### Compile-time checked SQL

Queries use `sqlx::query_as!` / `query!`, so a misspelled column or a wrong type is a build error, not a runtime one. Builds read the committed `server/.sqlx/` cache rather than a live database (`SQLX_OFFLINE` is set in `server/.cargo/config.toml`), so `cargo build`, Docker, and editors work without Postgres running. When you add or change a query, run `make prepare` and commit `.sqlx/`. CI fails if the cache is out of date.

### Adding a feature

1. Add a migration in `server/migrations/` (`<timestamp>_name.sql`).
2. Add a module like `todos.rs` with types and handlers, and `.merge()` its router in `lib.rs`.
3. Run `make prepare` (query cache) and `make types` (TypeScript types), then call it from `web/src/lib/api.ts`.

### UI components

Components follow shadcn/ui conventions: Tailwind classes, `cva` variants, and the `cn()` helper. Theme colors are CSS variables in `web/src/index.css`, and dark mode follows the OS. For more components (dialog, dropdown, and so on), copy them from the Solid ports [solid-ui](https://github.com/stefan-karger/solid-ui) or [shadcn-solid](https://github.com/hngngn/shadcn-solid), which are built on Kobalte.

## Production

`docker compose up --build` runs the same image you'd deploy. The image is multi-stage: Node builds the UI, Rust builds the binary, and the result runs on distroless as non-root, at about 53 MB total. Set `DATABASE_URL` in your host's environment. Optional: `PORT` (3000), `RUST_LOG`, `DB_MAX_CONNECTIONS` (10), `REQUEST_TIMEOUT_SECS` (10). The image has a `HEALTHCHECK` that runs `server --health-check`, which probes `/api/health` (and through it, the database). It runs anywhere that runs containers, such as Fly.io, Railway, Render, or a VPS.

Production checklist:

- Use a managed Postgres or real credentials. The compose credentials are for development only. TLS to the database is supported via rustls.
- Put TLS (HTTPS) in front, using your platform's proxy or Caddy.
- Revisit the Content-Security-Policy in `lib.rs` if you add third-party scripts, fonts, or analytics.

## Upgrades worth considering

- **Auth:** sessions via `tower-sessions`, or an external provider.
- **End-to-end tests:** Playwright against `make up`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
