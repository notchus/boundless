#!/usr/bin/env bash
# Boundless pre-commit hook — enforces the forbidden-patterns list before allowing a commit.
set -euo pipefail

red='\033[0;31m'
yellow='\033[1;33m'
nc='\033[0m'
fail=0

staged_files() {
    git diff --cached --name-only --diff-filter=ACM
}

# Only SOURCE-CODE files. TODO / disabled-test rules are about shipped code, not prose —
# markdown, config, shell, and docs are excluded so docs describing these rules don't trip it.
code_files() {
    staged_files | grep -E '\.(rs|swift|kt|kts|ts|tsx|js|jsx|mjs|cjs|svelte|java|go|rb|py)$' || true
}

violations=$(code_files | xargs -r -I{} grep -HnE "TODO|FIXME|XXX" {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: TODO/FIXME/XXX in staged code${nc}"; echo "$violations"; fail=1
fi

violations=$(staged_files | grep '\.rs$' | xargs -r -I{} grep -HnE 'println!|dbg!' {} 2>/dev/null | grep -v '/tests/' | grep -v '#\[cfg(test)\]' || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: println!/dbg! in non-test Rust code${nc}"; echo "$violations"; fail=1
fi

violations=$(staged_files | grep '\.swift$' | xargs -r -I{} grep -HnE 'print\(.*\b(address|phoneNumber|deviceToken)\b' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: print() of PII-named values in Swift${nc}"; echo "$violations"; fail=1
fi

violations=$(staged_files | grep -E '\.(ts|tsx|js|jsx|svelte)$' | xargs -r -I{} grep -Hn 'console\.log' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: console.log in committed code${nc}"; echo "$violations"; fail=1
fi

violations=$(staged_files | grep '\.swift$' | xargs -r -I{} grep -HnE 'Text\("[A-Z][a-zA-Z ]{3,}"' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${yellow}⚠️  Possible hardcoded user-visible string in Swift (use LocalizedStringKey)${nc}"; echo "$violations"
fi

violations=$(code_files | xargs -r -I{} grep -HnE '#\[ignore\]|@Ignore|test\.skip|describe\.skip|it\.skip' {} 2>/dev/null || true)
if [ -n "$violations" ]; then
    echo -e "${red}❌ Forbidden: disabled tests in staged code${nc}"; echo "$violations"; fail=1
fi

if [ "$fail" = "1" ]; then
    echo ""
    echo -e "${red}Pre-commit failed. Fix the violations above, or open a spec/ADR if there's a legitimate reason.${nc}"
    exit 1
fi

echo -e "✓ Pre-commit checks passed."
exit 0
