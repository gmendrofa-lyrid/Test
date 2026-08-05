# snop_cockpit_be — own-DB layer for the S&OP Cockpit (Part A: planning parameters).
# DATABASE_URL / DB_* are read from .env by cargo, sqlx-cli, and the app at runtime.

.DEFAULT_GOAL := help

# Build offline using the checked-in .sqlx/ query cache — no live DATABASE_URL needed.
# Requires `make prepare` to have been run once (with the DB reachable) to populate .sqlx/.
export SQLX_OFFLINE ?=

.PHONY: help run dev watch build release build-offline release-offline check-offline seed migrate setup reset-db prepare test fmt check clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

run: ## Run the API server (http://localhost:9093)
	cargo run

dev: watch ## Alias for `watch`

watch: ## Auto-reload the server on changes (needs cargo-watch)
	cargo watch -x run -w src

build: ## Debug build (needs DATABASE_URL — sqlx type-checks queries against the DB)
	cargo build

release: ## Optimized release build (needs DATABASE_URL)
	cargo build --release

build-offline: ## Debug build WITHOUT a DB (uses .sqlx/ cache; set SQLX_OFFLINE=1 if needed)
	SQLX_OFFLINE=1 cargo build

release-offline: ## Optimized release build WITHOUT a DB (uses .sqlx/ cache)
	SQLX_OFFLINE=1 cargo build --release

seed: ## Load default config + sample params (scripts/seed.sql)
	cargo run --bin seed

migrate: ## Apply database migrations
	sqlx migrate run

setup: migrate seed ## First-time setup: migrate + seed

reset-db: ## Drop, recreate, migrate, and reseed the database
	sqlx database reset -y
	$(MAKE) seed

prepare: ## Regenerate the .sqlx offline query cache
	cargo sqlx prepare

test: ## Run the native Rust API test executable
	cargo run --bin test

fmt: ## Format the code
	cargo fmt

check: ## Type-check / lint without producing a binary (needs DATABASE_URL)
	cargo check

check-offline: ## Type-check without a DB (uses .sqlx/ cache)
	SQLX_OFFLINE=1 cargo check

clean: ## Remove build artifacts
	cargo clean
