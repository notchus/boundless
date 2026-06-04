# 001 — Onboarding: Task List

> Tasks status: Draft (produced by `/speckit.tasks`, 2026-06-04)
> Spec: [`spec.md`](./spec.md) (Clarified) · Plan: [`plan.md`](./plan.md) (§10 resolved) · ADRs: 0001, 0014, 0015, 0016, 0017
> Contract: this file is what the `/speckit.implement` gate consumes. Anything not here is scope creep (P6). Every task maps to ≥1 spec AC (closes or enables).

> **Greenfield:** all trees (`core/`, `server/`, `api/`, `apple/`, `android/`, `web/`, `fixtures/`) are created by this slice. One task = one PR-sized slice. Each lists: **Does / Touches / Closes (AC) / Tests / Blocked by / ∥**.

## Dependency waves (parallelism map)

```
Wave 0:  T01 (scaffold)
Wave 1:  T02 (core domain types + fixtures)
Wave 2:  T03 (core::crypto)        ∥  T06 (migrations)
Wave 3:  T04 (auth codes + state)  ∥  T05 (session + refresh)
Wave 4:  T07 (member-auth endpoints) ∥ T08 (dev admin-create + invite + compat)
Wave 5:  T09 (admin WebAuthn — edge TS)
Wave 6:  T10 (API contracts + generated bindings)  ← CONTRACT-FREEZE GATE
Wave 7:  T11 ∥ T12 ∥ T13 ∥ T14 ∥ T15  (five UIs)
Wave 8:  T16 (cross-cutting verification)
```

**Hard serialization:** T01→T02→T03 and the T10 contract-freeze before any UI. **Most parallel:** Wave 7 (five UIs) and the within-wave pairs.

---

## Wave 0 — scaffolding

### T01 — Repo scaffolding, error-codes registry, CI drift gate — ✅ DONE (2026-06-04)
- **Does:** Create the workspace skeletons (`core/` crates, `server/`, `api/`, `fixtures/`); create `docs/error-codes.md` and seed the onboarding/auth error codes (P12); stand up the CI gate that (a) rejects hand edits to `api/generated/**` and (b) fails if `core/`/`api/` change without regenerated bindings (ADR-0001). Pin `dryoc 0.8.0` (corrected from 0.9.0 — see below) and `@simplewebauthn/server 13.x` in the manifests per `stack-matrix.md`.
- **Touches:** repo root, `core/*/Cargo.toml`, `server/`, `api/`, `fixtures/`, `docs/error-codes.md`, CI workflow, `rust-toolchain.toml` (dryoc needs Rust ≥1.89 — verify).
- **Closes/enables:** enables all; partially **AC13** (allow-list CI scaffold).
- **Tests:** CI dry-run green; binding-drift check fails on a deliberate ungenerated change (meta-test).
- **Blocked by:** —  · **∥:** no (blocks everything).
- **Done (2026-06-04):** `core/` workspace (8 skeleton crates) + `server/` + `api/` (placeholder `openapi.yaml`/`boundless.proto` + committed `generated/` tree) + `fixtures/` + `web/package.json`; `rust-toolchain.toml` → 1.95.0. `docs/error-codes.md` seeded (P12). Binding-drift gate = `scripts/{_bindings_common,generate-bindings,check-binding-drift,test-binding-drift}.sh` + `api/.bindings.lock`; network allow-list (AC13 scaffold) = `scripts/check-network-allowlist.sh` + `ci/{forbidden-trackers,network-allowlist}.txt`; wired in `.github/workflows/ci.yml`. **dryoc pin corrected `0.9.0` → `0.8.0`** (0.9.0 unpublished on crates.io) across `stack-matrix.md`/`plan.md`/this file/`DEFERRED.md`; `@simplewebauthn/server 13.3.1` pinned. All gates verified green locally (`cargo fmt/clippy/test --locked`, drift meta-test, allow-list across 3 lock files incl. `web/pnpm-lock.yaml`); `reviewer` findings H1/M2/M3/L1 fixed, M1 resolved (web lock now committed + scanned). CI itself runs on GitHub Actions (not triggerable locally).

---

## Wave 1 — core types

