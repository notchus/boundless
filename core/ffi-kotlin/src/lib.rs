//! `boundless-ffi-kotlin` — UniFFI binding crate that produces the AAR consumed by the
//! Android `core-bridge` module (docs/architecture.md §1).
//!
//! Re-exports the core domain/auth/sync surface across the UniFFI boundary; the Kotlin
//! clients hold no hand-rolled auth logic (P4).
//!
//! Scaffolded by spec 001 task **T01**; the UniFFI surface + AAR build land at the
//! contract freeze in **T10**.
