//! `boundless-logging` — the PII scrubber/detector for the structured-logging pipeline
//! (privacy invariant **I10**, constitution **P2**: "No PII in logs. Ever.").
//!
//! # What this crate is, and what it is *not* (yet)
//!
//! I10 describes a logging pipeline with two halves:
//!
//! 1. a single sink, `boundless::logging::emit()`, that **every** Worker log line goes
//!    through (direct `tracing::*` is lint-forbidden), which runs each line through a PII
//!    scrubber before persistence; and
//! 2. a CI step that replays the latest run's logs through that scrubber and fails on any
//!    redaction — i.e. PII should *never reach* the scrubber to begin with, because the
//!    tainted newtypes (`PhoneNumber`/`DeviceToken`/…) are not `Serialize`/`Display` and so
//!    cannot be formatted into a line at all (the compile-time P2 guarantee).
//!
//! This crate ships **the scrubber/detector** (half 1's core, [`scrub::detect_pii`]) and is
//! replayed by the standalone onboarding gate (spec 001 **T16**). The deployable `emit()`
//! sink, the no-raw-`tracing` lint, and the Logpush/latest-run CI replay land with the
//! Worker runtime — **T07-shell-B** (see `DEFERRED.md`). Keeping the detector here, in a
//! pure wasm32-safe crate, means the Worker (which is wasm) reaches the same single-source
//! detector the tests gate on — no drift (P4).
//!
//! # The detector
//!
//! [`scrub::detect_pii`] is a conservative, dependency-free byte scanner. It is the
//! defense-in-depth backstop: if PII ever leaked past the type system into a log line, it is
//! caught here. See [`scrub`] for the categories and the threshold rationale (notably: the
//! project's two legitimately-opaque non-PII values — `MemberId` UUIDs and version strings —
//! are deliberately never flagged, or the "logs are clean" gate would be vacuous).

#![forbid(unsafe_code)]

pub mod scrub;

pub use scrub::{contains_pii, detect_pii, Finding, PiiCategory};
