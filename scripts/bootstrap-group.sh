#!/usr/bin/env bash
# Bootstrap a Boundless Group's per-Group encryption key on a deployed/owner Postgres — spec 008
# operator provisioning (docs/runbooks/deploy-worker.md → "bootstrap the Group"). Run ONCE per Group,
# after scripts/provision-neon.sh (which applies the migrations incl. 0009 delegated_keys), and BEFORE
# issuing any member or running the AC16 cross-tenant check. Without it, every admin member endpoint
# fails closed (no Group key → 503; the AC16 detail read can't reach its clean 404).
#
# It drives examples/bootstrap_group_pg.rs, which mints a RANDOM per-Group key, wraps it under your REAL
# KEK (the value you `wrangler secret put KEK`), and writes ONLY the KEK-wrapped blob — `ON CONFLICT DO
# NOTHING`, so it NEVER overwrites an existing key (overwriting would orphan all PII encrypted under it).
# A re-run is safe (idempotent): it reports "already bootstrapped — left untouched".
#
# Connect as the database OWNER (Neon's `neondb_owner`, DIRECT/unpooled endpoint): its BYPASSRLS lands
# the insert under delegated_keys' FORCE RLS. (The runtime `boundless_app` role deliberately cannot.)
#
# Usage:
#   BOOTSTRAP_KEK_HEX=<64-hex> bash scripts/bootstrap-group.sh <owner-url> <group-id> [group-name]
#   # e.g.
#   BOOTSTRAP_KEK_HEX="$KEK" bash scripts/bootstrap-group.sh \
#     "postgresql://neondb_owner:PW@HOST/neondb?sslmode=require" \
#     00000000-0000-0000-0000-000000000001 "St. Mary's"
#
# The KEK is passed via the ENV (never an argv) so it does not appear in `ps`/shell history. It MUST be
# the exact 64-char-hex value the Worker holds as its `KEK` secret, or issuance/decryption will fail.
set -euo pipefail

log() { echo "$@" >&2; }
fail() { echo "✗ $1" >&2; exit 1; }

OWNER_URL="${1:-}"
GROUP_ID="${2:-}"
GROUP_NAME="${3:-Boundless Group}"
KEK_HEX="${BOOTSTRAP_KEK_HEX:-}"

[ -n "$OWNER_URL" ] || { log "usage: BOOTSTRAP_KEK_HEX=<64-hex> bootstrap-group.sh <owner-url> <group-id> [group-name]"; exit 2; }
[ -n "$GROUP_ID" ]  || { log "usage: BOOTSTRAP_KEK_HEX=<64-hex> bootstrap-group.sh <owner-url> <group-id> [group-name]"; exit 2; }
case "$OWNER_URL" in postgres://*|postgresql://*) : ;; *) fail "owner URL must be a postgres:// connection string" ;; esac
# uuid shape (8-4-4-4-12 hex) — the Worker's GROUP_ID binding is a uuid.
case "$GROUP_ID" in
  [0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]-[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]-[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]-[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]-[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]) : ;;
  *) fail "group-id must be a uuid (e.g. 00000000-0000-0000-0000-000000000001)" ;;
esac
# KEK must be exactly 64 hex chars (32 bytes). Validate before doing any work (clearer than a Rust panic).
case "$KEK_HEX" in
  ""|*[!0-9a-fA-F]*) fail "BOOTSTRAP_KEK_HEX must be set to the 64-char-hex KEK (the value you \`wrangler secret put KEK\`)" ;;
esac
[ "${#KEK_HEX}" -eq 64 ] || fail "BOOTSTRAP_KEK_HEX must be exactly 64 hex chars (32 bytes); got ${#KEK_HEX}"

# A redacted scheme://host/db for logging — NEVER the userinfo (P2: the owner password must not reach
# logs). Greedy `.*@` strips ALL userinfo (robust even with '@' in the password); then drop any `?query`.
safe_url="$(printf '%s' "$OWNER_URL" | sed -E 's#^(postgres(ql)?://).*@#\1#; s#\?.*$##')"
case "$safe_url" in
  *-pooler.*) log "⚠ this looks like a POOLED endpoint ('-pooler'). Use the DIRECT (unpooled) owner endpoint if the insert fails." ;;
esac
log "→ bootstrapping Group ${GROUP_ID} on ${safe_url} (as the database owner)"

SERVER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../server" && pwd)"
log "→ building the bootstrap helper…"
( cd "$SERVER_DIR" && cargo build -q -p boundless-server-store --example bootstrap_group_pg ) \
  || fail "failed to build the bootstrap example"
TARGET_DIR="${CARGO_TARGET_DIR:-${SERVER_DIR}/target}"
BIN="${TARGET_DIR}/debug/examples/bootstrap_group_pg"
[ -x "$BIN" ] || fail "built binary not found at ${BIN} (CARGO_TARGET_DIR override?)"

# Run the binary DIRECTLY (not `cargo run`) so its exact exit code reaches us — 0=new, 3=already, else=err.
# The KEK rides in the env (not argv), so it never lands in `ps`.
set +e
BOOTSTRAP_OWNER_URL="$OWNER_URL" \
BOOTSTRAP_GROUP_ID="$GROUP_ID" \
BOOTSTRAP_KEK_HEX="$KEK_HEX" \
BOOTSTRAP_GROUP_NAME="$GROUP_NAME" \
  "$BIN"
code=$?
set -e

case "$code" in
  0) log "✓ Group ${GROUP_ID} bootstrapped — a per-Group key was minted and KEK-wrapped. Member management is ready." ;;
  3) log "✓ Group ${GROUP_ID} already bootstrapped — left untouched (no key overwritten). Nothing to do." ;;
  *) fail "bootstrap failed (exit ${code}). Check the KEK (64-hex, matching the Worker's secret), the owner URL (DIRECT endpoint), and that migrations are applied (run provision-neon.sh)." ;;
esac
