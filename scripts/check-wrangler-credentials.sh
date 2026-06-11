#!/usr/bin/env bash
# Forbid committed SECRET VALUES in server/wrangler.toml (spec 008 T09 / ADR-0026).
#
# The root secrets — HMAC_KEY (I3), KEK (ADR-0025), ADMIN_API_SECRET (ADR-0026) — and any GROUP_KEY
# must NEVER live in the file: each is a `wrangler secret` at deploy + a test value injected by the
# vitest harness (miniflare.bindings). A real (non-localhost) Postgres connection string must not be
# committed either (the only allowed connection string is the LOCAL TEST `boundless_app@localhost`).
#
# Out of scope: account IDENTIFIERS (the Hyperdrive / KV `id`s) are NOT secrets; their pre-open-source
# genericization is tracked separately (DEFERRED.md → T07-shell-B "genericize the committed ids").
set -euo pipefail
cd "$(dirname "$0")/.."

FILE="${1:-server/wrangler.toml}"
rc=0
fail() { echo "❌ $1" >&2; rc=1; }

if [ ! -f "$FILE" ]; then
    echo "❌ $FILE not found" >&2
    exit 1
fi

# (1) No root secret is ASSIGNED a value (a `KEY = "..."` line). A comment mentioning the key is fine;
#     an assignment is not — these keys must only ever be `wrangler secret`s / injected test bindings.
bad_secret="$(grep -nE '^[[:space:]]*(HMAC_KEY|KEK|ADMIN_API_SECRET|GROUP_KEY)[[:space:]]*=' "$FILE" || true)"
if [ -n "$bad_secret" ]; then
    fail "a root secret is assigned a value in $FILE (must be a 'wrangler secret'): ${bad_secret}"
fi

# (2) No non-localhost Postgres connection string (a real Neon URL committed by mistake). The only
#     allowed connection string is the LOCAL TEST `…@localhost[:port]/…`; anything else is rejected.
bad_conn="$(grep -oE 'postgres(ql)?://[^"[:space:]]+' "$FILE" | grep -vE '@localhost(:|/|$)' || true)"
if [ -n "$bad_conn" ]; then
    fail "a non-localhost Postgres connection string is committed in $FILE: ${bad_conn}"
fi

if [ "$rc" -eq 0 ]; then
    echo "✓ $FILE carries no committed secret values (root secrets are wrangler secrets; only localhost DB creds)."
fi
exit $rc