### T02 — Core domain & value types + golden fixtures — ✅ DONE (2026-06-04)
- **Does:** Define the P4-generated types in `core/domain`: `MemberId`, `Role`, `Platform`, `AppVersion`, `ClientVersion`, and the tainted newtypes `PhoneNumber`, `DeviceToken`, `OnboardingCode`, `RecoveryCode`, `AccessToken`/`RefreshToken` — all with **no `Debug`/`Display`**, only `redacted_summary()` (P2). Author the golden JSON fixtures in `fixtures/auth/**` and `fixtures/manifest/**` named in plan §5.
- **Touches:** `core/domain/`, `fixtures/`.
- **Closes/enables:** enables **AC4, AC7** (types); fixtures underpin AC8/AC10/AC15/AC19.
- **Tests:** `insta` serialization snapshots of each wire type + the `redacted_summary()` forms; a compile/inspection test asserting tainted types expose no `Debug`/`Display`.
- **Blocked by:** T01 · **∥:** no (blocks 3–6, 10).
- **Done (2026-06-04):** `core/domain` defines the value types (`MemberId` transparent-UUID; `Role`; `Platform` with canonical `ios/ipados/watchos/macos/android/wearos/web` wire names; `AppVersion` string wire-form with **semantic** (not lexicographic) ordering + `FromStr`/`Display`/custom serde; `ClientVersion{platform, app_version}`) and the six tainted newtypes (`PhoneNumber`/`DeviceToken`/`OnboardingCode`/`RecoveryCode`/`AccessToken`/`RefreshToken`) — **no `Debug`/`Display`/`Serialize`**, only `redacted_summary()` + `expose_secret()`, enforced at compile time via `static_assertions::assert_not_impl_any!` (P2/I3). 15 golden fixtures authored (`fixtures/auth/`×9, `manifest/`×4, `compat/`×2) + per-dir README descriptors; the manifest Ed25519 **signature vectors are deferred to T03** (content bytes frozen; documented in `fixtures/manifest/README.md`). Deps pinned from the lock into `docs/stack-matrix.md`: `serde` 1.0.228, `serde_json` 1.0.150, `uuid` 1.23.2 (**`serde` feature only — no `rng`/getrandom**, keeping the lib wasm-safe + randomness-free), `insta` 1.47.2, `static_assertions` 1.1.0. **14 tests** green; `cargo fmt`/`clippy -D warnings`/`test` all `--locked` clean; **`cargo build --target wasm32-unknown-unknown` clean** (lib dep graph = serde + uuid only); binding-drift gate green (27 inputs, lock regenerated); network allow-list clean (3 lock files, no trackers). Reviewed via a 4-agent adversarial workflow (`reviewer` + `security-auditor` + `platform-parity` + `test-strategist`): **0 confirmed in-scope defects** (security-auditor clean on P2/I3/I4/I8). Two low/info test-hardening notes applied (inbound JSON round-trip for `MemberId`/`ClientVersion`; explicit named-fixture presence check). `platform-parity`'s 3 notes are forward-guidance for the **T10** UniFFI codegen (AppVersion record-vs-string, MemberId custom-type mapping, tainted-type formatter-free binding surface) — out of scope here, noted for T10.

---

## Wave 2 — crypto & schema (parallel)

### T03 — `core::crypto` (dryoc)
- **Does:** Phone **HMAC-SHA256 + constant-time compare** (I3); Onboarding/Recovery **code hashing at rest**; **Ed25519 detached-signature manifest verification** + the ADR-0014 **tiered fallback** (verify-fail→cached→bundled) and lower-`manifest_version`-ignore. All via `dryoc` (wasm32-safe). Injected RNG (no ambient randomness).
- **Touches:** `core/crypto/`, `core/crypto/tests/invariants.rs`.
- **Closes:** **AC3** (constant-time leg), **AC10** (verify + tiers — core leg).
- **Tests:** `i3_phone_lookup_constant_time`; `ac10_manifest_{verify_fail_with_cache,verify_fail_no_cache,lower_version_ignored,offline_first_launch}` (replay the `fixtures/manifest/**`).
- **Blocked by:** T02 · **∥:** with T06.

