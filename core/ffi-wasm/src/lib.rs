//! `boundless-ffi-wasm` — wasm-bindgen crate producing the JS package for the admin web
//! (docs/architecture.md §1). Limited use: client-side validation and WebAuthn request/
//! response shapes only — the heavy auth logic stays in Workers (ADR-0001, ADR-0017 D4).
//!
//! Scaffolded by spec 001 task **T01**; the wasm-bindgen surface lands at the contract
//! freeze in **T10**.
