#!/usr/bin/env bash
# Post-deploy smoke for the boundless-admin-web edge — spec 009 T12 (docs/runbooks/deploy-admin-web.md
# step 6). The admin-web analog of smoke-deployed-edge.sh. Written + lint-clean here; RUN live at T13.
#
# A pure-curl smoke can't perform a WebAuthn passkey ceremony (it needs a real browser + authenticator),
# so this script does every curl-able live assertion itself AND — opt-in — shells out to a Chromium
# CDP-virtual-authenticator Playwright leg (web/tests/smoke/deployed-ceremony.spec.ts) for the full AC10
# chain. The opt-in blocks mirror smoke-deployed-edge.sh's opt-in cross-tenant block.
#
# Usage:
#   bash scripts/smoke-deployed-admin-web.sh https://boundless-admin-web.<account>.workers.dev
#   # full AC10 passkey flow (seed a fresh invite first: scripts/seed-admin-invite.sh):
#   SMOKE_INVITE_TOKEN=<fresh token> DEPLOYED_CEREMONY=1 \
#     bash scripts/smoke-deployed-admin-web.sh https://boundless-admin-web.<account>.workers.dev
#
# Always asserts: /admin/signin reachable · an unauthenticated /admin/members → 307 /admin/signin
# (fail-closed) · Referrer-Policy: no-referrer on the invite route (AC11/F13) · every /api/test/* → 404
# (AC5 edge probe) · no response body echoes a connection-string/secret (P2). On a deployed (non-local)
# host it also asserts the WebAuthn rpId is the deployed host, not localhost, over HTTPS (AC11).
# Opt-in: a live invite-resolve round-trip (SMOKE_INVITE_TOKEN), the passkey ceremony (DEPLOYED_CEREMONY=1),
# and a ≥2-Group cross-tenant invite-resolve isolation probe (CROSS_TENANT_INVITE_TOKEN, AC14 edge leg).
set -euo pipefail

BASE="${1:-}"
if [ -z "$BASE" ]; then
  echo "usage: smoke-deployed-admin-web.sh <base-url>   (e.g. https://boundless-admin-web.<account>.workers.dev)" >&2
  exit 2
fi
BASE="${BASE%/}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

fail() { echo "❌ SMOKE FAILED: $1" >&2; exit 1; }

# No body may echo a Postgres connection string or a credential marker (P2). Explicit `if` so a no-match
# `grep` (exit 1) doesn't trip `set -e`.
assert_no_leak() { # $1=label $2=body
  if printf '%s' "$2" | grep -Eiq 'postgres(ql)?://|bypassrls|authorization: *bearer'; then
    fail "$1 response leaked a connection-string/credential substring (P2)"
  fi
}

# The host (no scheme/port/path) — used for the rpId equality check + the local-vs-deployed gating.
HOST="${BASE#*://}"; HOST="${HOST%%/*}"; HOST="${HOST%%:*}"
case "$HOST" in localhost|127.0.0.1|::1|0.0.0.0) IS_LOCAL=1 ;; *) IS_LOCAL=0 ;; esac

# 1. reachable — `--retry --retry-connrefused` doubles as a readiness wait for a just-deployed Worker.
curl -fsS --retry 10 --retry-connrefused --retry-delay 1 -o /dev/null "$BASE/admin/signin" \
  || fail "/admin/signin did not return 2xx"

# 2. fail-closed — an unauthenticated (app) route redirects to sign-in (the §10-F gate), never serves it.
out="$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' "$BASE/admin/members")" \
  || fail "/admin/members request did not complete"
code="${out%% *}"; loc="${out#* }"
[ "$code" = "307" ] || fail "unauthenticated /admin/members returned HTTP ${code}, expected a 307 redirect (fail-closed gate)"
case "$loc" in */admin/signin) : ;; *) fail "unauthenticated /admin/members redirected to '${loc}', expected …/admin/signin" ;; esac

