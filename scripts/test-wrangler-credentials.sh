#!/usr/bin/env bash
# Meta-test for check-wrangler-credentials.sh (the gate-convention pattern, sec-audit F6): proves the
# gate ACCEPTS the committed wrangler.toml AND BITES on each forbidden pattern (a committed secret
# value / a non-localhost connection string), so a future weakening of the gate is caught.
set -euo pipefail
cd "$(dirname "$0")/.."

GATE="scripts/check-wrangler-credentials.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0

expect_pass() {
    if bash "$GATE" "$1" >/dev/null 2>&1; then echo "  ✓ pass: $2"; else
        echo "  ✗ EXPECTED PASS: $2" >&2
        fails=$((fails + 1))
    fi
}
expect_fail() {
    if bash "$GATE" "$1" >/dev/null 2>&1; then
        echo "  ✗ EXPECTED FAIL: $2" >&2
        fails=$((fails + 1))
    else echo "  ✓ fail: $2"; fi
}

# The real committed file must pass.
expect_pass server/wrangler.toml "committed server/wrangler.toml"

# A committed value for any root secret must fail.
for key in HMAC_KEY KEK ADMIN_API_SECRET GROUP_KEY; do
    printf '[vars]\n%s = "deadbeefdeadbeef"\n' "$key" >"$tmp/secret.toml"
    expect_fail "$tmp/secret.toml" "committed $key value"
done

# A clean [vars] (GROUP_ID only — not a secret) passes.
printf '[vars]\nGROUP_ID = "00000000-0000-0000-0000-000000000001"\n' >"$tmp/clean.toml"
expect_pass "$tmp/clean.toml" "GROUP_ID-only [vars]"

# A localhost connection string passes; a real Neon host fails.
printf 'localConnectionString = "postgresql://boundless_app:boundless_app@localhost:55432/boundless_test"\n' >"$tmp/local.toml"
expect_pass "$tmp/local.toml" "localhost connection string"
printf 'localConnectionString = "postgresql://u:p@ep-cool-name-123.us-east-2.aws.neon.tech/db"\n' >"$tmp/neon.toml"
expect_fail "$tmp/neon.toml" "non-localhost (Neon) connection string"

# A comment mentioning the secret keys (no assignment) passes.
printf '# HMAC_KEY and KEK and ADMIN_API_SECRET are wrangler secrets\n[vars]\nGROUP_ID = "x"\n' >"$tmp/comment.toml"
expect_pass "$tmp/comment.toml" "comment mentioning the secret keys (no assignment)"

if [ "$fails" -ne 0 ]; then
    echo "✗ $fails meta-test case(s) failed" >&2
    exit 1
fi
echo "✓ check-wrangler-credentials meta-test passed"
