# Boundless Constitution

> **This file is the highest authority in the project.** Specs reference it. Plans must obey it. Implementation must enforce it through code, not comments. Amendments require an ADR.

Version: 1.0.0 (initial)
Ratified: TODO

---

## P1 — Accessibility is the product

The product fails if a 78-year-old user with cataracts and arthritic fingers cannot use it. Accessibility is not a polish step; it is the primary acceptance criterion.

**Enforced by:**
- Every screen must pass:
  - **iOS:** VoiceOver navigation; Dynamic Type to `accessibility5` (xxxLarge); Switch Control reachable; minimum 44pt touch targets.
  - **Android:** TalkBack semantics complete; font scaling to 200%; Switch Access reachable; minimum 48dp touch targets.
  - **Web:** WCAG 2.2 AA minimum, AAA where reasonable; keyboard-complete; visible focus rings; zoom to 400% without horizontal scroll.
- Snapshot tests are **required** at four variants: default + largest text + dark mode + RTL.
- The `a11y-bar.md` lints run in CI on every PR.

---

## P2 — No PII in logs. Ever.

Personally identifiable information includes: full name + address pair, exact street address, phone number, exact GPS coordinates, device push token, email, date of birth. Logging any of these is a build failure.

**Enforced by:**
- Tainted-type wrappers in Rust: `Address`, `PhoneNumber`, `DeviceToken` are newtypes with **no** `Debug`/`Display`. The only formatter is `redacted_summary()` which produces e.g. `"Address(zip=12345, …)"`.
- Equivalent wrappers in Swift (no `CustomStringConvertible`) and Kotlin (no `toString` returning raw value).
- A lint rule rejects `println!`, `dbg!`, `print(_:)`, `Log.d(...)`, `console.log` calls whose arguments include any PII type.
- A log-scrubber CI step replays the latest log fixtures and fails on any PII regex match.

---

## P3 — Locations are dropped after match

Per the product spec: "Computed at match, then locations dropped." This is a hard architectural invariant, not a soft promise.

**Enforced by:**
- The matching Durable Object holds plaintext addresses **only** for the duration of one matching call.
- After the call returns the chain (an ordered list of opaque rider IDs), the plaintext is dropped from memory and never written to durable storage.
- The DO's persistent state contains only: rider ID, driver ID, chain order, ETA, and audit timestamps.
- A Rust unit test (`core::crypto::test_locations_dropped`) replays the matching call and asserts that no plaintext address exists in the DO's memory dump after.
- A property-based test asserts the invariant across 10,000 random group configurations.

---

## P4 — The Rust core is the source of truth

Any platform-level divergence in business logic is a bug, not a feature.

**Enforced by:**
- Domain types (`Rider`, `Driver`, `Group`, `Chain`, `OptOut`, etc.) are defined once in `core/domain/`.
- Swift and Kotlin clients are **generated** via UniFFI. Hand-rolled type duplicates are a CI failure.
- Shared golden JSON fixtures (`fixtures/`) are replayed in every platform's test suite.
- Any client-only computation that affects user-visible behavior requires an ADR explaining why it cannot live in the core.

---

## P5 — Spec before code

No PR without a spec. No spec without a `/clarify` pass.

**Enforced by:**
- The PR template requires a link to `specs/NNN-*/spec.md`. CI fails if missing.
- The spec must include: user story, acceptance criteria (testable), edge cases, i18n key list, accessibility notes, privacy notes.
- The plan must enumerate which constitution principles it touches.

---

## P6 — Plan mode is the default

Implementation only happens against an approved plan and an approved task list. "I'll just hack on this" is a process violation.

**Enforced by:**
- Claude Code is run in plan mode for any non-trivial change.
- The `tasks.md` file is the contract for what gets implemented in this round. Any work not in `tasks.md` is scope creep — open a new task.

---

## P7 — Native UI on every platform

No cross-platform UI frameworks. Each platform gets its idiomatic UI, sharing only the Rust core.

