# Boundless Privacy Invariants

> Each invariant below is enforced by code. Comments are not invariants. Tests are.
>
> Numbering is permanent — never renumber, only deprecate.

---

## I1 — Addresses are encrypted at rest

Every persisted `Address` value is stored encrypted with a per-Group key. The Group key is itself encrypted with the KEK (Key Encryption Key) stored in Cloudflare Secrets Store.

**Enforcement:**
- DB schema constraint: address columns are `bytea`, not `text`.
- Type-system: `Address::from_db(bytes: &[u8])` requires a `GroupKey` to decrypt.
- Test: `core::crypto::tests::i1_addresses_encrypted` decrypts a fixture and asserts ciphertext != plaintext.

---

## I2 — Plaintext addresses exist only during matching

After the matching call returns, no plaintext address may exist in:
- Durable Object memory (assert via memory snapshot test)
- Worker logs (assert via log scrubber)
- Database (assert via schema constraint I1)

**Enforcement:**
- `MatchingContext::compute()` is the only function in the codebase that takes `Vec<Address>` as input.
- `MatchingContext` does not implement `Clone` or `Serialize` (cannot leak).
- `MatchingContext::drop()` zeroes the address vec.
- Test: `core::matching::tests::i2_addresses_dropped_after_match` constructs a context, computes, drops, asserts memory zero.

---

## I3 — Phone numbers are stored hashed for lookup, encrypted for display

The phone number column is two-fold:
- `phone_lookup_hash`: HMAC-SHA256 with a per-instance secret, used only for auth lookups.
- `phone_encrypted`: full phone, encrypted, decryptable only by Admin reads (audit-logged).

**Enforcement:**
- DB columns named exactly as above.
- Test: `core::crypto::tests::i3_phone_lookup_constant_time` verifies constant-time comparison on the hash.

---

## I4 — Device push tokens are scoped per device per app version

Tokens are bound to (Member, Platform, App Version). On any auth change, all tokens for that Member are invalidated.

**Enforcement:**
- Token table has `(member_id, platform, app_version)` composite key.
- `DeviceToken` type wraps `String` with no `Display` impl.
- The `(member_id, platform, app_version)` binding and the admin-mediated invalidation
  triggers (revoke/logout, new-device re-onboarding, deletion) are decided once in
  `core::auth` (`DeviceBinding`, `invalidation_for`, `reonboarding_invalidation`).
- Tests: `core::auth::tests::i4_tokens_invalidated_on_logout` and
  `core::auth::tests::i4_tokens_invalidated_on_reonboarding` (the new-device case, AC4).
- **Refresh-rotation underwrites I4** (ADR-0016 D2): because member sessions are indefinite,
  a stolen refresh credential is bounded not by expiry but by **rotation with replay
  detection** — replaying a rotated-away credential revokes the whole session family. Test:
  `core::auth::tests::auth_refresh_rotation_replay_detected` (the sole enforced control behind
  the no-forced-expiry decision; risk register R5/R6). The delete-leg device-token
  invalidation test ships with the deletion work (I12; tracked in `DEFERRED.md`).

---

## I5 — Admin reads of PII are audit-logged

Every server endpoint that returns PII to an Admin must emit an audit event including: timestamp, admin ID, member ID accessed, fields returned, request ID.

**Enforcement:**
- A `#[require_audit]` attribute macro wraps PII-returning handlers.
- Compile-time check: any handler returning a type that contains `Address` / `PhoneNumber` must have the attribute or fail the build.
- Test: integration test asserts that every PII handler in the OpenAPI spec has a matching audit-log entry.

---

## I6 — Riders never see other Riders' identities or addresses

The Rider app's data model contains only: this Rider's data, this Rider's assigned Driver (name + first name initial), the Approximate Pickup Time, the Doorbell Notification trigger.

**Enforcement:**
- The Rider client API endpoint `/api/rider/me/today` returns a `RiderViewToday` struct with no fields that could leak others' info.
- Other Riders in the same chain appear only as opaque counts ("Daniel will bring you and one other to the gathering").
- Test: snapshot test of the API response asserts no other Rider IDs or names.

