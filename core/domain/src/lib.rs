//! `boundless-domain` — the single source of truth for Boundless domain & value types
//! (constitution **P4**, ADR-0001).
//!
//! This crate defines:
//!
//! - **Value types** — [`MemberId`], [`SessionFamilyId`], [`Role`], [`Platform`],
//!   [`AppVersion`], [`ClientVersion`]. Plain, serializable, safe to log.
//! - **Tainted secret newtypes** — [`PhoneNumber`], [`DeviceToken`], [`OnboardingCode`],
//!   [`RecoveryCode`], [`AccessToken`], [`RefreshToken`]. Each has **no `Debug`/`Display`/
//!   `Serialize`**, only `redacted_summary()` (**P2**); the raw value is reachable only via
//!   the intentionally-alarming `expose_secret`.
//!
//! These are generated to Swift/Kotlin via UniFFI (wired in T10); clients hold no
//! hand-rolled duplicates. The crate is **pure**: no I/O, no ambient time/randomness, and
//! it compiles to `wasm32-unknown-unknown` (see `docs/forbidden-patterns.md`).
//!
//! Scaffolded by spec 001 task **T01**; types and golden fixtures land in **T02**.

mod ids;
mod platform;
mod role;
mod tainted;
mod version;

pub use ids::{MemberId, SessionFamilyId};
pub use platform::Platform;
pub use role::Role;
pub use tainted::{
    AccessToken, DeviceToken, OnboardingCode, PhoneNumber, RecoveryCode, RefreshToken,
};
pub use version::{AppVersion, AppVersionParseError, ClientVersion};
