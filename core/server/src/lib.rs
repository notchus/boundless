//! `boundless-server-core` — the member-authentication orchestration engine (spec 001 **T07**,
//! Layer A): the server-side logic for the `/api/auth/*` endpoints and the `GroupHub` Durable
//! Object's decision state. The single source of truth (P4) for *how the server decides*, so the
//! deployable Worker is a thin adapter that cannot drift.
//!
//! ## Functional core, imperative shell
//!
//! This crate is the **functional core**: it is pure, deterministic, and `wasm32`-safe — no I/O,
//! no ambient time or randomness. The I/O boundary is a set of **port traits** ([`AuthStore`],
//! [`AdminAlertSink`], [`SecretSource`]) plus the injected [`Clock`](boundless_auth::Clock). Tests
//! supply in-memory ports; the deployable Worker (`server/`, **T07-shell**, deferred — see
//! `DEFERRED.md`) supplies the Cloudflare/Postgres ports. The endpoint methods compose the
//! existing `core::auth` decisions ([`evaluate_onboarding_code`], [`evaluate_refresh`],
//! [`evaluate_version`], …) so the server enforces exactly what the clients render.
//!
//! [`evaluate_onboarding_code`]: boundless_auth::evaluate_onboarding_code
//! [`evaluate_refresh`]: boundless_auth::evaluate_refresh
//! [`evaluate_version`]: boundless_auth::evaluate_version
//!
//! ## What lands in T07 (Layer A — this slice)
//!
//! - [`AuthService`] — the composition root. Endpoints: [`AuthService::sign_in`] (AC7/AC8,
//!   no-existence-leak), [`AuthService::bind_device`] (AC17 lifecycle + atomic consume, AC4 device
//!   invalidation), [`AuthService::record_notification_decision`] (AC14),
//!   [`AuthService::refresh`] (AC18 rotate / replay-kill, uniform reject), and
//!   [`AuthService::recovery_rebind`] (AC19), plus [`AuthService::note_session_invalidated`] (AC15).
//! - [`GroupHubState`] — the DO's rate-limit windows + per-member-per-day alert dedup (§10-E).
//! - [`AdminAlert`] — PII-free admin alerts/flags (O4/AC8/AC14/AC15).
//! - [`normalize_phone`] — canonical E.164 for the lookup hash (single-source for issuance, P4).
//!
//! ## Deliberately **not** here (→ `DEFERRED.md` → Server / core (T07) and T07-shell)
//!
//! The deployable workers-rs runtime (`#[event]`/Router/`GroupHub` DO/Queues/KV/Turnstile) and
//! the Postgres-over-Hyperdrive [`AuthStore`] impl (with `SET LOCAL` RLS, atomic
//! `UPDATE … RETURNING`), the real CSPRNG [`SecretSource`], access-token signing, APNs/FCM device
//! registration, and the live integration tests are the **T07-shell** infra task. I5 admin-PII
//! audit + `audit_log` belong with admin issuance (spec 008 — this layer exposes no admin phone read).

mod alerts;
mod bind;
mod hub;
mod phone;
mod ports;
mod recovery;
mod refresh;
mod service;
mod signin;

pub use alerts::{AdminAlert, AlertKind};
pub use bind::{BindOutcome, BindRequest, BindResponse};
pub use hub::{GroupHubState, CODE_ATTEMPT_WINDOW_SECS};
pub use phone::{normalize_phone, PhoneNormalizeError};
pub use ports::{
    AdminAlertSink, AuthStore, DeviceStore, FamilyInfo, MemberRecord, OnboardingCodeRow,
    RecoveryCodeRow, RefreshClassification, SecretSource, SessionMaterial, SourceKey, StoreBackend,
};
pub use recovery::{RecoveryOutcome, RecoveryRequest, RecoveryResponse};
pub use refresh::{RefreshOutcome, RefreshRequest, RefreshResponse};
pub use service::{AuthConfig, AuthService, ManifestPointer, ACCESS_TTL_SECS};
pub use signin::{SignInRequest, SignInResponse};