### T06 — Database migrations `0001`–`0008`
- **Does:** Write numbered migrations: `groups`, `members` (`phone_lookup_hash`/`phone_encrypted`), `onboarding_codes`, `recovery_codes`, `device_tokens` (`(member_id,platform,app_version)`), `sessions` (rotation lineage), `admin_webauthn_credentials` (multi-per-admin), `admin_invitations`. Conventions: PII `*_encrypted bytea`, `created_at/updated_at/created_by`, RLS. **No `0009_admin_alerts`** (§10-E: counter lives in the DO). `pgcrypto` is **not** on the PII path (§10-H).
- **Touches:** `server/migrations/0001…0008_*.sql`.
- **Closes/enables:** enables **AC4, AC16, AC17, AC18, AC19, AC20**.
- **Tests:** migration up/down applies on a Postgres fixture; RLS smoke test on a PII table; schema assertion that PII columns are `bytea`.
- **Blocked by:** T02 (agreed schema) · **∥:** with T03.

---

## Wave 3 — core::auth (parallel)

### T04 — `core::auth`: state machine, version compare, Onboarding/Recovery code logic
- **Does:** Implement the device-side `OnboardingState` machine (incl. `NeedsReauthHelp`, `Offline` overlay, `ManifestFailReturning`, `BelowMinVersion`); `AppVersion` vs `client_min_version` comparison (O4/O1); Onboarding Code **single-use / TTL / rate-limit** request-shaping + result interpretation; Recovery Code lifecycle. Pure logic + **injected `Clock`** (no `SystemTime::now`).
- **Touches:** `core/auth/` (+ UniFFI surface), `core/auth/tests`.
- **Closes:** **AC17** (code single-use/TTL/rate-limit + regenerate-invalidates-prior — core leg); contributes **AC8/AC15** (state decision), **AC19** (recovery logic).
- **Tests:** `prop_onboarding_code_single_use_ttl_ratelimit` (proptest, seeds checked in); `ac17_regenerate_invalidates_prior`; version-compare/N-2 property test. `TestClock` for the wrong-client-clock case.
- **Blocked by:** T02, T03 · **∥:** with T05.

### T05 — `core::auth`: sessions, refresh rotation, device-token binding
- **Does:** Indefinite session + **silent refresh-token rotation** with replay/lineage detection (ADR-0016 D2); device-token binding `(member_id, platform, app_version)` and **invalidation on new-device re-onboarding** (I4); the invalidation-trigger set (revoke/logout, re-onboard, delete). Secure at-rest storage contracts per §10-F (consumed by the UI tasks).
- **Touches:** `core/auth/`, `core/auth/tests`.
- **Closes:** **AC4** (re-onboarding invalidation — core leg), **AC18** (indefinite + silent refresh + triggers), contributes **AC19**.
- **Tests:** `i4_tokens_invalidated_on_reonboarding` (distinct from `…_on_logout`); `prop_session_indefinite_until_admin_event`; `ac18_invalidation_triggers_exactly[revoke,reonboard,delete]`; **`auth_refresh_rotation_replay_detected`** (the new privacy-invariant test — DEFERRED §G item 1, ships here).
- **Blocked by:** T02, T03 · **∥:** with T04.

---

## Wave 4 — server (parallel)

### T07 — Server (workers-rs): member-auth endpoints + degradation + DO
- **Does:** `/api/auth/signin` (phone-hash lookup, no existence leak), `/api/auth/bind-device` (Onboarding Code, server-time validated, **cannot complete offline**), `/api/auth/refresh` (rotation), `/api/auth/recovery/rebind` (Driver). Every response carries `client_min_version` + `client_recommended_version` (O4/O5). `GroupHub` DO: rate-limit counter + token invalidation (I4); **below-min-version** path → calm screen + **one Queues admin alert/member/day** (non-PII); the **notification-declined** non-PII admin flag (AC14). All logging via `boundless::logging::emit()` (P2).
- **Touches:** `core/server/`, `server/`, DO class.
- **Closes:** **AC7** (server leg), **AC8**, **AC14** (server leg), **AC17/AC18/AC19** (enforcement legs), **AC4** (server leg); **I5** audit emission on any admin phone read path it exposes.
- **Tests:** `ac8_below_min_emits_one_alert_per_member_per_day` (Clock + Queue stub); `ac14_decline_records_nonpii_flag_and_advances`; `ac15_invalidated_rider_alert_once_per_day`; integration for bind/refresh/recovery; alert-payload-no-PII scrub.
- **Blocked by:** T03, T04, T05, T06 · **∥:** with T08.