---

## I7 — Drivers see Riders' names but not addresses until in-neighborhood

The Driver client API returns `Rider.first_name` and a neighborhood-level location (Geohash precision 6, ~1.2km) until the Driver is within 1 km of the Rider's actual location.

**Enforcement:**
- The `/api/driver/me/today/chain` endpoint returns `RiderForDriver` with `location: NeighborhoodLevel`.
- A second endpoint `/api/driver/me/rider/{id}/precise-location` requires a "I am en route" assertion + the Driver's current location within 1 km.
- Test: integration test asserts the geohash precision at >1km distance.

---

## I8 — No third-party analytics, ads, or trackers

No code from Google Analytics, Mixpanel, Amplitude, Sentry (cloud), Crashlytics, Facebook Pixel, or similar.

**Enforcement:**
- A repository allow-list of network domains, checked in CI against package lock files.
- Self-hosted observability only (OpenTelemetry → Tempo/Mimir/Loki, or Cloudflare Analytics Engine for non-PII).

---

## I9 — The Optional Live Tracker is opt-in by the Driver, E2E encrypted, never reveals Rider address

When the Driver opts in for a specific ride, the Driver's position is encrypted with the Rider's public key client-side and pushed via Cloudflare Workers to the Rider's app. The server cannot decrypt. The Rider's address is never sent to the Driver in this stream.

**Enforcement:**
- The position broadcast endpoint accepts `EncryptedPosition` (opaque bytes).
- The Driver's client never receives the Rider's coordinates — only "approaching."
- Test: integration test asserts the server's persisted records contain only ciphertext.

---

## I10 — Logs are scrubbed before persistence

The logging pipeline runs every log line through a scrubber that detects PII patterns (street-number + word, phone number patterns, email patterns, long opaque hex blobs that look like tokens).

**Enforcement:**
- All logs go through `boundless::logging::emit()` — direct `tracing::info!` is forbidden via lint.
- The scrubber has its own test suite of fixtures including red-team inputs.
- CI step replays the latest run's logs through the scrubber and fails on any redaction (i.e. you should never reach the scrubber with PII to begin with).

---

## I11 — Admin accounts are issued only by the developer

There is no signup form. There is no "request access" link. No member or anonymous caller can obtain or escalate to an Admin role — Admins exist only because the Developer deliberately provisions them.

> **Clarified by ADR-0015 (2026-06-04):** I11 is about *who initiates access* (only the Developer), not *how a registration link is transported*. The original "no email-based invite for Admins" wording is narrowed: the Developer **may** deliver a single-use, short-TTL, developer-minted Admin *registration* link out of band via Email Workers, provided the link carries no PII beyond the opaque token, carries no credential material, is consumed on first successful WebAuthn registration, and only initiates registration against an already-provisioned pending Admin record. Member-initiated or self-serve invites of any kind remain forbidden. This reconciles I11 with `docs/stack-matrix.md` ("Email Workers (admin invites)").

**Enforcement:**
- The Admin creation endpoint requires a developer-only auth header.
- Developer auth is a hardware-key-backed credential (YubiKey or equivalent) verified via WebAuthn.
- The registration link/token is single-use and TTL-bounded, validated server-side; reuse or expiry is rejected. Test: `core::auth::tests::i11_admin_invite_token_single_use` (+ spec 001 AC16).
- Test: integration test asserts unauth'd and admin-auth'd requests to `/api/dev/admins` are both rejected.

---

## I12 — Forgetting is a feature

Riders, Drivers, and Admins can request account deletion. Deletion:
- Removes their PII rows.
- Replaces their ID in historical chains with `Anonymous_NNNN`.
- Keeps audit logs (legal requirement) but with their PII redacted.

**Enforcement:**
- `core::deletion::forget_member(id)` is the only sanctioned path.
- A property test asserts that after `forget`, no query in the codebase can recover the original PII.

---

## Updating this file

- Adding an invariant requires the implementing test in the same PR.
- Removing or weakening an invariant requires an ADR.
- The numbering is permanent; deprecated invariants are marked `**DEPRECATED**` but never deleted.
