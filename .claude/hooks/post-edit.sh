#!/usr/bin/env bash
# Boundless post-edit hook — runs after Claude Code edits a file.
# Auto-formats and runs scoped tests to give the agent immediate feedback.
#
# Wired up in .claude/settings.json under hooks.PostToolUse.

set -euo pipefail

# CLAUDE_FILE_PATHS is set by Claude Code to the edited file paths (newline-separated).
files="${CLAUDE_FILE_PATHS:-}"
[ -z "$files" ] && exit 0

while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in
        *.rs)
            command -v rustfmt >/dev/null 2>&1 && rustfmt --edition 2021 "$f" || true
            # Run scoped tests for the touched crate
            crate_dir=$(dirname "$f" | sed -n 's|^\(core/[^/]*\)/.*$|\1|p')
            if [ -n "$crate_dir" ] && [ -d "$crate_dir" ]; then
                (cd "$crate_dir" && cargo test --quiet 2>&1 | tail -20) || true
            fi
            ;;
        *.swift)
            command -v swiftformat >/dev/null 2>&1 && swiftformat --quiet "$f" || true
            ;;
        *.kt)
            command -v ktlint >/dev/null 2>&1 && ktlint --format "$f" >/dev/null 2>&1 || true
            ;;
        *.ts|*.tsx|*.js|*.jsx|*.svelte)
            (cd web && command -v pnpm >/dev/null 2>&1 && pnpm exec prettier --write "../$f" >/dev/null 2>&1) || true
            ;;
        *.md)
            # No formatter; skip
            ;;
    esac
done <<< "$files"

exit 0