### T08 — Server: developer admin-create + Email Workers invite + compat harness
- **Does:** `/api/dev/admins` — developer-only, **hardware-key (WebAuthn) gated** (I11); mints a pending Admin + **single-use, 72h, server-time TTL** invitation token (AC16) delivered via **Email Workers** (ADR-0015 constraints: opaque token, no PII/credential in body). Stand up `server/tests/compat/` N-2 replay harness (O1).
- **Touches:** `server/` (`/api/dev/*`), Email Workers binding, `server/tests/compat/`.
- **Closes:** **AC1(a)** (dev endpoint authz: unauth + admin-auth both rejected), **AC9** (compat), **AC16** (mint/TTL leg).
- **Tests:** `ac1_admin_creation_rejects_unauth_and_admin`; `ac9_auth_endpoints_nminus2`; `ac16` mint side (single-use + server-time TTL); assert invite email body carries no PII/credential.
- **Blocked by:** T06 (admin tables), T03 (token hashing) · **∥:** with T07.

---

## Wave 5 — admin WebAuthn (edge)

### T09 — Admin WebAuthn verification on the SvelteKit edge (`@simplewebauthn/server`)
- **Does:** Server-side WebAuthn **registration + assertion verification** in SvelteKit server routes (TS, `@simplewebauthn/server`), per ADR-0017: `userVerification: required`, `attestation: none`, discoverable creds, **multiple credentials per admin**; consumes the T08 invite token on first successful registration; **Developer re-invite revokes prior credential(s)** (ADR-0015/0016 D4). **KV challenge store, 5-min TTL, one-time-use** (ADR-0017 D3). Documented P4 carve-out (ADR-0017 D4).
- **Touches:** `web/src/lib/server/webauthn/`, `web/` server routes, KV binding, `admin_webauthn_credentials` (read/write).
- **Closes:** **AC16** (consume leg), **AC20** (UV/no-attestation/multi-cred/recovery); supports **AC2**.
- **Tests:** `ac20_webauthn_requires_uv_no_attestation_multi_credential` (Playwright virtual authenticator); `ac16_invite_expired_routes_and_ttl_server_side` (reuse→`InviteExpired`, server-time TTL); KV challenge one-time-use test.
- **Blocked by:** T08 · **∥:** no (gates T15).

---

## Wave 6 — contracts (FREEZE GATE)

### T10 — API contracts + generated bindings (contract-freeze)
- **Does:** Finalize `api/openapi.yaml` (`client_min_version` **required** on every `/api/auth/*` response) + `api/boundless.proto` (WS open-handshake carries `client_min_version`). Generate `api/generated/<lang>/` (swift-openapi-generator / openapi-generator kotlin / openapi-typescript + ts-proto); build the UniFFI **XCFramework** + **AAR**. **Freeze** before any UI starts.
- **Touches:** `api/openapi.yaml`, `api/boundless.proto`, `api/generated/**`, `apple/BoundlessKit/`, `android/core-bridge/`, `web/src/lib/api/generated/`.
- **Closes:** **AC7** (contract leg — OpenAPI + proto required-field tests).
- **Tests:** `ac7_auth_responses_require_min_and_recommended_version` (OpenAPI); `ac7_ws_handshake_has_client_min_version` (proto); generated-binding-drift CI gate green.
- **Blocked by:** T05, T07 (shapes stable), T08/T09 (admin shapes) · **∥:** no (blocks Wave 7).

---

## Wave 7 — per-platform UI (all parallel)

### T11 — SwiftUI Rider onboarding UI
- **Does:** All Rider onboarding screens per the state machine (helper intro → phone entry → Onboarding Code → permissions(+declined) → auto-update step/enabled → silent complete); the calm `BelowMinVersion`/`NeedsReauthHelp` screens (never a form); `Offline` overlay. Consume `core::auth` via `BoundlessKit`. Rider Settings exposes **no** auto-update toggle.
- **Touches:** `apple/BoundlessRider/`, `RiderShared`.
- **Closes:** **AC5, AC6, AC8(snapshot), AC11, AC14(UI), AC15(Rider snapshot), AC1(b)** (no-signup-route, iOS).
- **Tests:** `swift-snapshot-testing` ×4 variants per screen; `AutoUpdateStep` resolves `onboarding.autoupdate.enabled`; `NeedsReauthHelp` no-form snapshot; `RiderSettings` no-toggle; VoiceOver traversal; `ios_onboarding_no_signup_route`.
- **Blocked by:** T10 · **∥:** T12–T15.

