//! The I5 audit gate — making "an admin-PII read that reaches the wire was audit-logged" a
//! **compile-time** guarantee, not a convention (spec 008 **T06**; AC7 compile leg).
//!
//! I5 is the one privacy invariant whose constitution text demands a compile control: *"any handler
//! returning a type that contains `Address`/`PhoneNumber` must … fail the build"* without an audit.
//! [`MemberService::read_detail`](crate::MemberService::read_detail) already builds the
//! [`AuditEntry`] and the store commits it **atomically** with the ciphertext SELECT (T05); this
//! module makes the *omission* of that audit a type error.
//!
//! ## The capability, applied to responses (the `DeveloperAuthority` idiom)
//!
//! The same "enforce through code, not comments" pattern as
//! [`DeveloperAuthority`](crate::DeveloperAuthority): a value that can only be constructed through
//! the sanctioned path *is* the proof the obligation was met.
//!
//! - [`PiiDisclosure<T>`] is the **only serializable carrier** of an audited PII bundle. Its
//!   constructor is `pub(crate)` — **un-forgeable outside this crate**, so the T09 Worker cannot
//!   fabricate one; it only obtains a `PiiDisclosure<MemberDetailView>` from `read_detail`, which
//!   built + committed the audit first. The `Serialize` impl delegates to the payload, so the wire
//!   shape is byte-identical; the carrier itself is **not** `Debug` (P2 — never print the bundle).
//! - [`AuditedResponse`] is a **sealed** marker (its supertrait is private, so only this crate
//!   decides what is sendable). It is implemented for `PiiDisclosure<T>` and for an explicit,
//!   hand-curated allowlist of provably-PII-free response types ([`MemberSummary`], the list/audit
//!   vecs). A new PII wire type is *non-sendable until a human adds it here* — and the only PII-
//!   bearing impl is `PiiDisclosure<T>`, which needs the audit.
//! - [`admin_response_body`] is the single seam the T09 router serializes admin responses through.
//!   Its `R: AuditedResponse + Serialize` bound means a bare PII DTO — or any un-blessed type —
//!   does not type-check there (`require_audit_compile_fail`, `tests/ui/require_audit/`).
//!
//! ## What this gate does NOT close (honest scope — see `DEFERRED.md` → T06)
//!
//! No *pure-Rust* gate is fully airtight: [`expose_secret`](boundless_domain::Address::expose_secret)
//! is a deliberate escape hatch, and a future handler could hand-roll a body with
//! `serde_json::json!({…})` and send it via `worker::Response::from_json` without ever naming a
//! [`PiiDisclosure`]. That residual is covered by I5's **own** named second layer — T08's
//! `openapi_pii_handlers_all_require_audit` integration test (every OpenAPI PII handler has a
//! matching audit) — plus the T09 P2 lint forbidding `Response::from_json`/`json!` on member PII in
//! `server/runtime/**`, and the P2/I10 scrubber. This module closes the *dominant, most-natural*
//! path: you cannot forge the carrier, cannot serialize the bare [`MemberDetailView`], and cannot
//! send a non-`AuditedResponse` through the seam.

use serde::{Serialize, Serializer};

use crate::admin_webauthn::{AdminCredential, AdminInviteRecord, AdminRegisterCompleteResult};
use crate::member::{
    AuditEntry, AuditLogView, DuplicatePhoneLinkView, MemberIssuedView, MemberListView,
    MemberSummary, RegenerateCodeView,
};

/// Module-private supertrait so [`AuditedResponse`] is **sealed**: only `boundless-server-core`
/// can decide what is sendable to an admin, so no downstream crate (the T09 Worker, the web tier)
/// can quietly add a PII type to the allowlist.
mod sealed {
    pub trait Sealed {}
}

/// An audited disclosure of member PII to an admin — the **only serializable carrier** of a
/// decrypted-PII wire bundle (I5).
///
/// Holds the wire payload alongside the [`AuditEntry`] that was committed for the read. The
/// constructor is `pub(crate)`, so — exactly like [`DeveloperAuthority`](crate::DeveloperAuthority)
/// — a value of this type cannot exist outside this crate: the T09 Worker can only acquire one from
/// [`MemberService::read_detail`](crate::MemberService::read_detail) (which audited first). The
/// [`Serialize`] impl forwards to the payload (the wire shape is unchanged from the bare DTO); the
/// carrier is deliberately **not** `Debug`, so the bundle cannot be printed into a log (P2).
pub struct PiiDisclosure<T> {
    /// The committed audit record for this disclosure (the Worker logs it; never serialized to the
    /// client — the `Serialize` impl forwards only the payload).
    audit: AuditEntry,
    /// The PII wire DTO being disclosed (e.g. [`MemberDetailView`](crate::MemberDetailView)).
    payload: T,
}

impl<T> PiiDisclosure<T> {
    /// The **sole** constructor — `pub(crate)`, so the audited carrier is un-forgeable outside this
    /// crate. Called only from the audited read path, *after* the [`AuditEntry`] is built and the
    /// store has committed it atomically with the ciphertext SELECT (I5 / §7).
    pub(crate) fn new(audit: AuditEntry, payload: T) -> Self {
        Self { audit, payload }
    }

    /// The committed audit record, for the Worker's structured-log / correlation path.
    pub fn audit(&self) -> &AuditEntry {
        &self.audit
    }

