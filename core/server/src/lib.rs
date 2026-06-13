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
//! - **T08:** [`AuthService::create_admin`] / [`AuthService::reissue_admin_invite`] — developer-only
//!   Admin provisioning + invitation mint ([`authorize_developer`] / [`DeveloperAuthority`] gate
//!   AC1; [`AdminProvisioningStore`] persists; AC16). The deployable `/api/dev/admins` route, the
//!   Developer WebAuthn verification, Email Workers delivery, and the invite consume (T09) are the
//!   deferred shell (→ `DEFERRED.md`).
//! - [`GroupHubState`] — the DO's rate-limit windows + per-member-per-day alert dedup (§10-E).
//! - [`AdminAlert`] — PII-free admin alerts/flags (O4/AC8/AC14/AC15).
//! - [`normalize_phone`] — canonical E.164 for the lookup hash (single-source for issuance, P4).
//!
//! ## Production port impls (T07-shell)
//!
//! - The Postgres [`AuthStore`] (`PgAuthStore`, `server/store`) is built + proven against real
//!   Postgres (ADR-0019/0020).
//! - The production [`SecretSource`] is [`RngSecretSource`] (this crate): opaque-random tokens from
//!   an **injected** CSPRNG, so the core stays randomness-free + `wasm32`-safe (ADR-0021).
//!
//! ## Deliberately **not** here (→ `DEFERRED.md` → T07-shell-B)
//!
//! The deployable workers-rs runtime (`#[event]`/Router/`GroupHub` DO/Queues/KV/Turnstile/Hyperdrive
//! Socket), the access-token **store column + per-request verify lookup** (the opaque bearer's
//! verification path + its DO-fold/write-through-on-revoke guard-rail, ADR-0021), APNs/FCM device
//! registration + the `PgDeviceStore` (needs spec-008 token encryption), and the live Worker
//! integration tests are the **T07-shell-B** infra task. I5 admin-PII audit + `audit_log` belong with
//! admin issuance (spec 008 — this layer exposes no admin phone read).

mod admin;
mod admin_webauthn;
mod alerts;
mod audited;
mod bind;
mod bootstrap;
mod hub;
mod member;
mod phone;
mod ports;
mod recovery;
mod refresh;
mod secrets;
mod service;
mod signin;

pub use admin::{
    authorize_developer, AdminInvitation, DevAdminCreateForbidden, DevCaller, DeveloperAuthority,
    INVITE_TTL_SECS,
};
pub use admin_webauthn::{
    AdminCredential, AdminInviteRecord, NewAdminCredential, RegisterCompleteOutcome,
};
pub use alerts::{AdminAlert, AlertKind};
pub use audited::{admin_response_body, AuditedResponse, PiiDisclosure};
pub use bind::{BindOutcome, BindRequest, BindResponse};
pub use bootstrap::{
    generate_group_key, load_group_key, GroupKeyBootstrap, GroupKeyMissing, INITIAL_KEK_VERSION,
};
pub use hub::{GroupHubState, CODE_ATTEMPT_WINDOW_SECS};
pub use member::{
    issuable_roles, AdminRoleForbidden, AuditEntry, AuditField, AuditLogView, AuditStore,
    DelegatedKeyStore, DetailRead, DuplicateDisclosureAudit, DuplicatePhoneLinkView, EditApplied,
    EditMemberInput, EditMemberOutcome, InsertMemberOutcome, IssuableRole, IssueMemberInput,
    IssueMemberOutcome, MemberConfig, MemberDetail, MemberDetailView, MemberEditWrite, MemberError,
    MemberIssuedView, MemberListView, MemberService, MemberStore, MemberSummary, NewMemberWrite,
    OnboardingStatus, RegenerateCodeView, RegenerateOutcome, StoredMemberPii, StoredMemberSummary,
    ONBOARDING_CODE_TTL_SECS,
};
pub use phone::{normalize_phone, PhoneNormalizeError};
pub use ports::{
    AdminAlertSink, AdminProvisioningStore, AdminWebAuthnStore, AuthStore, DeviceStore, FamilyInfo,
    MemberRecord, OnboardingCodeRow, RecoveryCodeRow, RefreshClassification, SecretSource,
    SessionMaterial, SourceKey, StoreBackend,
};
pub use recovery::{RecoveryOutcome, RecoveryRequest, RecoveryResponse};
pub use refresh::{RefreshOutcome, RefreshRequest, RefreshResponse};
pub use secrets::RngSecretSource;
pub use service::{AuthConfig, AuthService, ManifestPointer, ACCESS_TTL_SECS};
pub use signin::{SignInRequest, SignInResponse};
