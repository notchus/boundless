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

# 4. cross-tenant isolation (spec 008 T11 / AC16 / sec-audit F5) — OPT-IN. Runs only once the operator has
# seeded a SECOND Group and exports its member id + the admin shared secret. Proves a Group-A admin (this
# single-install Worker's only tenant — the GROUP_ID binding is the RLS tenant; X-Admin-Id is just the I5
# audit actor) cannot read or list a member that exists ONLY in another Group: RLS + the non-BYPASSRLS
# `boundless_app` role hide it. The production analog of test-provision-neon.sh's app-role isolation proof
# and of the worker_cross_tenant_admin_cannot_read_other_group miniflare test. See
# docs/runbooks/deploy-worker.md → "AC16 — cross-tenant isolation check".
#   ADMIN_API_SECRET       — the ADR-0026 admin shared secret (the value you `wrangler secret put`).
#   CROSS_TENANT_MEMBER_ID — a member uuid that exists ONLY in another Group (NOT this Worker's GROUP_ID).
#   X_ADMIN_ID (optional)  — any uuid; defaults below (the audit actor, never a tenant selector).
if [ -n "${ADMIN_API_SECRET:-}" ] && [ -n "${CROSS_TENANT_MEMBER_ID:-}" ]; then
  XADMIN="${X_ADMIN_ID:-00000000-0000-0000-0000-0000000000aa}"
  xtmp="$(mktemp)"
  trap 'rm -f "$xtmp"' EXIT
  # Defense-in-depth (P2/I8): beyond the connection-string check, no admin response may echo the shared
  # secret itself. (Admin bodies are value-free ErrorBody codes / PII-free summaries, so this never fires
  # today — it guards against a future endpoint reflecting a request header.)
  assert_no_secret() { # $1=label $2=body
    if printf '%s' "$2" | grep -qF "$ADMIN_API_SECRET"; then fail "$1 response echoed the admin shared secret (P2/I8)"; fi
  }

  # 4a. read the other Group's member by id → MUST be 404 ADMIN_MEMBER_NOT_FOUND (never the row). No `-f`
  # here: a 404 is the expected, correct answer, so capture the status explicitly instead of erroring on it.
  xcode="$(curl -sS -o "$xtmp" -w '%{http_code}' \
    -H "authorization: Bearer ${ADMIN_API_SECRET}" -H "x-admin-id: ${XADMIN}" \
    "$BASE/api/admin/members/${CROSS_TENANT_MEMBER_ID}")" || fail "cross-tenant detail request did not complete"
  xbody="$(cat "$xtmp")"
  [ "$xcode" = "404" ] || fail "cross-tenant member read returned HTTP ${xcode}, expected 404 — RLS isolation may be broken (is the Worker role NOBYPASSRLS?)"
  printf '%s' "$xbody" | grep -q 'ADMIN_MEMBER_NOT_FOUND' || fail "cross-tenant member read did not return ADMIN_MEMBER_NOT_FOUND (got: ${xbody})"
  assert_no_leak "cross-tenant detail" "$xbody"
  assert_no_secret "cross-tenant detail" "$xbody"

  # 4b. list THIS Group's members → the other Group's member id MUST be absent from the body.
  xlist="$(curl -fsS -H "authorization: Bearer ${ADMIN_API_SECRET}" -H "x-admin-id: ${XADMIN}" \
    "$BASE/api/admin/members")" || fail "cross-tenant list request did not return 2xx"
  if printf '%s' "$xlist" | grep -qF "$CROSS_TENANT_MEMBER_ID"; then
    fail "the member list LEAKED a cross-tenant member id — RLS isolation is broken"
  fi
  assert_no_leak "cross-tenant list" "$xlist"
  assert_no_secret "cross-tenant list" "$xlist"

  echo "✓ cross-tenant isolation (AC16): a Group-A admin cannot read or list the seeded Group-B member."
else
  echo "ℹ cross-tenant check skipped — set ADMIN_API_SECRET + CROSS_TENANT_MEMBER_ID with ≥2 Groups seeded (docs/runbooks/deploy-worker.md → AC16)."
fi