    /// Borrow the PII payload (e.g. to read individual fields). Never log the result (P2).
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Decompose at the boundary into `(audit, payload)`.
    pub fn into_parts(self) -> (AuditEntry, T) {
        (self.audit, self.payload)
    }
}

// The wire body is EXACTLY the payload's shape — the audit is logged, never sent to the client. So
// `serde_json::to_string(&disclosure)` yields the same JSON the bare DTO did (the spec-008 T05
// `member_detail_view_wire_keys_are_pinned` pinned-keys assertion is unaffected by the wrapping).
impl<T: Serialize> Serialize for PiiDisclosure<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.payload.serialize(serializer)
    }
}

/// A value the admin surface is permitted to serialize to a client — **sealed** (only this crate
/// populates it, via the private [`sealed::Sealed`] supertrait). The only PII-bearing implementor is
/// [`PiiDisclosure<T>`] (which requires the audit); everything else on the list is provably PII-free.
pub trait AuditedResponse: sealed::Sealed {}

// The audited PII carrier is always sendable — and its existence already proves the audit (I5).
impl<T> sealed::Sealed for PiiDisclosure<T> {}
impl<T> AuditedResponse for PiiDisclosure<T> {}

// ── The PII-free allowlist (opt-IN; negative trait bounds are not stable, so "PII-free" can never
// be a blanket negative impl — each sendable PII-free response is hand-affirmed here, and the type
// system re-checks it carries no tainted field via its own `Serialize` derive). T08 extends this
// list for the new admin wire DTOs (`MemberList`, `IssueMemberResponse`, …) it freezes. ──────────

/// The member-list summary (AC8 — no tainted type) is sendable directly; listing is not an audited
/// read (name-alone is not the P2-sensitive unit).
impl sealed::Sealed for MemberSummary {}
impl AuditedResponse for MemberSummary {}
/// The member list (AC8).
impl sealed::Sealed for Vec<MemberSummary> {}
impl AuditedResponse for Vec<MemberSummary> {}
/// The audit-log read (AC9) — field **names**, never values, so it is not a recursive PII read.
impl sealed::Sealed for Vec<AuditEntry> {}
impl AuditedResponse for Vec<AuditEntry> {}

// ── T09 admin wire envelopes (the HTTP response bodies the Worker serializes through the seam). Each
// is PII-free in the I5 sense — `MemberSummary` (display name only; the duplicate-phone disclosure's
// audit is store-enforced), `AuditEntry` (field names only), or a show-once `onboarding_code`
// credential (a P2 secret, not a disclosed member field). Blessing them here means the Worker emits
// EVERY admin response through `admin_response_body` — it hand-rolls no member-PII JSON (I5). ──────

/// `GET /api/admin/members` body (AC8).
impl sealed::Sealed for MemberListView {}
impl AuditedResponse for MemberListView {}
/// `POST /api/admin/members` 201 body (AC1/AC5) — carries the show-once code, no decrypted member field.
impl sealed::Sealed for MemberIssuedView {}
impl AuditedResponse for MemberIssuedView {}
/// `POST /api/admin/members` 409 body — the duplicate-phone surface-and-link (audited in the store).
impl sealed::Sealed for DuplicatePhoneLinkView {}
impl AuditedResponse for DuplicatePhoneLinkView {}
/// `POST /api/admin/members/{id}/regenerate-code` body (AC6) — show-once code only.
impl sealed::Sealed for RegenerateCodeView {}
impl AuditedResponse for RegenerateCodeView {}
/// `GET /api/admin/audit-log` body (AC9) — field names only.
impl sealed::Sealed for AuditLogView {}
impl AuditedResponse for AuditLogView {}

// ── spec 009 T04 — the Option B1 admin-WebAuthn wire response DTOs (ADR-0027). PII-free in the I5
// sense (opaque WebAuthn bytes + counters + server-time instants — no decrypted member name/phone/
// address), so the new `/api/admin/webauthn/*` endpoints are not `x-requires-audit` (AC13). Blessing
// them here means the Worker emits EVERY admin response — member AND webauthn — through
// `admin_response_body`, hand-rolling no JSON (the same single-seam discipline as the member views). ─

/// `POST /api/admin/webauthn/invite/resolve` 200 body — invite metadata, no PII.
impl sealed::Sealed for AdminInviteRecord {}
impl AuditedResponse for AdminInviteRecord {}
/// `POST /api/admin/webauthn/credentials/lookup` 200 body — the active credential (opaque COSE
/// public key + counter), no PII.
impl sealed::Sealed for AdminCredential {}
impl AuditedResponse for AdminCredential {}
/// `POST /api/admin/webauthn/register-complete` 200 body — the server-derived admin id only.
impl sealed::Sealed for AdminRegisterCompleteResult {}
impl AuditedResponse for AdminRegisterCompleteResult {}

/// Serialize an admin response body — the **single seam** the T09 router emits admin responses
/// through (P4: PII-response serialization single-sourced here). The `R: AuditedResponse + Serialize`
/// bound is the I5 gate: a bare [`MemberDetailView`](crate::MemberDetailView), or any un-blessed
/// type, fails to compile here, so the router cannot send member PII except as a
/// [`PiiDisclosure`] (which it could only obtain from an audited read).
pub fn admin_response_body<R: AuditedResponse + Serialize>(
    response: &R,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(response)
}
