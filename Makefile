.DEFAULT_GOAL := help

MOUNTOS ?= ../mountos-servers/bin/mountos

# Packaging & signing
APP_NAME := mountOS Desktop
BUNDLE_DIR := src-tauri/target/release/bundle
MAC_APP := $(BUNDLE_DIR)/macos/$(APP_NAME).app
TIMESTAMP_URL ?= http://timestamp.digicert.com

.PHONY: help install dev desktop-dev check test test-rust lint format build desktop-build cli-smoke verify clean \
	bundle sign-macos notarize-macos sign-windows release-macos release-windows

help: ## List every available make command
	@awk 'BEGIN {FS = ":.*## "; printf "mountOS Desktop commands:\n\n"} /^[a-zA-Z0-9_-]+:.*## / {printf "  %-16s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install: ## Install JavaScript dependencies from the lockfile
	npm ci

dev: ## Start the browser UI development server
	npm run dev

desktop-dev: ## Start the Tauri desktop application in development mode
	npm run tauri:dev

check: ## Run Svelte and TypeScript static checks
	npm run check

test: ## Run TypeScript unit tests
	npm test

test-rust: ## Run Rust unit and documentation tests
	cargo test --manifest-path src-tauri/Cargo.toml

lint: ## Run strict Rust Clippy checks
	cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

format: ## Format Rust source files
	cargo fmt --manifest-path src-tauri/Cargo.toml

build: ## Build the production web assets
	npm run build

desktop-build: ## Build the packaged desktop application
	npm run tauri:build

cli-smoke: ## Verify the configured mountos CLI public read commands
	$(MOUNTOS) --version
	$(MOUNTOS) check --json >/dev/null
	$(MOUNTOS) list --json >/dev/null

verify: check test test-rust lint build ## Run all local verification except packaging

# With APPLE_SIGNING_IDENTITY exported, Tauri signs the .app and .dmg during
# bundling, so release-macos only needs notarization afterwards. sign-macos is
# the standalone fallback for re-signing an unsigned .app (the .dmg must be
# rebuilt after a re-sign or it ships the unsigned copy).
bundle: ## Build distributable bundles (macOS app+dmg, Windows nsis)
ifeq ($(OS),Windows_NT)
	npm run tauri:build -- --bundles nsis
else
	npm run tauri:build -- --bundles app,dmg
endif

sign-macos: ## Codesign the built .app (env: APPLE_SIGNING_IDENTITY)
	@test -n "$(APPLE_SIGNING_IDENTITY)" || { echo "error: APPLE_SIGNING_IDENTITY is required (Developer ID Application identity)"; exit 1; }
	codesign --force --options runtime --timestamp --sign "$(APPLE_SIGNING_IDENTITY)" "$(MAC_APP)"
	codesign --verify --deep --strict "$(MAC_APP)"

notarize-macos: ## Notarize + staple the .dmg (env: APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID)
	@test -n "$(APPLE_ID)" || { echo "error: APPLE_ID is required"; exit 1; }
	@test -n "$(APPLE_PASSWORD)" || { echo "error: APPLE_PASSWORD (app-specific password) is required"; exit 1; }
	@test -n "$(APPLE_TEAM_ID)" || { echo "error: APPLE_TEAM_ID is required"; exit 1; }
	@dmg=$$(ls "$(BUNDLE_DIR)/dmg"/*.dmg 2>/dev/null | head -1); \
	test -n "$$dmg" || { echo "error: no .dmg under $(BUNDLE_DIR)/dmg; run make bundle first"; exit 1; }; \
	xcrun notarytool submit "$$dmg" --apple-id "$(APPLE_ID)" --password "$(APPLE_PASSWORD)" --team-id "$(APPLE_TEAM_ID)" --wait; \
	xcrun stapler staple "$$dmg"

# Run from a POSIX shell (Git Bash or CI); signtool comes from the Windows SDK.
sign-windows: ## signtool-sign the built NSIS installer (env: WINDOWS_CERT_PFX WINDOWS_CERT_PASSWORD)
	@test -n "$(WINDOWS_CERT_PFX)" || { echo "error: WINDOWS_CERT_PFX (path to .pfx) is required"; exit 1; }
	@test -n "$(WINDOWS_CERT_PASSWORD)" || { echo "error: WINDOWS_CERT_PASSWORD is required"; exit 1; }
	@exe=$$(ls "$(BUNDLE_DIR)/nsis"/*.exe 2>/dev/null | head -1); \
	test -n "$$exe" || { echo "error: no installer under $(BUNDLE_DIR)/nsis; run make bundle first"; exit 1; }; \
	signtool sign /fd SHA256 /td SHA256 /tr "$(TIMESTAMP_URL)" /f "$(WINDOWS_CERT_PFX)" /p "$(WINDOWS_CERT_PASSWORD)" "$$exe"; \
	signtool verify /pa "$$exe"

release-macos: bundle notarize-macos ## Bundle (signed via env) then notarize for macOS
release-windows: bundle sign-windows ## Bundle then signtool-sign for Windows

clean: ## Remove generated frontend and Rust build artifacts
	rm -rf dist src-tauri/target
