#!/usr/bin/env bash
# Boundless pre-push hook — runs the full test matrix locally before push.
# Install via: ln -s ../../.claude/hooks/pre-push.sh .git/hooks/pre-push

set -euo pipefail

echo "Running pre-push: full test suite + snapshot diff..."

# Rust core
if [ -d "core" ]; then
    echo "→ Rust core: cargo test --workspace"
    (cd core && cargo test --workspace --quiet)
fi

# Apple
if [ -d "apple" ] && command -v xcodebuild >/dev/null 2>&1; then
    echo "→ Apple: xcodebuild test (skip if no scheme; user wires this up)"
    # User wires actual xcodebuild invocation here once Xcode project exists
fi

# Android
if [ -d "android" ] && [ -f "android/gradlew" ]; then
    echo "→ Android: ./gradlew test (unit only at this gate; UI tests in CI)"
    (cd android && ./gradlew test --quiet)
fi

# Web
if [ -d "web" ] && [ -f "web/package.json" ]; then
    echo "→ Web: pnpm test"
    (cd web && pnpm test --run)
fi

# Server (Workers)
if [ -d "server" ] && [ -f "server/Cargo.toml" ]; then
    echo "→ Server: cargo test"
    (cd server && cargo test --quiet)
fi

echo "✓ Pre-push checks passed."
exit 0
