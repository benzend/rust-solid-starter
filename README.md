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
- Node 22.12+ (24 recommended) and [pnpm](https://pnpm.io/installation) 12 (`npm install -g pnpm@12`). pnpm 10 can't switch itself to 12, so upgrade it first.
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
| `make deploy` | Deploy to your server with Kamal |

## How it fits together

```
server/
  migrations/        SQL migrations, compiled in and applied on startup
  .sqlx/             cached query metadata for offline builds (generated, commit it)
  src/lib.rs         router, middleware (compression, timeout, security headers)
  src/todos.rs       example resource: shared types + handlers
  src/error.rs       AppError -> JSON { message } responses
  src/reporting.rs   the one file that knows about an error tracker (off by default)
  tests/api.rs       API tests against a real database
web/
  src/types/         GENERATED from Rust by ts-rs; don't edit by hand
  src/lib/api.ts     typed fetch wrapper (+ api.test.ts)
  src/lib/errors.ts  reportError(): the frontend's one error-tracker file
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

### Dependency age policy

pnpm 12 refuses to install any package version published less than a day ago, a guard against compromised releases, which are usually caught within hours. If an install fails with `ERR_PNPM_MINIMUM_RELEASE_AGE_VIOLATION` right after an update, wait a day, or re-resolve just the flagged packages to their previous versions with `pnpm update --depth Infinity <package>`.

### UI components

Components follow shadcn/ui conventions: Tailwind classes, `cva` variants, and the `cn()` helper. Theme colors are CSS variables in `web/src/index.css`, and dark mode follows the OS. For more components (dialog, dropdown, and so on), copy them from the Solid ports [solid-ui](https://github.com/stefan-karger/solid-ui) or [shadcn-solid](https://github.com/hngngn/shadcn-solid), which are built on Kobalte.

## Production

`docker compose up --build` runs the same image you'd deploy. The image is multi-stage: Node builds the UI, Rust builds the binary, and the result runs on distroless as non-root, at about 53 MB total. Set `DATABASE_URL` in your host's environment. Optional: `PORT` (3000), `RUST_LOG`, `DB_MAX_CONNECTIONS` (10), `REQUEST_TIMEOUT_SECS` (10). The image has a `HEALTHCHECK` that runs `server --health-check`, which probes `/api/health` (and through it, the database). It runs anywhere that runs containers. The template ships a [Kamal](https://kamal-deploy.org) setup for deploying it to your own server (below).

Production checklist:

- Use real credentials. The compose credentials are for development only. TLS to a managed database is supported via rustls (`?sslmode=require`).
- Revisit the Content-Security-Policy in `lib.rs` if you add third-party scripts, fonts, or analytics.

## Deploying (Kamal)

[Kamal](https://kamal-deploy.org) deploys the Docker image to any Linux server over SSH, with zero-downtime switchovers, automatic HTTPS, and rollbacks. Postgres runs next to the app as a Kamal "accessory", with a nightly backup. Everything lives in `config/deploy.yml`, `.kamal/secrets`, and `.github/workflows/deploy.yml`.

### Make it yours

1. Get a server with Ubuntu 24.04+ or Debian 12+ and SSH access as root by key (e.g. Hetzner, DigitalOcean). When creating it, paste `config/cloud-init.yml` into the provider's "cloud-init" or "user data" field: it turns off password logins, enables a firewall (SSH, HTTP, HTTPS only) and turns on automatic security updates. Point your domain's DNS at it.
2. In `config/deploy.yml`, fill in every `CHANGE ME`: service name, image, server IP (three places), domain, GitHub user. If the server is ARM, set `builder.arch: arm64` and `runs-on: ubuntu-24.04-arm` in the deploy workflow.
3. If you renamed the service, update the database host in `.kamal/secrets` (`<service>-db`).
4. In the GitHub repo settings:
   - **Secrets:** `POSTGRES_PASSWORD` (`openssl rand -hex 32`) and `SSH_PRIVATE_KEY` (a key whose public half is in the server's `~/.ssh/authorized_keys`).
   - **Variables:** `KAMAL_ENABLED` = `true`. Until this is set, the deploy workflow does nothing.
5. First deploy, from your machine: install Kamal (`gem install kamal`, Ruby 3.2+), export `POSTGRES_PASSWORD` (same value as the secret) and `KAMAL_REGISTRY_PASSWORD` (a GitHub token with `write:packages`), then run `kamal setup`. It installs Docker on the server and starts the proxy, Postgres, the backup job, and the app.

After that, every merge to `main` deploys automatically once CI passes. You can also run the **Deploy** workflow by hand from the Actions tab.

### Day to day

| Command | What it does |
|---|---|
| `kamal deploy` | Build and ship the current commit |
| `kamal rollback <version>` | Switch back to an earlier version (the last 5 are kept) |
| `kamal logs` | Follow the app's logs |
| `kamal db` | Open `psql` on the production database |
| `kamal accessory reboot db` | Restart Postgres (deploys never touch it) |

The production image has no shell, so `kamal app exec` only runs commands that exist in it, such as `/app/server --health-check`.

### Things to know

- **Migrations must be backward-compatible.** The app migrates on boot, and the old container keeps serving until the new one passes `/api/health`. Add columns before code uses them; drop them a deploy later. A rollback switches code, not the schema.
- **Builds are native in CI.** `builder.arch` must match the server. Building for x86 on an Apple Silicon Mac goes through emulation and is very slow for Rust, so let the workflow build, or use an ARM server.
- **Backups:** the `backup` accessory dumps the database daily and keeps 7 days in `~/<service>-backup/backups` on the server. That covers bad migrations and deletes, not losing the server. See [Backups](#backups) for off-site copies.
- **Managed Postgres instead:** delete the `db` and `backup` accessories and set `DATABASE_URL` in `.kamal/secrets` to the provider's URL.

### Backups

Local dumps alone don't survive losing the server. To also keep encrypted copies in S3-compatible storage (Cloudflare R2, Backblaze B2, AWS S3):

1. Create a bucket and an access key for it. Give the key write access but not delete where the provider allows, so a compromised server can't erase the backups. Add a lifecycle rule to expire old objects (e.g. after 30 days).
2. Run `age-keygen -o backup-key.txt`. Put the public key (`age1...`) in `BACKUP_AGE_RECIPIENT` and keep `backup-key.txt` somewhere safe that isn't the server, such as a password manager. Without it the backups can't be decrypted.
3. Uncomment the off-site lines in the `backup` accessory in `config/deploy.yml` and in `.kamal/secrets`, export the two key variables, and run `kamal accessory reboot backup`. Also add both as repository secrets and to the `env:` block in `.github/workflows/deploy.yml`, since CI deploys read `.kamal/secrets` too.
4. Optional: point `BACKUP_PING_URL` at a dead-man's-switch monitor such as [healthchecks.io](https://healthchecks.io), so you hear about it when backups stop.

**Test a restore once.** `kamal accessory exec backup --reuse "env BACKUP_ONCE=1 sh /usr/local/bin/db-backup.sh"` runs a backup now. Download the `.age` file, then restore it into a local database:

```bash
age -d -i backup-key.txt <file>.dump.age | pg_restore --clean --if-exists -d app
```

## Security

What the template does:

- **App:** strict browser security headers (CSP, HSTS, no framing, `nosniff`), SQL checked at compile time, no `unsafe` Rust, and error responses that never include internal details.
- **Container:** a distroless image with no shell or package manager, running as non-root.
- **Server:** key-only SSH, a firewall, and automatic security updates via `config/cloud-init.yml`. Postgres is never exposed outside Docker's network.
- **Dependencies:** `cargo deny` (`server/deny.toml`) fails CI on Rust crates with known vulnerabilities, unapproved licenses, or unknown sources, and CI reruns weekly so new advisories surface. Dependabot opens weekly update PRs for crates, npm packages and GitHub Actions. pnpm and Dependabot both skip versions published in the last day.
- **CI:** third-party actions are pinned to commit hashes, workflows get a read-only token by default, and the deploy workflow only deploys CI runs from pushes to this repository.

Per-repository settings to turn on (GitHub doesn't copy them from a template):

- **Settings > Code security:** Dependabot alerts and Dependabot security updates, so vulnerable npm packages and crates get fix PRs right away.
- **Settings > Rules:** protect `main` and require CI to pass, since merging to `main` deploys.

## Errors and logs

- **Every request gets an ID** (`x-request-id`, generated unless the client sends one). It's on every log line for that request and in the response headers. Server errors shown in the UI include it ("Something went wrong. (ref …)"), so a user's report can be matched to the logs.
- **Server:** expected errors (validation, not found, bad JSON) return `{ message }` with a 4xx. Database errors and panics are logged at ERROR and return a generic 500; details never reach the client. Startup failures are logged the same way. Logs are text in dev and JSON lines in production (`LOG_FORMAT=json`, set in the Dockerfile), one line per request plus any errors. Read them with `kamal logs`.
- **Frontend:** failed actions show the message inline. A page that crashes while rendering shows a "Something went wrong" screen inside the layout instead of a blank page. Uncaught errors and rejected promises are caught globally. All of these go through `reportError()`, which skips 4xx responses (user input, not bugs) and logs to the console.

### Adding error tracking

Reporting has one seam per side, so adding a service (Sentry, Honeybadger, and so on) means editing those two files plus some config, with no changes to handlers or pages. On the server, everything reports by logging with `tracing::error!`, and trackers plug in as a `tracing` layer. Example with Sentry:

1. **Server:** `cargo add sentry --features tracing`, then fill in `server/src/reporting.rs`:

   ```rust
   pub fn init<S>() -> (Option<impl Layer<S> + Send + Sync>, Guard)
   where
       S: Subscriber + for<'a> LookupSpan<'a>,
   {
       let Ok(dsn) = std::env::var("SENTRY_DSN") else {
           return (None, Guard { _client: None }); // no DSN: reporting stays off
       };
       let client = sentry::init((dsn, sentry::ClientOptions {
           release: sentry::release_name!(),
           ..Default::default()
       }));
       // ERROR events become Sentry issues; INFO/WARN become breadcrumbs.
       (Some(sentry::integrations::tracing::layer()), Guard { _client: Some(Box::new(client)) })
   }
   ```

   Add `SENTRY_DSN` to `env.secret` in `config/deploy.yml` and to `.kamal/secrets`.

2. **Frontend:** `pnpm add @sentry/solid`, then forward from `web/src/lib/errors.ts`:

   ```ts
   import * as Sentry from "@sentry/solid";

   const dsn = import.meta.env.VITE_SENTRY_DSN;
   if (dsn) Sentry.init({ dsn });

   export function reportError(error: unknown, context?: Record<string, unknown>) {
     if (error instanceof ApiRequestError && error.status < 500) return;
     const requestId = error instanceof ApiRequestError ? error.requestId : undefined;
     console.error(error, { ...context, requestId });
     Sentry.captureException(error, { extra: { ...context, requestId } });
   }
   ```

   `VITE_` variables are baked in at build time, so pass `VITE_SENTRY_DSN` to the Docker build (a DSN is safe to expose in the browser).

3. **Allow the tracker in the CSP.** The Content-Security-Policy in `server/src/lib.rs` only allows same-origin requests, so the browser will block reports until you add the tracker's host, e.g. `connect-src 'self' https://*.ingest.sentry.io`.

Treat these snippets as a starting point and check them against the tracker's current docs; SDK APIs change. Honeybadger works the same way: `Honeybadger.notify(error, { context })` in `errors.ts`.

## Upgrades worth considering

- **Auth:** sessions via `tower-sessions`, or an external provider.
- **End-to-end tests:** Playwright against `make up`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