**Enforced by:**
- Apple (iOS, iPadOS, watchOS, macOS): SwiftUI.
- Android (phone): Jetpack Compose. Wear OS: Compose for Wear OS.
- Web (admin only): SvelteKit.
- Any proposal to share UI requires an ADR and must demonstrate accessibility parity with native.

---

## P8 — i18n is not afterthought

Every user-visible string ships from a translation catalog on day one. English text in code is a build failure.

**Enforced by:**
- Lint: no string literals in user-visible code paths. All strings must come from the catalog.
- Catalog keys are validated for completeness across all configured locales before merge.
- `i18n-validator` subagent runs on any PR that touches user-visible code.
- Pseudo-locale (`zz-ZZ`) builds catch hardcoded strings.

---

## P9 — Privacy invariants are testable

Each privacy promise is a Rust unit test or property test, not a comment.

**Enforced by:**
- `core/crypto/tests/invariants.rs` enumerates every invariant from `docs/privacy-invariants.md` as a named test.
- New invariants require a test before merge.
- Property tests use `proptest` with seeds checked into the repo for reproducibility.

---

## P10 — Don't surprise the elderly user

Visual or interaction changes need a persona-grounded justification in the spec.

**Enforced by:**
- The spec template includes a "What changes for Maria?" (rider persona) section.
- The reviewer subagent flags changes that touch the rider's primary surface without this section filled.
- A/B tests, dark patterns, growth-hacking nudges, gamification: forbidden by default. Use must be justified in an ADR.

---

## P11 — Free, open, donation-supported

The product is and remains free. No paywalled features. Source is public. Donations are optional and never coerced.

**Enforced by:**
- All hard dependencies must have free-tier or open-source paths.
- The donation surface ("buy me a coffee" or equivalent) is opt-in, not interstitial.
- Anti-feature creep: any proposal for telemetry, ads, in-app purchases, or upsells is rejected at the spec gate.

---

## P12 — Operability is part of the build

If it can't be debugged, it can't ship.

**Enforced by:**
- Structured logging (without PII per P2) on every state transition.
- OpenTelemetry traces on every Worker request and Durable Object call.
- Every error type has a stable error code. Codes are documented in `docs/error-codes.md`.
- Runbooks for every alert in `docs/runbooks/`.

---

## P13 — Updates are nearly invisible

The rider must never see an "update required" interstitial. Operational changes flow through the server first, then through native auto-update overnight, then through admin-assisted help — in that order. The rider's primary surface is the last place an update prompt is allowed.

**Enforced by:**
- **Server-driven configuration.** Translations, copy, design tokens, feature flags, error messages, layout adjustments — served from Cloudflare KV with a signed manifest. Client pulls on launch, caches, applies without restart. See ADR-0014.
- **Backward compatibility N-2.** The server supports the current client version and the two previous *minor* versions, minimum. A rider on a 4-month-old build still works. Removing support for a version requires an ADR.
- **Auto-update at onboarding.** First-launch flow includes a "set up this phone" step that an admin or family member performs in person; enabling App Store / Play Store auto-update is one of those steps. The rider never toggles this themselves.
- **Graceful degradation below `client_min_version`.** The server returns `client_min_version` in every auth response. If the client is below it, the client shows a single calm screen ("This device needs Sarah's help — she's been told") and the server emits an admin alert via Queue. No mid-flow interrupts.
- **Admin sees the version distribution.** The admin dashboard displays who is on which version (via Analytics Engine, non-PII) and surfaces stragglers as actionable tasks ("call Margaret's daughter about updating her phone").
- **Manual fallback always exists.** The admin can export a printable phone list. If the entire app fleet is broken, the gathering still happens.
- **Matching never depends on the client version.** The server is the operational source of truth. The rider's app failing must never prevent the driver from arriving.

See `docs/operational-invariants.md` for the testable form (O1–O8) and `docs/update-strategy.md` for the ladder.

---

## Amendments

Amending this constitution requires:
1. An ADR under `docs/adr/` proposing the change with rationale.
2. A version bump in this file.
3. A migration plan if the change invalidates existing code or tests.

This file is **append-mostly**: prefer adding clarifying sub-bullets to removing principles.
