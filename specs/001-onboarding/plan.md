# 001 — Onboarding: Technical Plan

> Plan status: Ready for `/speckit.tasks` — §10 decisions resolved 2026-06-04 (produced via the `architect` + `platform-parity` + `security-auditor` + `test-strategist` subagents)
> Spec: [`spec.md`](./spec.md) (Clarified) · ADRs: 0001 (Rust core), 0014 (server-driven config), 0015 (admin invite channel), 0016 (auth model)
> Scope: auth model + first-launch for Rider, Driver, Admin. **Member-management UI is spec 008 and out of scope** here.

> **Greenfield note.** The implementation trees (`core/`, `server/`, `apple/`, `android/`, `web/`, `api/`, `fixtures/`) do **not** exist yet. This is the bottom-of-the-stack slice: it *creates* those paths (following `docs/architecture.md`); it modifies nothing existing. The generated-binding-drift CI check lands as part of this slice.

---

## 1. Constitution gate check

| Principle | How the plan obeys it | Status |
|---|---|---|
| **P1** Accessibility | Four snapshot variants per native screen (AC11); admin web → WCAG 2.2 AA via axe-core (AC11b). | ✅ |
| **P2** No PII in logs | `PhoneNumber`/`OnboardingCode`/`RecoveryCode`/`DeviceToken` are tainted newtypes (no `Debug`/`Display`, only `redacted_summary()`); all Worker logging via `boundless::logging::emit()`; scrubber replay (AC3). | ✅ |
| **P4** Rust core source of truth | All token/session/code/version logic in `core::auth`; clients generated via UniFFI; no hand-rolled duplicates. | ⚠️ one carve-out — see §10-A (browser-native WebAuthn on admin web). |
| **P5** Spec before code | Spec clarified; this plan precedes implementation. | ✅ |
| **P7** Native UI | SwiftUI (Rider/Driver), Compose, SvelteKit admin; no shared UI (AC1). | ✅ |
| **P8** i18n | All onboarding strings from catalog; pseudo-locale render (AC12). | ✅ |
| **P10** Don't surprise the elderly | Maria's flow is helper-run; a lone Rider with an invalid session sees `NeedsReauthHelp`, never a form (AC15). | ✅ |
| **P11** Free/open | Onboarding Code + Recovery Code avoid a paid SMS/email gateway; Email Workers is first-party Cloudflare. | ✅ |
| **P12** Operability | PII-free structured logging + OTel on every auth transition; stable error codes in `docs/error-codes.md` (must be created — task in §9). | ⚠️ `docs/error-codes.md` does not exist yet. |
| **P13** Updates invisible | O3 auto-update step (AC5), O4 degradation (AC7/AC8), O8 no rider-surface prompt. | ✅ |

---

## 2. Where each piece lives

**Rust core (`core/`, ADR-0001 / architecture §1) — the source of truth (P4):**
- `core/domain` — tainted/value types: `PhoneNumber`, `DeviceToken` (per architecture §1) + new `OnboardingCode`, `RecoveryCode`, `MemberId`, `Role`, `AccessToken`/`RefreshToken`, `ClientVersion`, `Platform`. Codes are tainted (no `Debug`/`Display`, P2).
- `core/auth` (new crate) — the device-side state machine, Onboarding/Recovery-code request shaping + result interpretation, version comparison (O4), session/refresh-rotation client logic, token-binding tuple `(member_id, platform, app_version)` (I4). Pure logic + injected `Clock`/RNG (no `SystemTime::now`).
- `core/crypto` — phone HMAC-SHA256 + constant-time compare (I3, AC3); code hashing at rest; libsodium manifest signature **verification** (ADR-0014, AC10).
- `core/sync` — proto-derived WS open-handshake types carrying `client_min_version` + `client_recommended_version` (AC7).
- `core/server` — Worker entry points + `GroupHub` DO auth methods.
- `core/ffi-swift`, `core/ffi-kotlin` — UniFFI bindings; clients hold **no** hand-rolled auth logic (P4). `core/ffi-wasm` — validation only for admin web.

