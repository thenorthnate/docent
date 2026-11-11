# Docent task runner. `just check` runs the same gates as CI.

# List recipes
default:
    @just --list

# Run every gate CI runs
check: fmt clippy test ui-lint ui-typecheck ui-test

# Rust formatting
fmt:
    cargo fmt --all --check

# Rust lints, warnings are errors
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Rust tests
test:
    cargo test --workspace

# UI lint
ui-lint:
    pnpm --filter docent-ui lint

# UI typecheck
ui-typecheck:
    pnpm --filter docent-ui typecheck

# UI tests
ui-test:
    pnpm --filter docent-ui test

# Build the release app without an installer
build:
    pnpm tauri build --no-bundle

# Install dependencies
setup:
    pnpm install --frozen-lockfile

# Run the app with hot reload
dev:
    pnpm tauri dev
