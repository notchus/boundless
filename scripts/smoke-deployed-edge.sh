#!/usr/bin/env bash
# Post-deploy smoke for the boundless-worker edge — spec 001 T07-shell-B (docs/runbooks/deploy-worker.md
# step 7). Also runnable account-free against a local `wrangler dev` (honors the [[hyperdrive]]
# localConnectionString → real local PG). The endpoint contracts it checks are the same ones the
# committed miniflare suite (server/test/worker.spec.ts) proves; this is the operator's live gate.
#
# Usage:
#   bash scripts/smoke-deployed-edge.sh https://boundless-worker.<account>.workers.dev
#   bash scripts/smoke-deployed-edge.sh http://localhost:8787      # against `wrangler dev`
#
# Asserts: /healthz 2xx + status:ok · /readyz db:"ok" (parsed from the JSON body — /readyz is always
# HTTP 200, so status alone proves nothing) · POST /api/auth/signin with a valid-E.164 not-on-file
# phone → AUTH_PHONE_NOT_ON_FILE (the correct empty-DB answer until issuance, spec 008) · and that NO
# response body echoes a connection string / credential substring (P2).
set -euo pipefail

BASE="${1:-}"
if [ -z "$BASE" ]; then
  echo "usage: smoke-deployed-edge.sh <base-url>   (e.g. https://boundless-worker.<account>.workers.dev)" >&2
  exit 2
fi
BASE="${BASE%/}"

fail() { echo "❌ SMOKE FAILED: $1" >&2; exit 1; }

# No body may echo a Postgres connection string or a credential marker (P2). Use an explicit `if` so
# a no-match `grep` (exit 1) doesn't trip `set -e`.
assert_no_leak() { # $1=label $2=body
  if printf '%s' "$2" | grep -Eiq 'postgres(ql)?://|password|bypassrls'; then
    fail "$1 response leaked a connection-string/credential substring (P2)"
  fi
}

# 1. liveness — `--retry --retry-connrefused` doubles as a readiness wait (e.g. a just-started
# `wrangler dev` still booting workerd); harmless against an already-up deployed edge.
health="$(curl -fsS --retry 10 --retry-connrefused --retry-delay 1 "$BASE/healthz")" || fail "/healthz did not return 2xx"
printf '%s' "$health" | grep -q '"status":"ok"' || fail "/healthz missing status:ok"
assert_no_leak "/healthz" "$health"

# 2. readiness — parse the JSON `db` field (NOT the HTTP status, which is always 200)
ready="$(curl -fsS "$BASE/readyz")" || fail "/readyz unreachable"
printf '%s' "$ready" | grep -q '"db":"ok"' || fail "/readyz db not ok (got: ${ready})"
assert_no_leak "/readyz" "$ready"

# 3. sign-in over the real store — empty DB ⇒ AUTH_PHONE_NOT_ON_FILE (no existence leak)
signin="$(curl -fsS -X POST "$BASE/api/auth/signin" \
  -H 'content-type: application/json' \
  -d '{"phone":"+15559999999","reported":{"platform":"ios","app_version":"1.2.0"}}')" \
  || fail "/api/auth/signin did not return 2xx"
printf '%s' "$signin" | grep -q 'AUTH_PHONE_NOT_ON_FILE' || fail "sign-in did not return AUTH_PHONE_NOT_ON_FILE (got: ${signin})"
assert_no_leak "/api/auth/signin" "$signin"

echo "✓ deployed-edge smoke passed (${BASE}): /healthz ok · /readyz db:ok · sign-in phone_not_on_file · no credential leak."
