# .env is deliberately not included here: Make would parse it as Make syntax and
# mangle values containing `$`. The server and sqlx-cli read .env themselves.
# `cargo test` is the exception (#[sqlx::test] only reads the environment), so
# its recipe loads .env through the shell, which handles quoting properly.
with-env = set -a; [ ! -f .env ] || . ./.env; set +a;

.DEFAULT_GOAL := help
.PHONY: help setup db dev server web types prepare test check fmt build up down

help: ## Show available commands
	@grep -E '^[a-z-]+:.*## ' Makefile | awk 'BEGIN {FS = ":.*## "}; {printf "  make %-8s %s\n", $$1, $$2}'

setup: ## First-time setup: .env file and frontend dependencies
	@test -f .env || cp .env.example .env
	cd web && npm install

db: ## Start Postgres in Docker (waits until it's ready)
	docker compose up -d --wait db

dev: db ## Postgres + Rust API (:3000) + Vite with hot reload (:5173)
	@$(MAKE) -j2 server web

server: ## Run only the Rust API
	cd server && cargo run

web: ## Run only the Vite dev server
	cd web && npm run dev

types: ## Regenerate TypeScript types from the Rust structs
	cd server && cargo test --lib export_bindings --quiet

prepare: db ## Refresh the .sqlx/ query cache after changing SQL (commit it)
	cd server && cargo sqlx prepare -- --all-targets

test: db types ## Run Rust and frontend tests (each Rust test gets its own database)
	$(with-env) cd server && cargo test
	cd web && npm test

check: db ## Format check, lint, type-check, and verify the .sqlx/ cache (what CI runs)
	cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings
	cd server && cargo sqlx prepare --check -- --all-targets
	cd web && npm run lint && npm run typecheck

fmt: ## Format everything
	cd server && cargo fmt
	cd web && npm run format

build: ## Production build of both sides
	cd web && npm run build
	cd server && cargo build --release

up: ## Run the production image + Postgres in Docker on :3000
	docker compose up --build

down: ## Stop Docker services (data is kept in a volume)
	docker compose down
