.PHONY: help dev build release release-mac release-win test lint fmt clean install-deps check

# Rust toolchain — use absolute path to ensure cargo/rustc are always found
RUST_BIN := $(HOME)/.rustup/toolchains/stable-aarch64-apple-darwin/bin
CARGO := PATH="$(RUST_BIN):$(HOME)/.cargo/bin:/opt/homebrew/bin:$$PATH" cargo
RUSTUP := PATH="$(RUST_BIN):$(HOME)/.cargo/bin:/opt/homebrew/bin:$$PATH" rustup

# Default target
help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ─── Development ──────────────────────────────────────────────────────────────

dev: ## Run the app in development mode
	$(CARGO) tauri dev

# ─── Build ────────────────────────────────────────────────────────────────────

build: ## Build the Rust backend (debug)
	cd src-tauri && $(CARGO) build

release: ## Build production release bundle for current platform
	$(CARGO) tauri build

release-mac: ## Build macOS bundle (.app + .dmg) for Apple Silicon
	$(CARGO) tauri build --target aarch64-apple-darwin

release-win: ## Build Windows bundle (.exe / .msi)
	$(CARGO) tauri build --target x86_64-pc-windows-gnu

# ─── Quality ──────────────────────────────────────────────────────────────────

test: ## Run all unit tests
	cd src-tauri && $(CARGO) test

test-verbose: ## Run tests with output
	cd src-tauri && $(CARGO) test -- --nocapture

check: ## Run cargo check (fast compile verification)
	cd src-tauri && $(CARGO) check

lint: ## Run clippy linter
	cd src-tauri && $(CARGO) clippy -- -D warnings

fmt: ## Format Rust code
	cd src-tauri && $(CARGO) fmt

fmt-check: ## Check formatting without modifying files
	cd src-tauri && $(CARGO) fmt -- --check

# ─── Setup ────────────────────────────────────────────────────────────────────

install-deps: ## Install required tooling
	$(CARGO) install tauri-cli --version "^2"
	$(RUSTUP) component add clippy rustfmt

# ─── Maintenance ──────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cd src-tauri && $(CARGO) clean

update: ## Update Rust dependencies
	cd src-tauri && $(CARGO) update
