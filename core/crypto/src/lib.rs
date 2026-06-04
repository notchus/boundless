//! `boundless-crypto` — single-source cryptography for Boundless (P4, I1/I3, ADR-0014).
//!
//! Phone **HMAC-SHA256 + constant-time compare** (I3), Onboarding/Recovery **code
//! hashing at rest**, per-Group **sealed-box/secretbox PII encryption** (I1), and
//! **Ed25519 detached-signature manifest verification** with ADR-0014's tiered fallback
//! (verify-fail → cached → bundled). All via `dryoc` (pure-Rust, wasm32-safe), with an
//! **injected RNG** — no ambient randomness.
//!
//! Scaffolded by spec 001 task **T01**; the crypto + `tests/invariants.rs` land in
//! **T03**, where this crate activates `dryoc.workspace = true`.
