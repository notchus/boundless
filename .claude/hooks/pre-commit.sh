#!/usr/bin/env bash
# Boundless pre-commit hook — enforces the forbidden-patterns list before allowing a commit.
# Install via: ln -s ../../.claude/hooks/pre-commit.sh .git/hooks/pre-commit
# (or wire up via lefthook / husky if used)

set -euo pipefail

red='\033[0;31m'
yellow='\033[1;33m'
nc='\033[0m'

fail=0

staged_files() {
    git diff --cached --name-only --diff-filter=ACM
}

# 1. No TODO / FIXME / XXX in staged code
violations=$(staged_files | xargs -I{} grep -HnE "TODO|FIXME|XXX" {} 2>/dev/null | grep -v "^docs/" || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: TODO/FIXME/XXX in staged code${nc}"
    echo "$violations"
    fail=1
fi

# 2. No println!/dbg! in Rust outside tests
violations=$(staged_files | grep '\.rs$' | xargs -I{} grep -HnE 'println!|dbg!' {} 2>/dev/null | grep -v '/tests/' | grep -v '#\[cfg(test)\]' || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: println!/dbg! in non-test Rust code${nc}"
    echo "$violations"
    fail=1
fi

# 3. No print() of common PII type names in Swift
violations=$(staged_files | grep '\.swift$' | xargs -I{} grep -HnE 'print\(.*\b(address|phoneNumber|deviceToken)\b' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: print() of PII-named values in Swift${nc}"
    echo "$violations"
    fail=1
fi

# 4. No console.log in TypeScript/JS/Svelte committed code
violations=$(staged_files | grep -E '\.(ts|tsx|js|jsx|svelte)$' | xargs -I{} grep -Hn 'console\.log' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: console.log in committed code${nc}"
    echo "$violations"
    fail=1
fi

# 5. Hardcoded user-visible strings (heuristic)
# SwiftUI: Text("...") with English-looking content
violations=$(staged_files | grep '\.swift$' | xargs -I{} grep -HnE 'Text\("[A-Z][a-zA-Z ]{3,}"' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${yellow}⚠️  Possible hardcoded user-visible string in Swift (use LocalizedStringKey)${nc}"
    echo "$violations"
    # Warning only — not a hard fail
fi

# 6. Disabled tests
violations=$(staged_files | xargs -I{} grep -HnE '#\[ignore\]|@Ignore|test\.skip|describe\.skip|it\.skip' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: disabled tests in staged code${nc}"
    echo "$violations"
    fail=1
fi

if [ "$fail" = "1" ]; then
    echo ""
    echo -e "${red}Pre-commit failed. Fix the violations above, or open a spec/ADR if there's a legitimate reason.${nc}"
    exit 1
fi

echo -e "✓ Pre-commit checks passed."
exit 0
