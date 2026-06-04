# Shared helpers for the binding-drift gate (ADR-0001).
# Sourced by scripts/generate-bindings.sh and scripts/check-binding-drift.sh so both
# compute the file set and hashes IDENTICALLY. Not meant to be run on its own.

# macOS may not ship `sha256sum`; fall back to `shasum -a 256` (same output format).
if ! command -v sha256sum >/dev/null 2>&1; then
    sha256sum() { shasum -a 256 "$@"; }
fi

# Contract INPUT files (the source of truth): the API contracts + every core Rust source,
# crate manifest, AND the committed Cargo.lock. The lock is included deliberately: a
# dependency bump (e.g. `cargo update`, or T03 activating dryoc) changes what the core
# compiles against — hence the generated UniFFI/wasm surface — even when no .rs/.toml
# changes; hashing the lock makes the drift gate catch that. Build output is excluded.
# Repo-root-relative, sorted, de-duplicated.
bindings_input_files() {
    {
        [ -f api/openapi.yaml ] && echo api/openapi.yaml
        [ -f api/boundless.proto ] && echo api/boundless.proto
        find core -type f \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' \) -not -path '*/target/*'
    } 2>/dev/null | LC_ALL=C sort -u
}

# Generated OUTPUT files (must stay in sync with the inputs; never hand-edited).
bindings_output_files() {
    find api/generated -type f 2>/dev/null | LC_ALL=C sort -u
}

# Read a newline-separated file list on stdin; emit "sha256␠␠path" lines, sorted by path.
bindings_hash_list() {
    local f
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        sha256sum "$f"
    done | LC_ALL=C sort -k2
}
