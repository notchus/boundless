# Onboarding log fixtures (spec 001)

`log_lines.jsonl` — representative structured **log events** the onboarding/auth flow emits,
one JSON object per line. Authored in **T16** (the cross-cutting verification sweep) and
replayed by the I10 PII scrubber in `core/logging/tests/onboarding_replay.rs` (AC3 / P2).

These are **test vectors, not real logs**. They are PII-free **by construction** — every field
is a non-tainted value:

- opaque `MemberId` / `SessionFamilyId` **UUIDs** (the I12 deletion stand-in — *not* PII; see
  `core/server/src/alerts.rs`),
- **version strings** (`reported_version`, the O5 stragglers signal),
- stable **error codes** from `docs/error-codes.md` (P12),
- `Platform` / `Role` enum wire forms, booleans, and ISO-8601 timestamps.

They carry **no** phone number, Onboarding/Recovery code, device/auth token, email, or address
— exactly the shapes the tainted newtypes (`PhoneNumber`/`DeviceToken`/…) keep out of any log
line at compile time (they are not `Serialize`/`Display`, P2). The replay asserts the scrubber
finds **zero** PII on every line; `core/logging/tests/scrub_redteam.rs` proves the detector
would catch PII if any leaked, so "clean" is meaningful.

## Coverage (branches swept)

| Branch | `event` / `error_code` |
|---|---|
| sign-in proceed / lookup miss | `auth.signin` · `AUTH_PHONE_NOT_ON_FILE` |
| state transition | `onboarding.transition` |
| bind accepted / invalid / expired / consumed | `auth.bind` · `AUTH_ONBOARDING_CODE_{INVALID,EXPIRED,CONSUMED}` |
| rate-limit lock + admin alert | `admin_alert` · `AUTH_ONBOARDING_CODE_RATE_LIMITED` |
| below-min / session-invalidated / notifications-declined alerts | `admin_alert` · `AUTH_{BELOW_MIN_VERSION,SESSION_INVALIDATED,NOTIFICATIONS_NOT_ENABLED}` |
| offline (bind + network) | `AUTH_BIND_REQUIRES_ONLINE` · `NET_OFFLINE` |
| refresh rotate / replay → family-kill | `auth.refresh` · `AUTH_REFRESH_REPLAY_DETECTED` |
| device-token invalidated (silent) | `auth.device` · `AUTH_DEVICE_TOKEN_INVALIDATED` |
| driver recovery rebind / rider not-available | `auth.recovery` · `AUTH_RECOVERY_NOT_AVAILABLE` |
| manifest verify-fail / stale | `manifest.verify` · `MANIFEST_{VERIFY_FAILED,VERSION_STALE}` |
| admin invite / WebAuthn / dev-create | `ADMIN_INVITE_EXPIRED` · `ADMIN_WEBAUTHN_UV_REQUIRED` · `DEV_ADMIN_CREATE_FORBIDDEN` |

> The deployable `boundless::logging::emit()` sink that routes real Worker logs through the
> scrubber, plus the Logpush/latest-run CI replay, land with the Worker runtime (**T07-shell-B**).