**Cloudflare server (`server/`, workers-rs):**
- `/api/auth/*` — sign-in (phone lookup), device-bind (Onboarding Code), token refresh, driver recovery re-bind. Every response carries `client_min_version` + `client_recommended_version` + manifest pointer (O4, ADR-0014).
- `/api/dev/*` — developer-only, hardware-key-backed Admin creation (I11). Mints pending Admin + invite token.
- `/api/admin/auth/*` — WebAuthn registration (consumes invite token) + assertion sign-in (ADR-0016 D4). **Server-side** WebAuthn verification lives here, not in the core.
- **Validation home:** Onboarding-Code single-use/TTL/rate-limit and refresh rotation are validated against **server time** (so binding can't complete offline; wrong-clock edge case). The `GroupHub` DO holds per-Group rate-limit state + token invalidation (I4); persistent rows go to Postgres via Hyperdrive.
- **Email Workers** delivers the admin registration link (ADR-0015). **Queues** carry the rate-limited below-min-version alert (O4/AC8) and the "notifications not enabled" non-PII flag (AC14). **KV** holds the signed manifest + per-Group `{adminName}` for the degradation screen (OQ7). **Turnstile** on the auth boundary (architecture §211) blunts automated code-guessing.

**Native clients:** SwiftUI Rider/Driver (`apple/BoundlessRider`, `apple/BoundlessDriver`) via `BoundlessKit`; Android Compose (`android/rider/app`, `android/driver/app`) via `core-bridge`; SvelteKit admin (`web/`) — registration-link landing, WebAuthn ceremony, `InviteExpired`, WebAuthn sign-in.

---

## 3. Data model & migrations

Numbered SQL under `server/migrations/`. Conventions: PII columns `*_encrypted bytea`; every row `created_at`/`updated_at`/`created_by`; RLS on every PII-bearing table.

| Migration | Table | Key columns / notes |
|---|---|---|
| `0001_groups.sql` | `groups` | id UUID PK, name, audit. FK anchor. |
| `0002_members.sql` | `members` | `id` (MemberId), `group_id` FK, `roles`, `phone_lookup_hash bytea` (indexed, I3), `phone_encrypted bytea` (display-only, I3), RLS. (Issuance writes here → spec 008; this slice defines the columns auth reads.) |
| `0003_onboarding_codes.sql` | `onboarding_codes` | `member_id` FK, `code_hash bytea`, `expires_at` (server TTL), `attempts`, `max_attempts`, `consumed_at`, `created_by`. Single-use + regenerate-invalidates-prior (AC17). |
| `0004_recovery_codes.sql` | `recovery_codes` | Driver-only, `member_id` FK, `code_hash bytea`, `consumed_at`, rotation lineage (AC19). |
| `0005_device_tokens.sql` | `device_tokens` | composite key `(member_id, platform, app_version)` (I4), `token` (never logged), `invalidated_at` (AC4). |
| `0006_sessions.sql` | `sessions` | `member_id`, hashed refresh token, **rotation chain/lineage** (D2), `revoked_at`. Indefinite lifetime; ended only by admin-mediated events (AC18). |
| `0007_admin_webauthn_credentials.sql` | `admin_webauthn_credentials` | `admin_id` FK, `credential_id`, `public_key`, `sign_count`, `aaguid`, **multiple per admin** (AC20), `revoked_at`. |
| `0008_admin_invitations.sql` | `admin_invitations` | pending `admin_id`, `token_hash bytea`, `expires_at` (server TTL), `consumed_at`, single-use (AC16). |
| ~~`0009_admin_alerts.sql`~~ | **Dropped (§10-E):** the below-min-version / notif-not-enabled per-member-per-day rate-limit counter lives in the `GroupHub` DO; alerts emit via Queues; audit via `audit_log`. No dedicated table. |
| `audit_log` | (architecture §6) | this slice adds the obligation that any Admin phone read during issuance/recovery emits an audit event (I5). |

---

## 4. API surface

**OpenAPI 3.1** — source of truth `api/openapi.yaml`; generated clients in `api/generated/<lang>/` (swift-openapi-generator / openapi-generator kotlin / openapi-typescript), **never hand-edited**. `client_min_version` is a **required** field on every `/api/auth/*` response (AC7).

Endpoint sketches (shapes finalized at implementation; field names not final):
- `POST /api/auth/signin` — `{ phone_lookup_hash }` → `{ member_match, client_min_version, client_recommended_version, manifest_pointer }`. No existence leak (voice-and-tone).
- `POST /api/auth/bind-device` — `{ onboarding_code, platform, app_version, … }` → session + `DeviceToken` (AC4, AC17). Server-validated; no offline completion.
- `POST /api/auth/refresh` — refresh-token rotation; silent; carries `client_min_version` (AC18, AC7). **Access token = opaque-random bearer (ADR-0021).**
- `POST /api/auth/recovery/rebind` — Driver: `{ phone_lookup_hash, recovery_code, … }` → re-bind, old token invalidated, fresh code issued (AC19).
- `POST /api/dev/admins` — developer-only, hardware-key-backed; unauth'd **and** admin-auth'd both rejected (AC1, I11).
- `GET /api/admin/auth/invite/{token}` + `POST /api/admin/auth/register` — WebAuthn registration; consumes invite token (AC16, AC20).
- `POST /api/admin/auth/signin` — WebAuthn assertion (AC2, D4).

**Proto / WebSocket** — source of truth `api/boundless.proto` (Buf); the open-handshake message also carries `client_min_version` (AC7). Generated via protoc-gen-swift/-kotlin + ts-proto into `api/generated/<lang>/`; `core/sync` consumes the proto types.

---

## 5. Shared contracts & platform parity (P4)

Every type below is defined **once** in `core/` and *generated*; hand-rolled platform duplicates are a CI failure (ADR-0001).

**P4-generated core types (UniFFI → Swift/Kotlin; never hand-rolled):** `MemberId`, `Role`, `OnboardingCode`, `RecoveryCode`, `PhoneNumber`, `DeviceToken`, `Platform`, `AppVersion` (comparison logic in core, not per platform), `AuthResponse` (carries both version fields + session material), `Session`/`RefreshCredential` (rotation in core), `OnboardingState` (the whole state machine — so all platforms transition identically), the WebAuthn request/response shapes (primary consumer is TS, but shapes originate in core so server + web agree), `ManifestPointer`/`ManifestVersion`. Crypto (phone hash, manifest verify) is single-source in `core/crypto`.

**Generated clients per platform:** iOS = swift-openapi-generator + protoc-gen-swift + `BoundlessKit` XCFramework; Android = openapi-generator(kotlin) + protoc-gen-kotlin + `core-bridge` AAR; Web = openapi-typescript + ts-proto + wasm-bindgen (validation/WebAuthn-shape only). Generated trees never hand-edited.

**Golden fixtures (`fixtures/`, replayed in Rust + Swift + Kotlin, and TS where applicable):** `signin_ok`, `phone_not_on_file`, `device_bind_ok`, `device_bind_invalid_expired`, `below_min_version`, `needs_reauth_help`, `driver_recovery_ok`, `manifest/{verify_ok, verify_fail_with_cache, verify_fail_no_cache, lower_version_ignored}`, `admin_webauthn_register`, `admin_invite_expired`, `compat/{n_minus_1, n_minus_2}`. A new shared type with no fixture, or a fixture not referenced by all required suites, is a parity gap CI must flag.

**Divergence risks → prevention:**
- **DR-1 below-min/`NeedsReauthHelp` decision** — version compare + state decision in core; platforms only render. Prevents a forbidden "Update Now" CTA (O8) or a sign-in form for a Rider (P10). Fixture-backed + per-platform snapshot.
- **DR-2 silent refresh** — rotation in `core::auth`; clients never reimplement. Backs the Maria guarantee (AC18).
- **DR-3 Onboarding-Code bind** — server-validated only; `Offline` is a core-modeled overlay, not a client path (no offline bind).
- **DR-4 manifest verify + tiers** — single libsodium-via-core impl; four manifest fixtures replayed in all four languages.
- **DR-5 binding-artifact versioning** — additive changes ship with regeneration; breaking changes to `AuthResponse`/`Session`/`OnboardingState`/WebAuthn shapes require a version bump **and** an ADR (manifest-schema breaks follow ADR-0014's `manifest:v2:*` rollout).

---

## 6. Security & privacy plan (risk register)

Auditor stance: paranoid. Full register; **R5/R6/R8/R3 carry flags** (accepted-residual or needs-new-invariant-test).

| # | Asset / threat | Invariant | Mitigation | Test |
|---|---|---|---|---|
| R1 | Phone logged on auth/error path | I3, P2/I10 | tainted `PhoneNumber`; `emit()` only; no request-body logging; error paths don't echo input | AC3 scrubber replay incl. `PhoneNotOnFile`/`BindingFailed`/offline branches |
| R2 | Lookup hash compared non-constant-time (membership oracle) | I3 | constant-time comparator in core; HMAC secret in Secrets Store | `i3_phone_lookup_constant_time` |
| R3 | Device token not invalidated on re-onboard/auth-change/**delete** | I4, I12 | invalidate prior token on bind; revoke/logout; delete drops all tokens | `i4_tokens_invalidated_on_reonboarding` (distinct from `…_on_logout`); **+ named I12-suite test for the delete leg (flag)** |
| R4 | Onboarding Code brute-force/replay/offline-bypass/clock-spoof | ADR-0016 D1, I4 | single-use, short-TTL, rate-limited, **server-time** validated; regenerate invalidates prior; Turnstile | AC17 |
| **R5** | **Indefinite refresh token theft (no forced expiry) — accepted residual per ADR-0016** | ADR-0016 D2, I4 | **must implement all 3 compensating controls:** refresh-token rotation, device binding (I4), working admin revoke. Secure at-rest storage (Keychain / Android secure store / server-side session — **pin in §10-F**). If any control is descoped, the residual leaves ADR-0016's accepted envelope → revisit the ADR. | AC18 + **add refresh-rotation-replay test (R6)** |
| **R6** | Refresh rotation/replay has **no enforced invariant test** — the sole control behind R5 | ADR-0016 D2 (underwrites I4) | server invalidates prior refresh on each rotation; replay kills the session family | **FLAG: add named test `auth_refresh_rotation_replay_detected`; consider adding the invariant to `privacy-invariants.md` (implementing test same PR)** |
| R7 | Driver Recovery Code capture/replay/self-serve hijack | ADR-0016 D3, I4 | single-use, server-validated, rotated on use; tainted (never logged); re-bind endpoint reachable only for Driver role; re-bind invalidates old token | AC19 (+ reused-code-rejected) |
| **R8** | Forgetting (I12) coverage for the new auth artifacts | I12, ADR-0016 D2 | `forget_member` must drop phone hash+ciphertext, all tokens, sessions/refresh, outstanding codes; admin delete revokes WebAuthn creds; audit retained PII-redacted | **FLAG: extend I12 property test to assert these are unrecoverable/inert** |
| R9 | Admin invite link interception (email) | I11 (per ADR-0015) | ADR-0015's 6 constraints: dev-minted, single-use, short-TTL server-time, no PII/credential in transit, registration-only; interceptor must complete with own authenticator → detectable | AC16 (+ assert no PII/credential in email body) |
| R10 | Dev/admin-creation endpoint authz | I11 | hardware-key-backed dev auth; no signup surface on any client | AC1 (a) endpoint authz, (b) per-platform no-signup-route |
| R11 | Admin WebAuthn UV-required/no-attestation tradeoff | ADR-0016 D4, I11 | UV required (reg + assert); `attestation:none`; passkeys or hardware keys; resident creds preferred; multiple creds; recovery = Developer re-invite revoking prior. Consider server *rejecting* assertions without the `uv` flag. | AC20, AC2 |
| R12 | Below-min admin alert flood / PII in alert | O4, I8/P2 | exactly one alert/member/day via Queues; payload non-PII (version + opaque id); `{adminName}` from KV manifest, not auth response | AC8, AC15 (+ scrub alert payload) |
| R13 | Third-party tracker via onboarding/WebAuthn/push deps | I8, P11 | no third-party analytics/SDK; self-hosted OTel; standard WebAuthn APIs; no SMS/email provider | AC13 network allow-list against lock files |

**Flagged for their own privacy-invariant test:** R6 (refresh rotation), R8 (forgetting coverage), R3 (delete-leg token invalidation).

---

## 7. Test strategy

**Pyramid.** Bulk of catching power = Rust property/unit in `core::auth` (token/session/code invariants are properties, not examples). UI snapshots cover the a11y bar. Integration covers the server contract. E2E capped at the happy path (≤3).

**Levels:** Rust unit+`proptest` (core::auth) · `insta` serialization snapshots · `swift-snapshot-testing` (iOS) · `Paparazzi` (Compose) · `Playwright`+`axe-core` (admin web) · workers-rs+`sqlx` server integration · compat replay (`server/tests/compat/`, O1) · log-scrubber replay (P2/I10) · XCUITest/Espresso first-launch E2E.

**AC → test map:**

| AC | Level | Concrete test |
|---|---|---|
| AC1 | integration + surface inspection | `ac1_admin_creation_rejects_unauth_and_admin`; `{ios,compose,web}_onboarding_no_signup_route` |
| AC2 | Playwright | `ac2_no_password_field` (virtual authenticator) |
| AC3 | Rust + scrubber | `i3_phone_lookup_constant_time` + scrubber replay |
| AC4 | Rust + integration | `i4_tokens_invalidated_on_reonboarding` |
| AC5 | snapshot ×4 | `AutoUpdateStep` resolves `onboarding.autoupdate.enabled` |
| AC6 | snapshot/inspection | `RiderSettings` — no auto-update toggle |
| AC7 | contract | `ac7_auth_responses_require_min_and_recommended_version` (OpenAPI); `ac7_ws_handshake_has_client_min_version` (proto) |
| AC8 | integration + snapshot | `ac8_below_min_emits_one_alert_per_member_per_day`; `BelowMinVersion` calm-screen snapshot, no CTA |
| AC9 | compat replay | `ac9_auth_endpoints_nminus2` (current + 2 minors) |
| AC10 | core (+per-platform verify) | `ac10_manifest_{verify_fail_with_cache,verify_fail_no_cache,lower_version_ignored,offline_first_launch}` |
| AC11 | snapshot ×4 (iOS+Compose) + VO/TalkBack walkthrough | a11y matrix §below |
| AC11b | Playwright+axe + keyboard | `ac11b_axe_zero_violations[4 routes]`, `ac11b_webauthn_keyboard_only`, reflow 200%/400% |
| AC12 | pseudo-locale render | `pseudo_locale_renders_all_onboarding_screens[zz-ZZ]` |
| AC13 | CI allow-list | `ac13_onboarding_adds_no_third_party` (lock files) |
| AC14 | integration + snapshot | `ac14_decline_records_nonpii_flag_and_advances`; `Permissions.declined` snapshot |
| AC15 | integration + snapshot | `ac15_invalidated_rider_alert_once_per_day`; `NeedsReauthHelp` no-form snapshot; Driver-variant routes to `PhoneEntry` |
| AC16 | Rust + integration | `i11_admin_invite_token_single_use`; `ac16_invite_expired_routes_and_ttl_server_side` |
| AC17 | Rust property + integration | `prop_onboarding_code_single_use_ttl_ratelimit`; `ac17_regenerate_invalidates_prior` |
| AC18 | Rust property + integration | `prop_session_indefinite_until_admin_event`; `ac18_invalidation_triggers_exactly[revoke,reonboard,delete]` |
| AC19 | Rust + integration | `ac19_driver_recovery_code_rebind_…`; `ac19_rider_has_no_self_serve_recovery` |
| AC20 | integration + Playwright | `ac20_webauthn_requires_uv_no_attestation_multi_credential`; `ac20_register_passkey_and_backup_key` |

**A11y matrix (AC11):** every onboarding screen — `PhoneEntry`, `PhoneNotOnFile`, `DeviceBinding` (+ keyboard-visible variant), `BindingFailed`, `Permissions` (+ declined), `AutoUpdateStep`/enabled, `BelowMinVersion` (both named + name-less strings), `NeedsReauthHelp` (assert no form), Driver re-auth entry, `Offline` overlay — × {default, largest-text, dark, RTL} × {iOS, Compose}. Touch targets ≥44pt/48dp; contrast ≥4.5:1; `Complete` is silent (assert *absence* of an "all set" screen). Persona acceptance walkthrough (VoiceOver + largest-text + dark). Admin web (AC11b) per-route, not four-variant.

**Determinism:** proptest seeds checked in (P9); injected `Clock` (`TestClock`) for all TTL/expiry/idle tests + the wrong-client-clock case; Playwright virtual authenticator for WebAuthn; fixed locale/scale/appearance for snapshots.

**Hard-to-test (scope flags):** AC5/AC6 — the app cannot verify the OS auto-update toggle actually flipped (OS-level); testable assertion is "flow presents step + shows confirmation; Settings omits toggle"; the rest is a manual/E2E checklist. AC11 VO/TalkBack reading order needs the recorded walkthrough, not snapshots. AC8/AC15 "one alert/day" needs `Clock` + Queue stub (not live Queue). AC10 — verify is one core test; "each client invokes it before primary surface" needs a per-platform wiring assertion. AC1 "no signup route" is bounded to the state-machine graph (a rogue route elsewhere wouldn't be caught). AC2/AC20 prove server *policy* (UV-required/no-attestation), not hardware enforcement.

---

## 8. Estimated test counts

~6 Rust property · ~14 Rust unit (core::auth + `core/crypto/tests/invariants.rs`, replayed on iOS/Android via bindings) · ~4 `insta` · ~80 native snapshots (10 screens ×4 ×2) + pseudo-locale · ~12 server integration · 1 compat suite ×3 minors · 1 scrubber suite · ~6 web (Playwright+axe) · ≤3 E2E.

---

## 9. Sequencing (dependency-ordered, bottom-up)

1. **Scaffolding** — verify glossary nouns (done per ADR-0016); **create `docs/error-codes.md`** + seed auth codes (P12); scaffold `core/`, `server/`, `api/`. *Blocks everything.*
2. **Core domain types + fixtures** — tainted/value types + golden JSON in `fixtures/` (P4). *Blocks 3–7.*
3. **`core::crypto` + `core::auth`** — phone HMAC + constant-time (`i3_…`), code hashing, manifest verify (AC10), state machine, version compare, refresh-rotation logic, `i4_tokens_invalidated_on_reonboarding`. (crypto ∥ state-machine after step 2.) *Blocks bindings + server.*
4. **Migrations** `0001`–`0009`. *∥ to step 3 once schema agreed.*
5. **Server endpoints** — `/api/dev/admins`, `/api/auth/*`, `/api/admin/auth/*`; DO rate-limit + token invalidation; Email Workers invite; Queues alerts; compat harness (AC9). *Depends 3+4.*
6. **API contracts + generated bindings** — freeze `openapi.yaml` + `boundless.proto`; generate `api/generated/<lang>/`; build XCFramework/AAR. *Depends 3+5; blocks all UI; contracts sketched ∥ with 5, frozen at its end.*
7. **Per-platform UI** — SwiftUI Rider, SwiftUI Driver, Compose Rider, Compose Driver, SvelteKit admin. **All five parallelizable** once step 6 bindings exist (P7).
8. **Cross-cutting verification** — scrubber replay (AC3), allow-list (AC13), pseudo-locale (AC12), compat (AC9). Last, across all surfaces.

**Hard serialization:** 1→2→3 and the contract-freeze gate at end of step 6 before any UI. **Most parallel:** step 7 (five UIs) and crypto-vs-state-machine within step 3.

---

## 10. Decisions — resolved 2026-06-04 (was: open conflicts)

All §10 items from the initial plan are now resolved.

**A — admin auth conflict → RESOLVED (in-app WebAuthn).** `architecture.md` §4 (Cloudflare Access) vs ADR-0016 D4 (in-app WebAuthn) reconciled in favor of **in-app WebAuthn** against our own credential store. `architecture.md` §4, the trust-boundary table, and the diagram are amended. → **ADR-0017**.

**B — P4 carve-out → RESOLVED (documented).** The admin WebAuthn *ceremony* (browser-native) and *verification* (TS on the edge) live outside `core::auth`; the narrow P4 exception is recorded in **ADR-0017 D4**. All other auth logic stays in `core::auth`.

**C — crypto + WebAuthn crates → RESOLVED (verified via docs-researcher; dryoc pin corrected 0.9.0 → 0.8.0 against crates.io at T01).** **`dryoc` 0.8.0** (latest *published* release; MIT, pure-Rust, wasm32, MSRV 1.89) for all crypto — Ed25519 manifest verify, sealed-box/secretbox PII encryption, constant-time HMAC phone hash; replaces the `sodiumoxide`/`dryoc` TODO (`sodiumoxide` is deprecated, C-FFI, no wasm). Admin WebAuthn verification uses **`@simplewebauthn/server` 13.x** (MIT) on the SvelteKit Cloudflare edge; **`webauthn-rs` rejected** (can't run in Workers wasm — `openssl-sys`). Both pinned in `stack-matrix.md`. (ADR-0017)

**D — concrete values → DECIDED (defaults; confirm at implementation):**
- Onboarding Code: single-use, **TTL 72h**, **rate-limit 5 attempts / 15 min** per member (then lock + admin alert), server-time validated.
- Access token **~15 min**, **opaque-random 32-byte bearer** verified by a constant-time keyed-HMAC store lookup — **wire format DECIDED → ADR-0021** (not EdDSA-JWT; honors the time-independent, family-status-gated revocation model with zero new key-mgmt infra). Refresh token opaque 256-bit, stored HMAC-hashed, **rotated every refresh** with a family/lineage id for replay detection, **indefinite** lifetime (ADR-0016 D2).
- Admin invite token **72h**, single-use (AC16). Recovery Code single-use, **no expiry** (driver-held), rotated on use.

**E — admin-alert storage → DECIDED.** The per-member-per-day rate-limit counter lives in the **`GroupHub` DO** (ephemeral, fast); the alert event is emitted via **Queues**. No dedicated Postgres table — migration `0009_admin_alerts.sql` is **dropped** from §3 (audit entries use `audit_log`).

**F — refresh credential at-rest storage → DECIDED.** Apple: **Keychain**. Android: **EncryptedSharedPreferences / Keystore-backed**. Web admin: **httpOnly, Secure, SameSite=Strict server-side session cookie** (post-assertion). Never `UserDefaults`/`@AppStorage`/`localStorage` (forbidden-patterns). Completes security R5's compensating controls.

**G — three new privacy-invariant tests → DEFERRED to implementation** (P9: the test ships with the code). Recorded in `DEFERRED.md` → Auth / Onboarding: `auth_refresh_rotation_replay_detected`; extend the I12 forgetting test to the auth artifacts; named delete-leg token invalidation.

**H — pgcrypto vs core crypto → DECIDED.** **`core::crypto` (dryoc) owns all PII hashing/encryption and code hashing** (P4/I1/I3). `pgcrypto` is **not** on the PII path. Migrations 0002–0004 store ciphertext/hashes produced by the core.

**I — `core/auth` crate → CONFIRMED.** Added as a first-class crate in `architecture.md` §1.

**J — Critical Alerts capability-upgrade path → DEFERRED.** Interim is standard notifications (OQ6); upgrade when the entitlement lands. Recorded in `DEFERRED.md`.

> **Fallback note (ADR-0017):** `@simplewebauthn`'s Cloudflare Workers support is "unofficially supported"; if it ever breaks, the fallback is a native `webauthn-rs` sidecar — tracked in `DEFERRED.md`, not built now.

---

## 11. References

Spec `specs/001-onboarding/spec.md`; constitution `.specify/memory/constitution.md`; ADRs 0001/0014/0015/0016; `docs/{architecture,stack-matrix,privacy-invariants,operational-invariants,a11y-bar,forbidden-patterns,domain-glossary,voice-and-tone}.md`. Paths this slice creates: `core/{domain,auth,crypto,sync,server,ffi-*}`, `server/{migrations,tests/compat}`, `api/{openapi.yaml,boundless.proto,generated/<lang>}`, `fixtures/`, `apple/`, `android/`, `web/`, and `docs/error-codes.md`.
