# Boundless Error Codes

> Constitution **P12**: "Every error type has a stable error code. Codes are documented
> here." This file is the registry. Adding a new error type requires adding its code here
> in the same PR; the `reviewer` subagent flags error variants with no registered code.

---

## Conventions

- **Stable & append-only.** A code's string never changes once shipped (clients,
  catalogs, runbooks, and dashboards key off it). Deprecated codes are marked
  `**DEPRECATED**` but never deleted or reused.
- **Namespaced** `DOMAIN_SUBJECT_CONDITION` in `SCREAMING_SNAKE_CASE`
  (`AUTH_…`, `ADMIN_…`, `DEV_…`, `MANIFEST_…`, `NET_…`).
- **PII-free (P2/I10).** A code and its structured log fields must never contain a phone
  number, address, token, or other tainted value — only the stable code, an opaque
  request id, and non-PII context (e.g. a version string). Logging is via the PII-free
  `emit()` path, never raw `tracing::*`.
- **Codes are not user copy.** User-facing strings come from the i18n catalog (P8); the
  "Catalog key" column links a code to the calm message a person actually sees. Several
  codes deliberately share a non-leaking message (e.g. lookup miss).
- **`Retryable`** marks whether the same client action can succeed on retry without an
  out-of-band step (admin re-issue, version upgrade, reconnect).

---

## Onboarding & authentication (spec 001)

| Code | Meaning | Maps to (state / AC) | Catalog key | Retryable |
|---|---|---|---|---|
| `AUTH_PHONE_NOT_ON_FILE` | Phone-lookup hash matched no member. **Never reveals whether a number exists** (no existence leak). | `PhoneNotOnFile` · AC1/AC3 | `onboarding.signin.phone_not_on_file` | yes |
| `AUTH_BIND_REQUIRES_ONLINE` | Device-bind attempted with no connectivity; the Onboarding Code is server-validated and cannot complete offline. | `Offline` overlay on `DeviceBinding` · OQ8 | (reuses `onboarding.signin.*`) | yes (on reconnect) |
| `AUTH_ONBOARDING_CODE_INVALID` | Onboarding Code did not match. | `BindingFailed` · AC17 | `onboarding.binding.code_invalid` | yes |
| `AUTH_ONBOARDING_CODE_EXPIRED` | Onboarding Code past its server-side TTL (default 72h). | `BindingFailed` · AC17 | `onboarding.binding.code_invalid` | no (admin re-issue) |
| `AUTH_ONBOARDING_CODE_CONSUMED` | Onboarding Code already used, or superseded by a regenerated code (regenerate invalidates the prior). | `BindingFailed` · AC17 | `onboarding.binding.code_invalid` | no (admin re-issue) |
| `AUTH_ONBOARDING_CODE_RATE_LIMITED` | Too many bind attempts (default 5 / 15 min per member); locked + admin alerted. | `BindingFailed` · AC17 | `onboarding.binding.code_invalid` | yes (after window) |
| `AUTH_BELOW_MIN_VERSION` | Reporting client is below `client_min_version`; calm degradation screen only, no "Update Now". One admin alert/member/day. | `BelowMinVersion` · AC8/O4/O8 | `auth.below_min_version` / `auth.below_min_version_generic` | no (needs upgrade, not rider-actionable) |
| `AUTH_SESSION_INVALIDATED` | A previously-valid session was ended (admin revoke/logout, I4 device change, or deletion). | `NeedsReauthHelp` (Rider) / `PhoneEntry` (Driver) · AC15/AC18 | `auth.below_min_version` (Rider) / `auth.signin_again` (Driver) | Rider: no (admin) · Driver: yes |
| `AUTH_REFRESH_REPLAY_DETECTED` | A rotated (stale) refresh credential was replayed; the whole session family is killed. Backs the no-forced-expiry decision. | AC18 · ADR-0016 D2 · invariant `auth_refresh_rotation_replay_detected` | `auth.below_min_version` (Rider) / `auth.signin_again` (Driver) | no |
| `AUTH_DEVICE_TOKEN_INVALIDATED` | A `(member_id, platform, app_version)` device token was invalidated (re-onboard, logout/revoke, deletion). | AC4 · I4 | — (silent; client re-registers on next bind) | n/a |
| `AUTH_NOTIFICATIONS_NOT_ENABLED` | Notification permission was declined at onboarding (or Critical Alerts is unavailable-because-pending). A non-PII admin flag is recorded (deduped per member per day) and the flow advances — **operational flag, not a client-facing error** (onboarding never blocks/scolds). | AC14 · O8/P10 | — (no client surface) | n/a |
| `AUTH_RECOVERY_CODE_INVALID` | Driver Recovery Code did not match, expired-by-use, or already consumed. | Driver recovery · AC19 | `onboarding.binding.code_invalid` | no (Admin fallback) |
| `AUTH_RECOVERY_NOT_AVAILABLE` | Self-serve recovery attempted for a non-Driver (Riders recover only via Admin re-issue). | AC19 | `auth.below_min_version` | no (admin) |

## Manifest / server-driven config (ADR-0014, spec 001)

