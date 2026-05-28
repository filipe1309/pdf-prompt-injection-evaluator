.PHONY: help dev build release release-mac release-win test lint fmt clean install-deps check

# Ensure Rust toolchain is on PATH
export PATH := $(HOME)/.cargo/bin:$(HOME)/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$(PATH)

# Default target
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ─── Development ──────────────────────────────────────────────────────────────

dev: ## Run the app in development mode
	cargo tauri dev

# ─── Build ────────────────────────────────────────────────────────────────────

build: ## Build the Rust backend (debug)
	cd src-tauri && cargo build

release: ## Build production release bundle for current platform
	cargo tauri build

release-mac: ## Build macOS bundle (.app + .dmg) for Apple Silicon
	cargo tauri build --target aarch64-apple-darwin

release-win: ## Build Windows bundle (.exe / .msi)
	cargo tauri build --target x86_64-pc-windows-msvc

# ─── Quality ──────────────────────────────────────────────────────────────────

test: ## Run all unit tests
	cd src-tauri && cargo test

test-verbose: ## Run tests with output
	cd src-tauri && cargo test -- --nocapture

check: ## Run cargo check (fast compile verification)
	cd src-tauri && cargo check

lint: ## Run clippy linter
	cd src-tauri && cargo clippy -- -D warnings

fmt: ## Format Rust code
	cd src-tauri && cargo fmt

fmt-check: ## Check formatting without modifying files
	cd src-tauri && cargo fmt -- --check

# ─── Setup ────────────────────────────────────────────────────────────────────

install-deps: ## Install required tooling
	cargo install tauri-cli --version "^2"
	rustup component add clippy rustfmt

# ─── Maintenance ──────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cd src-tauri && cargo clean

update: ## Update Rust dependencies
	cd src-tauri && cargo update
