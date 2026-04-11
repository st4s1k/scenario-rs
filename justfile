set windows-shell := ["bash", "-cu"]

gui_dir := "gui"

# List available recipes
[doc("List all available recipes")]
default:
    @just --list

# --- Version ---

# Show current version
[doc("Show current version")]
version:
    @rustc scripts/version.rs -o target/version --edition 2021
    @target/version

# Set version across all packages
[doc("Set version across all packages")]
bump-version new_version:
    @rustc scripts/bump-version.rs -o target/bump-version --edition 2021
    @target/bump-version {{new_version}}

# --- Rust workspace ---

# Run cargo check on the workspace
[doc("Run cargo check on the workspace")]
check:
    cargo check

# Generate JSON Schema for the scenario config format
[doc("Generate JSON Schema for scenario config")]
schema:
    @cargo run --example generate_schema -p scenario-rs-core

# Build the workspace (debug)
[doc("Build the workspace (debug)")]
build:
    cargo build

# Build the workspace (release)
[doc("Build the workspace (release)")]
build-release:
    cargo build --release

# Run Rust tests with coverage (requires cargo-tarpaulin)
[doc("Run Rust tests with coverage (requires cargo-tarpaulin)")]
test:
    cargo tarpaulin

# Run Angular unit tests with coverage
[doc("Run Angular unit tests with coverage")]
ng-test: npm-install
    cd {{gui_dir}} && npx ng test --watch=false --code-coverage

# Run all tests (Rust + Angular)
[doc("Run all tests (Rust + Angular)")]
test-all: test ng-test

# --- TUI ---

# Run TUI in development mode
[doc("Run TUI in development mode")]
tui-run *args:
    cargo run -p scenario-rs-tui -- {{args}}

# Build TUI (release)
[doc("Build TUI (release)")]
tui-build:
    cargo build -p scenario-rs-tui --release

# --- Frontend ---

# Install frontend dependencies
[doc("Install frontend dependencies")]
npm-install:
    cd {{gui_dir}} && npm install

# Build Angular frontend
[doc("Build Angular frontend")]
ng-build: npm-install
    cd {{gui_dir}} && npm run build

# --- Tauri ---

# Run GUI in development mode
[doc("Run GUI in development mode")]
tauri-dev: npm-install
    cd {{gui_dir}} && npm run tauri dev

# Build GUI (release)
[doc("Build GUI (release)")]
tauri-build: npm-install
    cd {{gui_dir}} && npm run tauri build

# Build GUI (debug)
[doc("Build GUI (debug)")]
tauri-build-debug: npm-install
    cd {{gui_dir}} && npm run tauri build -- --debug

# --- Composite ---

# Build everything (Rust workspace + GUI debug)
[doc("Build everything (Rust workspace + GUI debug)")]
build-all: build tauri-build-debug

# Build everything (Rust workspace + GUI release)
[doc("Build everything (Rust workspace + GUI release)")]
build-all-release: build-release tauri-build

# Full verification: check + all tests + Angular build
[doc("Full verification: check + all tests + Angular build")]
verify: check test-all ng-build