| Code | Meaning | Maps to (state / AC) | Catalog key | Retryable |
|---|---|---|---|---|
| `MANIFEST_VERIFY_FAILED` | Manifest libsodium signature verification failed. Falls back per ADR-0014 tiers (cached → bundled). Never blocks the primary surface. | `Complete` / `ManifestFailReturning` · AC10/O2 | — (silent; falls back) | yes (next launch) |
| `MANIFEST_VERSION_STALE` | Fetched manifest `manifest_version` is lower than the cached one; ignored. | AC10 | — (silent) | n/a |
| `NET_OFFLINE` | No connectivity. Shown as the `Offline` overlay on the current sign-in step; the network action is deferred until connectivity, then resumed. | `Offline` · edge cases | (reuses `onboarding.signin.*`) | yes (on reconnect) |

## Admin / developer (spec 001, ADR-0015/0016/0017)

| Code | Meaning | Maps to (state / AC) | Catalog key | Retryable |
|---|---|---|---|---|
| `ADMIN_INVITE_EXPIRED` | Admin registration link past its server-side TTL (default 72h). | `InviteExpired` · AC16 | `admin.onboarding.invite_expired` | no (developer re-invite) |
| `ADMIN_INVITE_CONSUMED` | Admin registration link already used (single-use); consumed on first successful WebAuthn registration. | `InviteExpired` · AC16 | `admin.onboarding.invite_expired` | no (developer re-invite) |
| `ADMIN_WEBAUTHN_UV_REQUIRED` | WebAuthn registration/assertion lacked the user-verification (`uv`) flag; rejected. | AC20 · ADR-0016 D4 | `admin.onboarding.register_credential` | yes |
| `ADMIN_WEBAUTHN_VERIFICATION_FAILED` | WebAuthn registration or assertion verification failed (bad challenge, signature, or unknown credential). | AC2/AC20 | `admin.onboarding.register_credential` | yes |
| `ADMIN_WEBAUTHN_CHALLENGE_EXPIRED` | The KV-held WebAuthn challenge expired (5-min TTL) or was already used (one-time). | AC20 · ADR-0017 D3 | `admin.onboarding.register_credential` | yes |
| `DEV_ADMIN_CREATE_FORBIDDEN` | A non-Developer (unauthenticated **or** admin-authenticated) called the Admin-creation endpoint. Admins are issued only by the Developer (I11). | AC1 · I11 | — (no client surface; there is no signup) | no |

## Admin member-management — issuance (spec 008, ADR-0025)

| Code | Meaning | Maps to (state / AC) | Catalog key | Retryable |
|---|---|---|---|---|
| `ADMIN_MEMBER_PHONE_INVALID` | The submitted phone could not be normalized/validated to E.164 (single-source `normalize_phone`). The submitted value is never echoed or logged (P2). | issuance / edit validation · AC1/AC11 | `admin.member.phone_invalid` | yes |
| `ADMIN_MEMBER_ADDRESS_INVALID` | The submitted address failed validation (empty / unparseable). The submitted value is never echoed or logged (P2). | issuance / edit validation · AC1/AC11 | `admin.member.address_invalid` (UI copy authored at T10) | yes |
| `ADMIN_MEMBER_ROLES_REQUIRED` | No role was selected — a member must hold at least one role (Rider, Driver, or both). Issuance with an empty role set, or an edit that would clear all roles, is rejected before any write. | issuance / edit validation · AC13 | `admin.member.roles_required` (UI copy authored at T10) | yes |
| `ADMIN_MEMBER_DUPLICATE_PHONE` | The submitted phone is already enrolled in this Group (`(group_id, phone_lookup_hash)` unique). The existing member is surfaced and linked — an I5-audited, **admin-surface-only** read; this disclosure is never reused on a member-facing endpoint (the no-existence-leak discipline holds on `/api/auth/*`). | duplicate-phone edge · AC1 · I5 | `admin.member.duplicate_phone` | no (resolve by editing the existing member) |
| `ADMIN_MEMBER_EDIT_STALE` | Optimistic-concurrency reject: the member's `updated_at` changed since the edit was loaded (a concurrent Admin edit). No partial write. | concurrent-edit edge · AC11 | `admin.member.edit_stale` | yes (after refresh) |
| `ADMIN_MEMBER_ROLE_FORBIDDEN` | A member-issuance request named the `Admin` role. Admins are issued **only** by the Developer (I11); the issuable role set is Rider/Driver (`Admin` is unrepresentable in the issuance input — the wire `Vec<Role>` → issuable-roles conversion refuses it). Distinct from `DEV_ADMIN_CREATE_FORBIDDEN` (the `/api/dev/*` surface). | issuance role validation · AC10 · I11 | — (no client surface; the admin UI offers no Admin option) | no (the role cannot be issued here) |
| `ADMIN_GROUP_KEY_MISSING` | Member issuance attempted with no per-Group encryption key (Group bootstrap incomplete — ADR-0025). Fails closed — no member row is written and an address is never stored unencrypted. | Group-key-missing edge · AC12 · I1 | — (no client surface; an operator bootstraps the key — see `docs/runbooks/key-management.md`) | no (operator) |

---

## Updating this file

- Adding an error type requires registering its code here in the same PR (P12).
- Codes are permanent and append-only; deprecate, never delete or reuse.
- Each alertable code should have (or reference) a runbook under `docs/runbooks/` (P12);
  runbooks are added alongside the alerting that fires them.
