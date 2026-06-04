# Auth golden fixtures (spec 001)

Canonical example payloads for the onboarding/auth flow. Authored in **T02**; replayed by
the server (T07/T08) and the per-platform UIs (T11–T15), and swept by the log-scrubber
(T16). Field names track `plan.md` §4 and are finalized at the **T10 contract freeze** —
treat these as the canonical examples the OpenAPI is aligned to, not a frozen wire schema.

These are **test vectors, not real data**: `member_id`s are well-known UUIDs, tokens are
never present as raw secrets (a successful bind is represented by non-secret session
metadata — the refresh wire format is deliberately left open per plan §10-D), and
`phone_lookup_hash` values are opaque placeholders (the plaintext phone never appears —
I3). Error fixtures carry the stable codes registered in `docs/error-codes.md` (P12).

| File | Represents | State / AC | Notes |
|---|---|---|---|
| `signin_ok.json` | Phone matched → proceed to device binding | `PhoneEntry`→`DeviceBinding` · AC7 | Carries `client_min_version` + `client_recommended_version` (O4/O5) and the manifest pointer (ADR-0014). |
| `phone_not_on_file.json` | Lookup miss | `PhoneNotOnFile` · AC1/AC3 | `AUTH_PHONE_NOT_ON_FILE`. **No existence leak** — same version fields as `signin_ok`; the calm copy lives in the catalog, not here. |
| `device_bind_ok.json` | Onboarding Code accepted; session issued; device token registered | `Permissions` · AC4/AC17 | `device_token_registered: true` bound to `(member_id, platform, app_version)` (I4); `access_token_expires_in_secs: 900` (~15 min, plan §10-D). |
| `device_bind_invalid_expired.json` | Onboarding Code bad/expired | `BindingFailed` · AC17 | `AUTH_ONBOARDING_CODE_EXPIRED` (shares the `onboarding.binding.code_invalid` catalog key with the invalid/consumed/rate-limited variants). |
| `below_min_version.json` | Client below `client_min_version` | `BelowMinVersion` · AC8/O4/O8 | `AUTH_BELOW_MIN_VERSION`. `reported_client_version` is below `client_min_version`. The `{adminName}` for the calm screen comes from the KV manifest, **not** this response (security R12). The server emits exactly one admin alert / member / day. |
| `needs_reauth_help.json` | A Rider's previously-valid session was invalidated | `NeedsReauthHelp` · AC15/AC18 | `AUTH_SESSION_INVALIDATED`; `role: rider` → never a sign-in form (a Driver would route to `PhoneEntry`). One admin alert / member / day. |
| `driver_recovery_ok.json` | Driver self-serve re-bind with a Recovery Code | Driver recovery · AC19 | Old device token invalidated (I4); a fresh Recovery Code is issued (`recovery_code_rotated: true`). |
| `admin_webauthn_register.json` | WebAuthn registration ceremony options | `Registering` · AC20/AC2 | ADR-0016 D4 / ADR-0017: `userVerification: required`, `attestation: none`, resident key preferred, ES256/EdDSA/RS256 params; `challenge_ttl_secs: 300` (KV, one-time — ADR-0017 D3). `user.name`/`id` carry no PII. |
| `admin_invite_expired.json` | Reused / expired admin registration link | `InviteExpired` · AC16 | `ADMIN_INVITE_EXPIRED`; recovery is a Developer re-invite. |