### T12 — SwiftUI Driver onboarding UI
- **Does:** Driver onboarding (self-run); **Recovery Code one-time capture** screen (D3); Driver session-expiry routes to interactive re-auth (`auth.signin_again`).
- **Touches:** `apple/BoundlessDriver/`.
- **Closes:** **AC11, AC14(UI), AC15(Driver branch), AC1(b), AC19(UI)**.
- **Tests:** snapshots ×4; Recovery-Code-capture snapshot; Driver re-auth route test; `ios_driver_no_signup_route`.
- **Blocked by:** T10 · **∥:** T11,T13–T15.

### T13 — Compose Rider onboarding UI
- **Does:** Android Compose mirror of T11 (TalkBack semantics, 48dp targets).
- **Touches:** `android/rider/app/`.
- **Closes:** **AC5, AC6, AC8(snapshot), AC11, AC14(UI), AC15(Rider), AC1(b)** (Compose).
- **Tests:** `Paparazzi` ×4 variants; TalkBack traversal; no-toggle / no-signup-route.
- **Blocked by:** T10 · **∥:** T11,T12,T14,T15.

### T14 — Compose Driver onboarding UI
- **Does:** Android Compose mirror of T12 (incl. Recovery Code capture, driver re-auth).
- **Touches:** `android/driver/app/`.
- **Closes:** **AC11, AC14(UI), AC15(Driver), AC1(b), AC19(UI)**.
- **Tests:** `Paparazzi` ×4; Recovery-Code snapshot; re-auth route; no-signup-route.
- **Blocked by:** T10 · **∥:** T11–T13,T15.

### T15 — SvelteKit admin onboarding UI
- **Does:** Invite-link landing → WebAuthn **registration ceremony** → `InviteExpired` → WebAuthn **sign-in** (no password field). Consumes T09 verification + the generated TS client.
- **Touches:** `web/` routes + components.
- **Closes:** **AC2, AC11b, AC1(b)** (web no-signup-route).
- **Tests:** Playwright + axe-core zero-violations on all 4 routes; `ac2_no_password_field`; keyboard-only WebAuthn ceremony; 200%/400% reflow; `aria-live` on expired/error; RTL/dark.
- **Blocked by:** T09, T10 · **∥:** T11–T14.

---

## Wave 8 — cross-cutting verification

### T16 — Cross-cutting verification sweep
- **Does:** Run the suite-wide checks across all surfaces: log-scrubber replay over onboarding fixtures incl. error/offline branches (P2/I10); network allow-list against all lock files (I8); pseudo-locale (`zz-ZZ`) render across every onboarding screen; compat replay (O1).
- **Touches:** `fixtures/onboarding/log_lines.jsonl`, CI checks, all UI test suites.
- **Closes:** **AC3** (scrubber leg), **AC12** (pseudo-locale), **AC13** (allow-list), **AC9** (final compat run).
- **Tests:** scrubber replay (zero PII reaches scrubber); `ac13_onboarding_adds_no_third_party`; `pseudo_locale_renders_all_onboarding_screens`.
- **Blocked by:** T07–T15 · **∥:** no (final gate).

---

## Deferred (NOT tasks here — tracked in `DEFERRED.md`)

- The two remaining new privacy-invariant tests land with the **deletion** work, not this spec: extend the I12 forgetting property test to the auth artifacts, and the named delete-leg device-token invalidation (the refresh-rotation-replay test **is** in T05). Account deletion flow itself is out of scope (spec §Out of scope).
- Critical Alerts capability upgrade (post-entitlement); the `@simplewebauthn`→`webauthn-rs`-sidecar fallback if edge support breaks.

---

## AC coverage check

AC1✓(T08+T11–T15) AC2✓(T15/T09) AC3✓(T03+T16) AC4✓(T05+T07) AC5✓(T11/T13) AC6✓(T11/T13) AC7✓(T07+T10) AC8✓(T07+T11/T13) AC9✓(T08+T16) AC10✓(T03) AC11✓(T11–T14) AC11b✓(T15) AC12✓(T16) AC13✓(T16) AC14✓(T07+T11–T14) AC15✓(T05/T07+T11–T14) AC16✓(T08+T09) AC17✓(T04+T07) AC18✓(T05+T07) AC19✓(T05/T07+T12/T14) AC20✓(T09/T15). **All 21 acceptance criteria covered.**