# 3. Referrer-Policy: no-referrer on the invite route (AC11/F13) — the single-use token rides in this URL
# path, so the browser must not leak it via Referer. An unknown token still renders 200 (InviteExpired).
hdrs="$(curl -fsS -D - -o /dev/null "$BASE/admin/onboard/smoke-referrer-probe")" \
  || fail "/admin/onboard/<probe> did not return 2xx"
printf '%s' "$hdrs" | grep -iq '^referrer-policy: *no-referrer' \
  || fail "/admin/onboard route is missing 'Referrer-Policy: no-referrer' (AC11/F13)"

# 4. AC5 edge probe — every dev-only /api/test/* seam is tree-shaken to a 404 in the prod build. POST hits
# the handler (a GET would be a 405); a live seam would be a 200/4xx-other, an I11 bypass.
for seam in reset seed-invite seed-session seed-member; do
  scode="$(curl -sS -o /dev/null -w '%{http_code}' -X POST "$BASE/api/test/${seam}")" \
    || fail "/api/test/${seam} probe did not complete"
  [ "$scode" = "404" ] || fail "/api/test/${seam} returned HTTP ${scode}, expected 404 — a dev seam is LIVE in prod (I11/AC5)"
done

# 5. WebAuthn rpId — deployed host only (a local wrangler-dev dry-run is http://localhost, where rpId IS
# localhost). The sign-in GET mints assertion options carrying rpId; assert it is the deployed host, never
# localhost, over HTTPS (AC11).
if [ "$IS_LOCAL" = "0" ]; then
  case "$BASE" in https://*) : ;; *) fail "a deployed base must be HTTPS (got '${BASE}') — WebAuthn requires a secure origin (AC11)" ;; esac
  opts="$(curl -fsS "$BASE/api/admin/auth/signin")" || fail "GET /api/admin/auth/signin (assertion options) did not return 2xx"
  assert_no_leak "/api/admin/auth/signin" "$opts"
  rpid="$(printf '%s' "$opts" | sed -n 's/.*"rpId":"\([^"]*\)".*/\1/p')"
  [ -n "$rpid" ] || fail "sign-in options carried no rpId (got: ${opts})"
  [ "$rpid" != "localhost" ] || fail "WebAuthn rpId is 'localhost' on a deployed host — RP_ID is misconfigured (AC11)"
  [ "$rpid" = "$HOST" ] || fail "WebAuthn rpId ('${rpid}') is not the deployed host ('${HOST}') — RP_ID/origin mismatch (AC11/D7)"
  echo "✓ AC11: rpId=${rpid} (not localhost), HTTPS, Referrer-Policy no-referrer."
else
  echo "ℹ rpId/HTTPS checks skipped (local host '${HOST}') — Referrer-Policy + fail-closed + seam-404 still asserted."
fi

echo "✓ deployed-admin-web smoke passed (${BASE}): reachable · unauth→signin · Referrer-Policy · /api/test/* 404."

# 6. Live invite-resolve round-trip (OPT-IN) — proves the web → Rust-Worker → Neon B1 path. A GET mints
# registration options and does NOT consume the invite (consume is the register POST), so this is safe to
# run before the ceremony. SMOKE_INVITE_TOKEN = a fresh scripts/seed-admin-invite.sh token. The token must
# ride in the request URL (it is a path segment), but we pass that URL via a curl config on stdin (`-K -`),
# NOT argv, so the single-use token is not exposed in `ps`/`/proc/<pid>/cmdline`; never echoed to stdout.
if [ -n "${SMOKE_INVITE_TOKEN:-}" ]; then
  itmp="$(mktemp)"; trap 'rm -f "$itmp"' EXIT   # mktemp + EXIT-trap (the smoke-deployed-edge.sh §4 idiom)
  icode="$(printf 'url = "%s/api/admin/auth/invite/%s"\n' "$BASE" "$SMOKE_INVITE_TOKEN" \
    | curl -sS -o "$itmp" -w '%{http_code}' -K -)" || fail "invite-resolve request did not complete"
  ibody="$(cat "$itmp")"
  [ "$icode" = "200" ] || fail "live invite-resolve returned HTTP ${icode}, expected 200 (is the invite seeded + live, and the Worker reachable?)"
  printf '%s' "$ibody" | grep -q '"publicKey"' || fail "invite-resolve 200 body carried no registration options"
  assert_no_leak "invite-resolve" "$ibody"
  echo "✓ live invite-resolve: a seeded invite resolves to registration options through the deployed Worker + Neon."
else
  echo "ℹ invite-resolve skipped — set SMOKE_INVITE_TOKEN=<fresh seed-admin-invite.sh token> to round-trip the B1 path."
fi

# 7. The full passkey ceremony (OPT-IN, DEPLOYED_CEREMONY=1) — the AC10 chain a curl smoke can't do:
# register → sign in → live roster → issue a member → sign out → revoked cookie bounced to /admin/signin.
# Chromium's CDP virtual authenticator drives the REAL deployed routes. Needs a fresh SMOKE_INVITE_TOKEN
# (the register POST consumes it — re-seed for a re-run).
if [ "${DEPLOYED_CEREMONY:-}" = "1" ]; then
  [ -n "${SMOKE_INVITE_TOKEN:-}" ] || fail "DEPLOYED_CEREMONY=1 requires SMOKE_INVITE_TOKEN (a fresh seed-admin-invite.sh token)"
  command -v pnpm >/dev/null 2>&1 || fail "pnpm not found on PATH (needed for the Playwright ceremony leg)"
  ( cd "${ROOT}/web" && DEPLOYED_BASE="$BASE" DEPLOYED_INVITE_TOKEN="$SMOKE_INVITE_TOKEN" \
      pnpm exec playwright test --config playwright.deployed.config.ts ) \
    || fail "the deployed passkey ceremony leg failed (web/tests/smoke/deployed-ceremony.spec.ts)"
  echo "✓ AC10 ceremony: register → sign in → live roster → issue a member → sign out (real passkey, deployed routes)."
else
  echo "ℹ ceremony leg skipped — set DEPLOYED_CEREMONY=1 + SMOKE_INVITE_TOKEN=<fresh seed token> to run the full passkey flow."
fi

# 8. Cross-tenant invite-resolve isolation (OPT-IN, AC14 edge leg) — a token seeded in ANOTHER Group must
# be invisible to this single-install Worker (GROUP_ID-scoped, non-BYPASSRLS boundless_app role): it
# resolves as not-found (410), exactly like a never-issued token — no existence oracle. Mirrors
# smoke-deployed-edge.sh §4. CROSS_TENANT_INVITE_TOKEN = a token seeded in a SECOND Group.
if [ -n "${CROSS_TENANT_INVITE_TOKEN:-}" ]; then
  # Token URL via stdin config (`-K -`), not argv (see §6) — keep the cross-tenant token out of `ps`.
  xcode="$(printf 'url = "%s/api/admin/auth/invite/%s"\n' "$BASE" "$CROSS_TENANT_INVITE_TOKEN" \
    | curl -sS -o /dev/null -w '%{http_code}' -K -)" \
    || fail "cross-tenant invite-resolve request did not complete"
  [ "$xcode" = "410" ] || fail "cross-tenant invite-resolve returned HTTP ${xcode}, expected 410 — a Group-B token must be invisible to the GROUP_ID-scoped Worker (RLS isolation may be broken; is the role NOBYPASSRLS?)"
  echo "✓ cross-tenant isolation (AC14): a second Group's invite token is invisible (410) to this Worker."
else
  echo "ℹ cross-tenant check skipped — set CROSS_TENANT_INVITE_TOKEN with a token seeded in a SECOND Group (docs/runbooks/deploy-admin-web.md)."
fi
