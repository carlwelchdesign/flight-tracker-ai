.DEFAULT_GOAL := help

.PHONY: help dev down logs verify verify-rust verify-web compose-check

help:
	@echo "Flight Tracker AI"
	@echo "  make dev           Start the complete development stack"
	@echo "  make down          Stop the development stack"
	@echo "  make logs          Follow service logs"
	@echo "  make verify        Run Rust, web, and Compose checks"

dev:
	docker compose up --build

down:
	docker compose down

logs:
	docker compose logs --follow

verify: verify-rust verify-web compose-check

verify-rust:
	docker compose run --rm --no-deps api \
		sh -c 'cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked'

verify-web:
	npm --prefix apps/web ci --no-audit
	npm --prefix apps/web audit --omit=dev --audit-level=moderate
	npm --prefix apps/web run lint
	npm --prefix apps/web run typecheck
	npm --prefix apps/web test
	npm --prefix apps/web run build

compose-check:
	docker compose config --quiet
