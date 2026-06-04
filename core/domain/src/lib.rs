//! `boundless-domain` — the single source of truth for Boundless domain & value types
//! (constitution P4, ADR-0001).
//!
//! This crate defines the tainted newtypes (`PhoneNumber`, `DeviceToken`,
//! `OnboardingCode`, `RecoveryCode`, `AccessToken`/`RefreshToken`) — each with **no
//! `Debug`/`Display`**, only `redacted_summary()` (P2) — and the value types
//! (`MemberId`, `Role`, `Platform`, `AppVersion`, `ClientVersion`, …). They are
//! generated to Swift/Kotlin via UniFFI; clients hold no hand-rolled duplicates.
//!
//! Pure logic, no I/O, no ambient time/randomness (see `docs/forbidden-patterns.md`).
//!
//! Scaffolded by spec 001 task **T01**; the types land in **T02**.
