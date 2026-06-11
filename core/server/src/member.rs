//! Admin member-management — the issuance / edit / regenerate / detail-read / audit decision layer
//! (spec 008, AC1/AC4/AC8/AC11/AC13 + partial AC5/AC6/AC10/AC12).
//!
//! [`MemberService`] is the *functional core* of the surface Sarah uses to issue, browse, view, edit,
//! and re-issue members (the producer side of the closed-group model — there is no signup, I11). Like
//! [`AuthService`](crate::AuthService) it composes the existing core decisions ([`normalize_phone`],
//! `core::crypto` hashing + secretbox, T04's [`load_group_key`] fail-closed gate) behind I/O ports
//! ([`MemberStore`], [`AuditStore`], [`DelegatedKeyStore`]) + the injected [`Clock`]. Tests supply
//! in-memory ports; the deployable Worker (T09) supplies the Postgres/Cloudflare ports.
//!
//! ## The privacy spine (what this module enforces in code, not comments)
//!
//! - **I1 / P2 — PII encrypted at rest, never logged.** Name and address are encrypted with the
//!   per-Group secretbox key the moment they are accepted ([`MemberService::issue_member`]); the
//!   stored columns are `bytea` ciphertext. The inputs carry the **tainted** [`MemberName`]/[`Address`]/
//!   [`PhoneNumber`] (no `Debug`/`Serialize`), so the raw values cannot be logged.
//! - **I3 / AC4 — phone two-fold.** The phone is stored as a keyed lookup hash ([`phone_lookup_hash`],
//!   for the AC4 sign-in match) **and** as ciphertext ([`encrypt_field`], for an audited display read);
//!   the plaintext is normalized in-core ([`normalize_phone`], single-source P4) and dropped.
//! - **I5 / AC7 — audited reads.** Any decision that decrypts a member's PII for display
//!   ([`MemberService::read_detail`], the duplicate-phone disclosure) emits an [`AuditEntry`] **in the
//!   same store transaction** as the ciphertext read (the atomicity is a port contract — see
//!   [`MemberStore::read_member_detail_audited`]). The audit row records field **names**
//!   ([`AuditField`]), never values (AC9).
//! - **AC8 — the list projection carries no PII.** [`MemberSummary`] holds a plain display `String`
//!   name + roles + status and **is** `Serialize`; because the tainted types are *not* `Serialize`, a
//!   tainted field could not compile into it (the AC8 compile guarantee). Listing decrypts only the
//!   name (the P2-sensitive unit is the name+address *pair*), so it is **not** an audited read.
//! - **AC10 / I11 — no admin creation.** Issuance takes [`IssuableRole`] (`Rider`/`Driver` only), so
//!   issuing an `Admin` is *unrepresentable* at the type boundary (the [`DeveloperAuthority`]
//!   discipline). The wire `Vec<Role>` → `Vec<IssuableRole>` conversion ([`issuable_roles`]) is the one
//!   place an `Admin` is rejected ([`AdminRoleForbidden`] → `ADMIN_MEMBER_ROLE_FORBIDDEN`).
//! - **AC12 — fail closed without a Group key.** Every field-touching method loads the per-Group key
//!   **first** via T04's [`load_group_key`]; a missing/corrupt key yields `ADMIN_GROUP_KEY_MISSING` and
//!   no member row is written or read (no `unwrap()` on the key load).
//!
//! ## The two-type `MemberDetail` split (parity R1 — the highest-likelihood P2/drift seam)
//!
//! The core [`MemberDetail`] holds tainted `MemberName`/`PhoneNumber`/`Address`, so it is — by
//! construction — **not** `Serialize`/`Debug`. The wire [`MemberDetailView`] is a *separate*
//! all-`String` `Serialize` DTO the core builds via [`MemberDetail::to_wire`] (the sanctioned
//! `expose_secret` boundary) only after the audit row is committed. The Worker must serialize the
//! view with serde (never a hand-rolled re-projection), so the field names are single-sourced in this
//! drift-tracked crate (the seam the spec-001 `ManifestPointer` miss came through).
//!
//! [`AuthService`]: crate::AuthService
//! [`load_group_key`]: crate::load_group_key
//! [`DeveloperAuthority`]: crate::DeveloperAuthority
//! [`normalize_phone`]: crate::normalize_phone

use boundless_auth::{Clock, UnixSeconds};
use boundless_crypto::{
    decrypt_field, encrypt_field, onboarding_code_hash, phone_lookup_hash, CodeHash, GroupKey,
    HmacKey, Kek, PhoneLookupHash,
};
use boundless_domain::{Address, MemberId, MemberName, OnboardingCode, PhoneNumber, Role};
use serde::Serialize;

use crate::audited::PiiDisclosure;
use crate::bootstrap::load_group_key;
use crate::phone::normalize_phone;
use crate::ports::{SecretSource, StoreBackend};

/// The Onboarding-Code TTL minted at issuance: 72 hours (ADR-0016; matches the `bind` gate default).
/// Validated against **server time** (the injected [`Clock`]), never the admin's device clock.
pub const ONBOARDING_CODE_TTL_SECS: i64 = 72 * 60 * 60;

/// Decode a just-decrypted PII field into a display `String`. The bytes are **valid UTF-8 by
/// construction** — every `*_encrypted` blob this service reads was encrypted by it from a
/// `String`-backed tainted type, and a successful Poly1305 decrypt returns exactly those bytes — so
/// `from_utf8_lossy` can never actually substitute `U+FFFD` here; it is the panic-free decode, not a
/// tolerance of arbitrary input. If a future external writer ever populates `*_encrypted` with
/// non-UTF-8 bytes, switch to `from_utf8` and map the error into the `GroupKeyMissing`/decrypt-failure
/// collapse (tracked in `DEFERRED.md` → T09).
fn decode_field(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// ===== Roles ======================================================================================

/// A role a member may be **issued** with — `Rider` or `Driver` only.
///
/// `Admin` is deliberately *not* a variant: Admins are provisioned only by the Developer (I11), so
/// issuing one through this surface is **unrepresentable** rather than merely rejected at runtime —
/// the same "enforce through code" discipline as [`DeveloperAuthority`](crate::DeveloperAuthority).
/// The wire `Vec<Role>` is converted via [`issuable_roles`], the one place an `Admin` is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssuableRole {
    /// A group member who needs transportation to the Gathering.
    Rider,
    /// A group member who drives others to the Gathering.
    Driver,
}

impl From<IssuableRole> for Role {
    fn from(role: IssuableRole) -> Self {
        match role {
            IssuableRole::Rider => Role::Rider,
            IssuableRole::Driver => Role::Driver,
        }
    }
}

/// An issuance request named a role that cannot be issued here — in practice `Role::Admin` (I11/AC10).
///
/// Maps to the stable `ADMIN_MEMBER_ROLE_FORBIDDEN` code (`docs/error-codes.md`, P12). Distinct from
/// `DEV_ADMIN_CREATE_FORBIDDEN` (the `/api/dev/*` surface) — different actor, different surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminRoleForbidden;

impl AdminRoleForbidden {
    /// The stable error code (`docs/error-codes.md`, P12).
    pub const fn error_code(self) -> &'static str {
        "ADMIN_MEMBER_ROLE_FORBIDDEN"
    }
}

/// Convert a wire `Vec<Role>` into the issuable subset, rejecting `Admin` (I11/AC10).
///
/// This is the **only** seam where an `Admin` role can be refused at issuance; once it returns
/// `Ok`, the resulting `Vec<IssuableRole>` cannot represent an Admin, so the rest of the issuance
/// path is an Admin-free type guarantee. An empty role set is **not** rejected here (that is a
/// separate validation concern) — but issuance requires at least one role, enforced by
/// [`MemberService::issue_member`].
pub fn issuable_roles(roles: &[Role]) -> Result<Vec<IssuableRole>, AdminRoleForbidden> {
    roles
        .iter()
        .map(|r| match r {
            Role::Rider => Ok(IssuableRole::Rider),
            Role::Driver => Ok(IssuableRole::Driver),
            Role::Admin => Err(AdminRoleForbidden),
        })
        .collect()
}

// ===== Projections + status =======================================================================

/// Where a member is in the issue → onboard lifecycle (spec "States and transitions"). The single
/// core source T08 mirrors into the OpenAPI `OnboardingStatus` schema — the wire spelling is the
/// `snake_case` rename pinned here (`onboarding_status_wire_casing` asserts it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStatus {
    /// Account exists, a live Onboarding Code is outstanding, no device bound.
    IssuedNotOnboarded,
    /// A device is bound (spec 001 `bind-device` succeeded); the Onboarding Code is consumed.
    Onboarded,
    /// No live code (consumed or past TTL); the admin must regenerate.
    CodeExpiredOrLost,
    /// Device replaced/revoked (the spec-001 I4 re-onboarding path).
    NeedsReonboarding,
}

/// The member-**list** projection (AC8) — **no tainted PII type**.
///
/// Carries a plain display `String` name (decrypted at the read boundary), the role set, and the
/// onboarding status. Because the tainted types are not `Serialize`, this struct deriving `Serialize`
/// is a *compile-time* guarantee that no tainted field crept in (the AC8 proof). Distinct from the
/// PII-free auth-path [`MemberRecord`](crate::MemberRecord), which must never carry a name (it flows
/// through `find_member_by_phone` into PII-free auth responses) — this one is the admin-list view.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct MemberSummary {
    /// The opaque member identity (never displayed; serializes as the canonical UUID).
    pub member_id: MemberId,
    /// The member's display name, decrypted to plain text for the admin UI (AC3 — plain `String`,
    /// never persisted in clear). Still PII for **logging** purposes: log only the redacted form.
    pub name: String,
    /// The role set established at issuance (AC13).
    pub roles: Vec<Role>,
    /// Where the member is in the onboarding lifecycle.
    pub onboarding_status: OnboardingStatus,
}

// `Debug` is hand-written to **redact the plaintext `name`** (P2): unlike the tainted types, `name`
// here is a plain `String`, so a derived `Debug` would print it under a stray `{:?}` once the Worker
// (T09) / web (T10) handle lists. Mirrors the tainted types' `redacted_summary` discipline and the
// no-`Debug` posture of the sibling `MemberDetailView`; the non-PII fields still print for diagnostics.
impl core::fmt::Debug for MemberSummary {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemberSummary")
            .field("member_id", &self.member_id)
            .field("name", &"<redacted>")
            .field("roles", &self.roles)
            .field("onboarding_status", &self.onboarding_status)
            .finish()
    }
}

/// The **core** member-detail projection — holds tainted PII, so it is *not* `Serialize`/`Debug` by
/// construction (P2). Built by decrypting a member's stored ciphertext at an audited read; converted
/// to the wire [`MemberDetailView`] via [`MemberDetail::to_wire`] only at the explicit boundary.
pub struct MemberDetail {
    /// The opaque member identity.
    pub member_id: MemberId,
    /// The decrypted name (tainted).
    pub name: MemberName,
    /// The decrypted phone (tainted).
    pub phone: PhoneNumber,
    /// The decrypted home address (tainted).
    pub address: Address,
    /// The role set.
    pub roles: Vec<Role>,
    /// The onboarding lifecycle status.
    pub onboarding_status: OnboardingStatus,
    /// The optimistic-concurrency token (the row's server-time `updated_at`).
    pub updated_at: UnixSeconds,
}

impl MemberDetail {
    /// Project to the wire DTO, revealing the tainted fields via `expose_secret` (the **sole
    /// sanctioned boundary**, mirroring `SessionMaterial`/`AdminInvitation`). `pub(crate)` (T06): the
    /// bare [`MemberDetailView`] is not a public producer — the only public PII-detail surface is
    /// `read_detail`'s [`PiiDisclosure`]`<MemberDetailView>`, built only after the audit commits. The
    /// resulting view is plain-`String`-by-necessity — it must **never** be logged.
    pub(crate) fn to_wire(&self) -> MemberDetailView {
        MemberDetailView {
            member_id: self.member_id,
            name: self.name.expose_secret().to_string(),
            phone: self.phone.expose_secret().to_string(),
            address: self.address.expose_secret().to_string(),
            roles: self.roles.clone(),
            onboarding_status: self.onboarding_status,
            updated_at: self.updated_at,
        }
    }
}

/// The **wire** member-detail DTO (the audited GET/PATCH response shape) — plain `String` fields.
/// The two-type split's wire half (T08 mirrors it into OpenAPI). Plain-`String` by necessity, so
/// unlike the tainted core [`MemberDetail`] it has no compile-time log guard: it must never be
/// logged (route errors on this path through redacted summaries, never `?view`).
///
/// **The I5 gate (T06): its fields are private and its only constructor is the `pub(crate)`
/// [`MemberDetail::to_wire`].** It keeps `Serialize` solely so [`PiiDisclosure`]`<MemberDetailView>`
/// (the audited carrier) can emit the wire body by delegation — but no code outside this crate can
/// *construct* one (private fields, no public ctor), so a future Worker cannot
/// `Response::from_json(&bare_view)` a fabricated one. The PII detail therefore reaches the wire
/// **only** as a [`PiiDisclosure`] minted after an audit row was committed. (A holder of an
/// *already-audited* disclosure can of course serialize its payload — that is the point; the gate is
/// about producing PII for the wire *without* an audit, not field-level secrecy after one. The
/// irreducible `expose_secret`+hand-rolled-JSON path is covered by I5's named second layer — T08's
/// OpenAPI-PII-handler-coverage test — + a T09 P2 lint; see `DEFERRED.md` → T06.)
#[derive(Clone, Serialize)]
pub struct MemberDetailView {
    /// The opaque member identity.
    member_id: MemberId,
    /// The member's name (plaintext — never log).
    name: String,
    /// The member's phone, E.164 (plaintext — never log).
    phone: String,
    /// The member's home address (plaintext — never log).
    address: String,
    /// The role set.
    roles: Vec<Role>,
    /// The onboarding lifecycle status.
    onboarding_status: OnboardingStatus,
    /// The optimistic-concurrency token the client echoes back on edit.
    updated_at: UnixSeconds,
}

// ===== Wire response envelopes (the admin HTTP response shapes — T08 contract / T09 Worker) ========
//
// The serializable wire bodies the T09 Worker emits, mirrored ONE-FOR-ONE by the `api/openapi.yaml`
// admin schemas (T08). Like [`MemberDetailView`] they live HERE (not the Worker) so the field names
// are single-sourced in this drift-tracked crate (P4 — the seam the spec-001 `ManifestPointer` miss
// came through), and each is blessed [`AuditedResponse`](crate::AuditedResponse) in `audited.rs` so the
// Worker serializes EVERY admin response through the sealed `admin_response_body` seam — there is no
// hand-rolled member-PII JSON in the Worker (the I5 posture). All are PII-free in the I5 sense (no
// decrypted address/phone): [`MemberSummary`] carries only a display name (and the duplicate-phone
// disclosure's audit is written atomically in the store, not here), and the show-once
// `onboarding_code` is a freshly-minted credential, not a disclosed member field. The code fields are
// still **secrets** (P2) — a constructed view must NEVER be logged. The two code-bearing views expose
// the code via `expose_secret` only inside their `new` (the single sanctioned boundary).

/// `GET /api/admin/members` → `{ members: [...] }` (AC8).
#[derive(Serialize)]
pub struct MemberListView {
    members: Vec<MemberSummary>,
}

impl MemberListView {
    /// Wrap the PII-free member summaries.
    pub fn new(members: Vec<MemberSummary>) -> Self {
        Self { members }
    }
}

/// `POST /api/admin/members` 201 → `{ member, onboarding_code, code_expires_at }` (AC1/AC5). The
/// show-once Onboarding Code is revealed via `expose_secret` inside [`new`](Self::new) (the single
/// sanctioned boundary); the constructed view holds it as a plain `String` and must NEVER be logged (P2).
#[derive(Serialize)]
pub struct MemberIssuedView {
    member: MemberSummary,
    onboarding_code: String,
    code_expires_at: UnixSeconds,
}

impl MemberIssuedView {
    /// Project an `Issued` outcome to the wire, exposing the show-once code at this boundary.
    pub fn new(
        member: MemberSummary,
        onboarding_code: &OnboardingCode,
        code_expires_at: UnixSeconds,
    ) -> Self {
        Self {
            member,
            onboarding_code: onboarding_code.expose_secret().to_string(),
            code_expires_at,
        }
    }
}

/// `POST /api/admin/members` 409 → `{ error_code, existing }` — the duplicate-phone surface-and-link
/// (I5-audited in the store atomically with the conflict detection; admin-surface-only, never on `/api/auth/*`).
#[derive(Serialize)]
pub struct DuplicatePhoneLinkView {
    error_code: &'static str,
    existing: MemberSummary,
}

impl DuplicatePhoneLinkView {
    /// Build the duplicate-phone link body around the existing member's name-only summary.
    pub fn new(existing: MemberSummary) -> Self {
        Self {
            error_code: "ADMIN_MEMBER_DUPLICATE_PHONE",
            existing,
        }
    }
}

/// `POST /api/admin/members/{id}/regenerate-code` → `{ onboarding_code, code_expires_at }` (AC6).
#[derive(Serialize)]
pub struct RegenerateCodeView {
    onboarding_code: String,
    code_expires_at: UnixSeconds,
}

impl RegenerateCodeView {
    /// Project a `Regenerated` outcome to the wire, exposing the show-once code at this boundary.
    pub fn new(onboarding_code: &OnboardingCode, code_expires_at: UnixSeconds) -> Self {
        Self {
            onboarding_code: onboarding_code.expose_secret().to_string(),
            code_expires_at,
        }
    }
}

/// `GET /api/admin/audit-log` → `{ entries: [...] }` (AC9) — field names only, never values.
#[derive(Serialize)]
pub struct AuditLogView {
    entries: Vec<AuditEntry>,
}

impl AuditLogView {
    /// Wrap the audit entries (names only).
    pub fn new(entries: Vec<AuditEntry>) -> Self {
        Self { entries }
    }
}

// ===== Audit (I5) =================================================================================

/// A member-PII field, recorded in an [`AuditEntry`] by **name**, never value (AC9 / R6). The single
/// source for both the DB `audit_log.fields text[]` write tokens and the wire `fields: string[]` read
/// array — [`AuditField::as_str`] and the `Serialize` rename produce the same lowercase token
/// (`audit_field_as_str_casing` asserts it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditField {
    /// The member's name field was read.
    Name,
    /// The member's phone field was read.
    Phone,
    /// The member's address field was read.
    Address,
}

impl AuditField {
    /// The stable lowercase token stored in `audit_log.fields` and serialized on the wire.
    pub const fn as_str(self) -> &'static str {
        match self {
            AuditField::Name => "name",
            AuditField::Phone => "phone",
            AuditField::Address => "address",
        }
    }
}

/// One admin-PII-read audit record (I5): timestamp, the acting admin, the member whose PII was read,
/// the field **names** returned, and the request id. Carries **no PII value**, so it is the AC9
/// audit-log read view itself (`Serialize`). `timestamp` is set from the service's injected [`Clock`]
/// (server time — the "admin's clock is wrong" edge); `request_id` MUST be a server-minted opaque id
/// (never client-echoed), so the persisted, admin-readable row has no PII/secret injection point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditEntry {
    /// Server-time instant of the read (epoch seconds on the wire).
    pub timestamp: UnixSeconds,
    /// The admin who performed the read (the actor, I5).
    pub admin_id: MemberId,
    /// The member whose PII was read.
    pub member_id: MemberId,
    /// The field names returned (never values, AC9).
    pub fields: Vec<AuditField>,
    /// The server-minted opaque request correlation id.
    pub request_id: String,
}

// ===== Errors + outcomes ==========================================================================

/// A member operation was rejected for a stable, registered reason (P12). Business outcomes that are
/// not rejections (issued / duplicate / stale) are carried in the outcome enums, not here — the store
/// `Error` channel stays purely infrastructural (the `core::server` idiom).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberError {
    /// The submitted phone could not be normalized to E.164 (`normalize_phone`).
    PhoneInvalid,
    /// The submitted address failed validation (empty after trimming).
    AddressInvalid,
    /// No role was selected — a member must hold at least one role (AC13, "Rider, Driver, or both").
    /// An issuance with an empty set, or an edit that would clear all roles, is rejected.
    RolesRequired,
    /// No usable per-Group encryption key (Group bootstrap incomplete, or the key/a field could not
    /// be decrypted) — fail closed (AC12). The wire collapses both "no key" and "corrupt blob/field"
    /// to one code (no oracle); the Worker logs the underlying `SecretboxError` variant (DEFERRED L1).
    GroupKeyMissing,
}

impl MemberError {
    /// The stable error code (`docs/error-codes.md`, P12).
    pub const fn error_code(self) -> &'static str {
        match self {
            MemberError::PhoneInvalid => "ADMIN_MEMBER_PHONE_INVALID",
            MemberError::AddressInvalid => "ADMIN_MEMBER_ADDRESS_INVALID",
            MemberError::RolesRequired => "ADMIN_MEMBER_ROLES_REQUIRED",
            MemberError::GroupKeyMissing => "ADMIN_GROUP_KEY_MISSING",
        }
    }
}

/// The result of [`MemberService::issue_member`]. `Issued` holds the show-once [`OnboardingCode`]
/// (tainted), so the enum is deliberately not `Serialize`: the Worker reveals the code on the wire
/// exactly once. `DuplicatePhone` is a first-class outcome (an admin-only, I5-audited surface-and-link
/// — never an error), carrying the existing member's name-only [`MemberSummary`].
pub enum IssueMemberOutcome {
    /// The member was created; the plaintext Onboarding Code is returned once with its expiry.
    Issued {
        /// The new member's list summary (no PII).
        member: MemberSummary,
        /// The single-use Onboarding Code (tainted; shown to the admin exactly once).
        onboarding_code: OnboardingCode,
        /// The code's server-side expiry (`now + `[`ONBOARDING_CODE_TTL_SECS`]).
        code_expires_at: UnixSeconds,
    },
    /// The phone is already enrolled in this Group; the existing member is surfaced (name only) and
    /// the disclosure was audited (I5). The admin resolves by editing the existing member.
    DuplicatePhone {
        /// The existing member's name-only summary (never address/phone).
        existing: MemberSummary,
    },
    /// The request was rejected for a stable reason (invalid phone/address, or no Group key).
    Rejected(MemberError),
}

impl IssueMemberOutcome {
    /// The stable error code, or `None` on a clean issue (mirrors `SignInResponse::error_code`, P12).
    pub fn error_code(&self) -> Option<&'static str> {
        match self {
            IssueMemberOutcome::Issued { .. } => None,
            IssueMemberOutcome::DuplicatePhone { .. } => Some("ADMIN_MEMBER_DUPLICATE_PHONE"),
            IssueMemberOutcome::Rejected(e) => Some(e.error_code()),
        }
    }
}

/// The result of [`MemberService::edit_member`]. Optimistic concurrency: a stale edit (the row's
/// `updated_at` moved) is rejected with no partial write (AC11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMemberOutcome {
    /// The edit applied.
    Updated,
    /// A concurrent edit moved the row (or the member is gone) — refresh and retry (`ADMIN_MEMBER_EDIT_STALE`).
    Stale,
    /// The edit was rejected for a stable reason (invalid phone/address, or no Group key).
    Rejected(MemberError),
}

impl EditMemberOutcome {
    /// The stable error code, or `None` on a clean edit (P12).
    pub fn error_code(self) -> Option<&'static str> {
        match self {
            EditMemberOutcome::Updated => None,
            EditMemberOutcome::Stale => Some("ADMIN_MEMBER_EDIT_STALE"),
            EditMemberOutcome::Rejected(e) => Some(e.error_code()),
        }
    }
}

/// The result of [`MemberService::read_detail`] (the audited PII read, AC7).
pub enum DetailRead {
    /// The member's decrypted detail, wrapped in a [`PiiDisclosure`] (T06 / the I5 gate): the only
    /// serializable PII-detail carrier, mintable only here after the audit row was committed
    /// atomically with the ciphertext read — so the Worker cannot serialize the detail without it.
    /// `Box`ed so the large carrier (view + its committed [`AuditEntry`]) does not bloat the small
    /// `NotFound`/`GroupKeyMissing` variants (`clippy::large_enum_variant`).
    Detail(Box<PiiDisclosure<MemberDetailView>>),
    /// No such member (a 404 — no PII read, no audit row written).
    NotFound,
    /// The Group key (or a stored field) could not be decrypted — fail closed (`ADMIN_GROUP_KEY_MISSING`).
    GroupKeyMissing,
}

impl DetailRead {
    /// The stable error code, or `None` for a successful read / a plain not-found (P12).
    pub fn error_code(&self) -> Option<&'static str> {
        match self {
            DetailRead::Detail(_) | DetailRead::NotFound => None,
            DetailRead::GroupKeyMissing => Some("ADMIN_GROUP_KEY_MISSING"),
        }
    }
}

/// The result of [`MemberService::regenerate_onboarding_code`] (AC6).
pub enum RegenerateOutcome {
    /// A fresh code was minted (the prior live code superseded atomically); shown once with its expiry.
    Regenerated {
        /// The fresh single-use Onboarding Code (tainted).
        onboarding_code: OnboardingCode,
        /// The fresh code's server-side expiry.
        code_expires_at: UnixSeconds,
    },
    /// No such member.
    NotFound,
}

// ===== Inputs =====================================================================================

/// An issuance request. Holds tainted PII ([`MemberName`]/[`PhoneNumber`]/[`Address`]), so it is not
/// `Serialize`/`Debug` (P2). The phone is the **raw, as-entered** value — `issue_member` canonicalizes
/// it in-core via [`normalize_phone`] (single-source P4, so the lookup hash matches sign-in's, AC4).
/// Roles are the issuable subset, so an `Admin` is unrepresentable here (I11).
pub struct IssueMemberInput {
    /// The member's name (encrypted at rest, I1/AC3).
    pub name: MemberName,
    /// The member's phone as the admin entered it (canonicalized in-core; I3/AC4).
    pub phone: PhoneNumber,
    /// The member's home address (encrypted at rest, I1/AC2).
    pub address: Address,
    /// The role set established at issuance — `Rider` and/or `Driver` (AC13; Admin unrepresentable).
    pub roles: Vec<IssuableRole>,
}

/// An edit request — each PII field is optional (only changed fields are re-validated/re-encrypted);
/// `expected_updated_at` is the optimistic-concurrency token the client loaded (AC11). Holds tainted
/// PII, so not `Serialize`. A `Some` phone is the raw value (re-normalized in-core).
pub struct EditMemberInput {
    /// New name, if changed (re-encrypted on save).
    pub name: Option<MemberName>,
    /// New raw phone, if changed (re-normalized + lookup hash recomputed so the next sign-in matches).
    pub phone: Option<PhoneNumber>,
    /// New address, if changed (re-encrypted on save).
    pub address: Option<Address>,
    /// New role set, if changed (still the issuable subset — Admin unrepresentable).
    pub roles: Option<Vec<IssuableRole>>,
    /// The `updated_at` the edit was loaded against — a stale value is rejected (`ADMIN_MEMBER_EDIT_STALE`).
    pub expected_updated_at: UnixSeconds,
}

// ===== Store ports ================================================================================

/// What [`MemberStore::insert_member`] writes for a brand-new member, in one transaction with the
/// Onboarding-Code mint (R13 atomicity). All PII is already a keyed hash or ciphertext — this struct
/// carries **no** plaintext (P2). A new member is always [`OnboardingStatus::IssuedNotOnboarded`].
pub struct NewMemberWrite {
    /// The acting admin (`created_by`, audit write-side, I5).
    pub created_by: MemberId,
    /// The role set (`text[]`); always the issuable subset (no Admin).
    pub roles: Vec<Role>,
    /// The keyed phone-lookup hash (I3; the `(group_id, phone_lookup_hash)` uniqueness key).
    pub phone_lookup: PhoneLookupHash,
    /// The encrypted phone (`nonce ‖ ciphertext`, I3 display side).
    pub phone_encrypted: Vec<u8>,
    /// The encrypted name (`nonce ‖ ciphertext`, I1/AC3).
    pub name_encrypted: Vec<u8>,
    /// The encrypted address (`nonce ‖ ciphertext`, I1/AC2).
    pub address_encrypted: Vec<u8>,
    /// The at-rest hash of the minted Onboarding Code (AC5).
    pub onboarding_code_hash: CodeHash,
    /// The code's server-side expiry.
    pub code_expires_at: UnixSeconds,
}

/// The audit context [`MemberStore::insert_member`] stamps onto the **duplicate-phone disclosure** row
/// *iff* the insert conflicts — written in the **same transaction** as the conflict detection +
/// existing-member fetch, so the surface-and-link disclosure can never occur without its audit row
/// (I5). Unused on a clean create (a create is a write, audited via `created_by`, not a read).
pub struct DuplicateDisclosureAudit {
    /// The server-minted opaque request id for the disclosure audit.
    pub request_id: String,
    /// The field names disclosed by the surface-and-link (`[Name]` — name only).
    pub fields: Vec<AuditField>,
}

/// A member's stored summary row (the store-shape behind [`MemberSummary`]): the encrypted name + the
/// PII-free fields. The orchestration decrypts `name_encrypted` to build the [`MemberSummary`].
pub struct StoredMemberSummary {
    /// The opaque member identity.
    pub member_id: MemberId,
    /// The encrypted name (`nonce ‖ ciphertext`).
    pub name_encrypted: Vec<u8>,
    /// The role set.
    pub roles: Vec<Role>,
    /// The onboarding lifecycle status.
    pub onboarding_status: OnboardingStatus,
}

/// A member's full stored PII row (the store-shape behind [`MemberDetail`]) — the three ciphertexts +
/// the PII-free fields. Returned only by the **audited** read [`MemberStore::read_member_detail_audited`].
pub struct StoredMemberPii {
    /// The opaque member identity.
    pub member_id: MemberId,
    /// The encrypted name.
    pub name_encrypted: Vec<u8>,
    /// The encrypted phone.
    pub phone_encrypted: Vec<u8>,
    /// The encrypted address.
    pub address_encrypted: Vec<u8>,
    /// The role set.
    pub roles: Vec<Role>,
    /// The onboarding lifecycle status.
    pub onboarding_status: OnboardingStatus,
    /// The optimistic-concurrency token.
    pub updated_at: UnixSeconds,
}

/// The new ciphertext/hashes [`MemberStore::edit_member`] applies for changed fields only (`None` =
/// leave as-is). No plaintext (P2). The store updates the `Some` fields, bumps `updated_at`, and
/// commits only if the row's `updated_at` still equals the caller's `expected_updated_at` (AC11).
pub struct MemberEditWrite {
    /// New encrypted name, if changed.
    pub name_encrypted: Option<Vec<u8>>,
    /// New keyed phone-lookup hash, if the phone changed (so the next sign-in matches, AC11).
    pub phone_lookup: Option<PhoneLookupHash>,
    /// New encrypted phone, if changed.
    pub phone_encrypted: Option<Vec<u8>>,
    /// New encrypted address, if changed.
    pub address_encrypted: Option<Vec<u8>>,
    /// New role set, if changed.
    pub roles: Option<Vec<Role>>,
}

/// The outcome of [`MemberStore::insert_member`] (the atomic member + code write, or a phone conflict).
pub enum InsertMemberOutcome {
    /// The member was created; carries the new opaque id.
    Created(MemberId),
    /// The phone is already enrolled; carries the existing member's summary row (fetched + audited in
    /// the same transaction as the conflict detection).
    DuplicatePhone(StoredMemberSummary),
}

/// The outcome of [`MemberStore::edit_member`] (optimistic concurrency).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditApplied {
    /// The update applied.
    Updated,
    /// The row's `updated_at` moved (concurrent edit) or the member is gone — no write.
    Stale,
}

/// The persistence boundary for **member rows + audit rows** (spec 008). One impl per backend
/// (in-memory for tests; Postgres-over-Hyperdrive in the Worker). As in [`AuthStore`](crate::AuthStore),
/// **atomicity is a port contract**: the methods whose docs say "one transaction" must be one txn in
/// the Postgres twin (proven at T07), and the orchestration relies on it (notably the audit INSERT
/// being committed *with* the ciphertext SELECT — I5/§7). The audit **write** lives here, co-located
/// with the PII SELECT it must be atomic with; [`AuditStore`] is read-only **by design** — there is no
/// standalone audit-write method, so "read PII without auditing" is unrepresentable at the port.
// Same non-`Send` AFIT rationale as `AuthStore` (the wasm `?Send` Worker drives these futures).
#[allow(async_fn_in_trait)]
pub trait MemberStore: StoreBackend {
    /// Create a member + mint its Onboarding Code **in one transaction** (R13). On a phone conflict
    /// (`(group_id, phone_lookup)` unique), instead fetch the existing member's summary **and** write
    /// the `disclosure` audit row **in the same transaction**, returning [`InsertMemberOutcome::DuplicatePhone`]
    /// — so the surface-and-link disclosure (I5-audited, admin-only) is atomic with the conflict
    /// detection. `disclosure` is unused on a clean create.
    async fn insert_member(
        &mut self,
        write: NewMemberWrite,
        disclosure: DuplicateDisclosureAudit,
        now: UnixSeconds,
    ) -> Result<InsertMemberOutcome, Self::Error>;

    /// The group's member list (Rider/Driver members; **excludes Admins** — they are not managed on
    /// this surface, I11). Returns the encrypted name + PII-free fields; the orchestration decrypts the
    /// name into a [`MemberSummary`]. **Not** an audited read (name-alone is not the P2-sensitive unit).
    async fn list_members(&mut self) -> Result<Vec<StoredMemberSummary>, Self::Error>;

    /// Read a member's full PII ciphertext **and** write the `audit` row in **one transaction** (I5/§7
    /// — a failed audit INSERT rolls back the read, so PII is never read without an audit row).
    /// Returns `None` (no audit written) if the member does not exist.
    async fn read_member_detail_audited(
        &mut self,
        member_id: MemberId,
        audit: AuditEntry,
    ) -> Result<Option<StoredMemberPii>, Self::Error>;

    /// Apply an edit under optimistic concurrency: update the `Some` fields and bump `updated_at`
    /// **only if** the row's current `updated_at` equals `expected_updated_at`; otherwise
    /// [`EditApplied::Stale`] with no write (AC11). One `UPDATE … WHERE updated_at = $expected`.
    async fn edit_member(
        &mut self,
        member_id: MemberId,
        write: MemberEditWrite,
        expected_updated_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<EditApplied, Self::Error>;

    /// Supersede the member's prior live Onboarding Code and install `new_code_hash` as the new live
    /// one **in one transaction** (supersede-then-insert; at most one live code per member, AC6).
    /// Returns `true` iff the member existed.
    async fn regenerate_code(
        &mut self,
        member_id: MemberId,
        new_code_hash: CodeHash,
        code_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<bool, Self::Error>;
}

/// The **read-only** audit-log query port (AC9). The audit **write** is co-located in
/// [`MemberStore::read_member_detail_audited`]/`insert_member` (by the atomicity contract) — this port
/// has no write method by design, so the only way PII is read is through a method that also audits.
// Same non-`Send` AFIT rationale as `AuthStore`.
#[allow(async_fn_in_trait)]
pub trait AuditStore: StoreBackend {
    /// The audit log for the group, optionally filtered to one member. Returns field **names**, never
    /// values (AC9), so reading the log is not itself a recursive PII read.
    async fn list_audit_log(
        &mut self,
        member: Option<MemberId>,
    ) -> Result<Vec<AuditEntry>, Self::Error>;
}

/// The persistence boundary for the per-Group key envelope (ADR-0025). Returns the **wrapped** bytes
/// only — the [`Kek`] never enters the store layer (it lives in [`MemberConfig`]); the orchestration
/// unwraps via T04's [`load_group_key`], reusing the proven fail-closed gate (AC12). T09 may cache.
// Same non-`Send` AFIT rationale as `AuthStore`.
#[allow(async_fn_in_trait)]
pub trait DelegatedKeyStore: StoreBackend {
    /// The group's KEK-wrapped secretbox key (`delegated_keys.wrapped_key`), or `None` if the Group
    /// was never bootstrapped (issuance then fails closed — AC12).
    async fn current_wrapped_key(&mut self) -> Result<Option<Vec<u8>>, Self::Error>;
}

// ===== The service ================================================================================

/// Per-instance member-management configuration: the HMAC secret (for the phone-lookup + code hashes)
/// and the KEK (to unwrap the per-Group key). Not `Debug` — both are unloggable key material (P2).
pub struct MemberConfig {
    /// The per-instance HMAC secret (from Secrets Store in production).
    pub hmac_key: HmacKey,
    /// The Key-Encryption-Key that wraps the per-Group secretbox key (from Secrets Store, ADR-0025 R3).
    pub kek: Kek,
}

/// The member-management engine (spec 008). Generic over its ports + clock so tests use in-memory
/// doubles and the Worker uses Cloudflare/Postgres impls. **Standalone** — it does not reuse
/// [`AuthService`](crate::AuthService) (different ports, no alert sink / hub), keeping that engine
/// untouched. The single `store: St` implements all three member ports, so `St::Error` unifies the
/// `?` across them (the [`StoreBackend`] supertrait, as `AuthStore + DeviceStore` already do).
pub struct MemberService<St, Sec, Clk> {
    /// The persistence boundary (members, audit rows, the wrapped Group key).
    pub store: St,
    /// The injected source of fresh secrets (Onboarding Codes + field nonces; no ambient randomness).
    pub secrets: Sec,
    /// The injected time source — **server time** in production (the "admin's clock is wrong" edge).
    pub clock: Clk,
    /// Per-instance config (HMAC secret + KEK).
    pub config: MemberConfig,
}

impl<St, Sec, Clk> MemberService<St, Sec, Clk>
where
    St: MemberStore + AuditStore + DelegatedKeyStore,
    Sec: SecretSource,
    Clk: Clock,
{
    /// Assemble the engine from its parts.
    pub fn new(store: St, secrets: Sec, clock: Clk, config: MemberConfig) -> Self {
        Self {
            store,
            secrets,
            clock,
            config,
        }
    }

    /// Load + unwrap the per-Group key, failing closed (AC12). Reuses T04's [`load_group_key`] gate:
    /// `None` (never bootstrapped) **and** a corrupt/wrong-KEK blob both yield `GroupKeyMissing`.
    async fn load_group_key(&mut self) -> Result<Result<GroupKey, MemberError>, St::Error> {
        let wrapped = self.store.current_wrapped_key().await?;
        Ok(load_group_key(wrapped.as_deref(), &self.config.kek)
            .map_err(|_| MemberError::GroupKeyMissing))
    }

    /// Encrypt one PII field under `key` with a fresh injected-CSPRNG nonce (R1 — a fresh nonce per
    /// field, never reused). The stored blob is `nonce ‖ ciphertext`.
    fn encrypt(&mut self, key: &GroupKey, plaintext: &[u8]) -> Vec<u8> {
        let nonce = self.secrets.fresh_nonce();
        encrypt_field(plaintext, key, &nonce)
    }

    /// Issue a new member (AC1/AC4/AC5/AC12). Order: validate phone (in-core normalize) → validate
    /// address → load the Group key (fail closed) → compute the two-fold phone + encrypt name/address
    /// → mint the Onboarding Code → atomic member+code insert. A phone conflict surfaces-and-links the
    /// existing member (name only), audited (I5). `actor_admin` is the `created_by` write-side actor.
    pub async fn issue_member(
        &mut self,
        actor_admin: MemberId,
        input: IssueMemberInput,
        request_id: String,
    ) -> Result<IssueMemberOutcome, St::Error> {
        let now = self.clock.now();

        // A member must hold at least one role (AC13 — "Rider, Driver, or both"); an empty set is
        // invalid input, like a bad phone/address. Reject before composing any write.
        if input.roles.is_empty() {
            return Ok(IssueMemberOutcome::Rejected(MemberError::RolesRequired));
        }
        // Validate + canonicalize the phone in-core (single-source P4; AC4 lookup-hash match).
        let phone = match normalize_phone(input.phone.expose_secret()) {
            Ok(p) => p,
            Err(_) => return Ok(IssueMemberOutcome::Rejected(MemberError::PhoneInvalid)),
        };
        // Validate the address (non-empty after trimming).
        if input.address.expose_secret().trim().is_empty() {
            return Ok(IssueMemberOutcome::Rejected(MemberError::AddressInvalid));
        }
        // Load the per-Group key first — fail closed before any write (AC12).
        let group_key = match self.load_group_key().await? {
            Ok(k) => k,
            Err(e) => return Ok(IssueMemberOutcome::Rejected(e)),
        };

        // Two-fold phone (I3/AC4) + field encryption (I1).
        let phone_lookup = phone_lookup_hash(&self.config.hmac_key, &phone);
        let phone_encrypted = self.encrypt(&group_key, phone.expose_secret().as_bytes());
        let name_encrypted = self.encrypt(&group_key, input.name.expose_secret().as_bytes());
        let address_encrypted = self.encrypt(&group_key, input.address.expose_secret().as_bytes());

        // Mint the Onboarding Code (AC5) — server-time TTL.
        let code = self.secrets.fresh_onboarding_code();
        let onboarding_code_hash = onboarding_code_hash(&self.config.hmac_key, &code);
        let code_expires_at = now.saturating_add_secs(ONBOARDING_CODE_TTL_SECS);

        let roles: Vec<Role> = input.roles.iter().copied().map(Role::from).collect();
        let write = NewMemberWrite {
            created_by: actor_admin,
            roles: roles.clone(),
            phone_lookup,
            phone_encrypted,
            name_encrypted,
            address_encrypted,
            onboarding_code_hash,
            code_expires_at,
        };
        let disclosure = DuplicateDisclosureAudit {
            request_id,
            fields: vec![AuditField::Name],
        };

        match self.store.insert_member(write, disclosure, now).await? {
            InsertMemberOutcome::Created(member_id) => {
                // We hold the plaintext name (the input), so the summary needs no decrypt.
                let member = MemberSummary {
                    member_id,
                    name: input.name.expose_secret().to_string(),
                    roles,
                    onboarding_status: OnboardingStatus::IssuedNotOnboarded,
                };
                Ok(IssueMemberOutcome::Issued {
                    member,
                    onboarding_code: code,
                    code_expires_at,
                })
            }
            InsertMemberOutcome::DuplicatePhone(existing) => {
                // The disclosure audit was written atomically with the conflict fetch; decrypt the
                // existing name with the same Group key to build the name-only link summary.
                match decrypt_field(&existing.name_encrypted, &group_key) {
                    Ok(name_bytes) => {
                        let name = decode_field(&name_bytes);
                        Ok(IssueMemberOutcome::DuplicatePhone {
                            existing: MemberSummary {
                                member_id: existing.member_id,
                                name,
                                roles: existing.roles,
                                onboarding_status: existing.onboarding_status,
                            },
                        })
                    }
                    Err(_) => Ok(IssueMemberOutcome::Rejected(MemberError::GroupKeyMissing)),
                }
            }
        }
    }

    /// The member list (AC8) — decrypts each member's name into a PII-free [`MemberSummary`]. Loads
    /// the Group key (fail closed); **emits no audit event** (name-alone is not an audited read).
    /// Returns `Err(GroupKeyMissing)` if the key (or a name field) cannot be decrypted.
    pub async fn list_members(
        &mut self,
    ) -> Result<Result<Vec<MemberSummary>, MemberError>, St::Error> {
        let group_key = match self.load_group_key().await? {
            Ok(k) => k,
            Err(e) => return Ok(Err(e)),
        };
        let rows = self.store.list_members().await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            match decrypt_field(&row.name_encrypted, &group_key) {
                Ok(name_bytes) => out.push(MemberSummary {
                    member_id: row.member_id,
                    name: decode_field(&name_bytes),
                    roles: row.roles,
                    onboarding_status: row.onboarding_status,
                }),
                Err(_) => return Ok(Err(MemberError::GroupKeyMissing)),
            }
        }
        Ok(Ok(out))
    }

    /// Read a member's full detail for display (AC7) — an **audited** read. Order (§7): load the Group
    /// key (fail closed, no audit) → build the [`AuditEntry`] (server time, fields = name+phone+address)
    /// → atomic ciphertext-SELECT + audit-INSERT → decrypt → wire view. A not-found member writes no
    /// audit row (`NotFound`); a decrypt failure after the audit commit fails closed (`GroupKeyMissing`)
    /// leaving at most a benign audit row (never a missing one).
    pub async fn read_detail(
        &mut self,
        actor_admin: MemberId,
        member_id: MemberId,
        request_id: String,
    ) -> Result<DetailRead, St::Error> {
        let now = self.clock.now();
        let group_key = match self.load_group_key().await? {
            Ok(k) => k,
            Err(_) => return Ok(DetailRead::GroupKeyMissing),
        };
        let audit = AuditEntry {
            timestamp: now,
            admin_id: actor_admin,
            member_id,
            fields: vec![AuditField::Name, AuditField::Phone, AuditField::Address],
            request_id,
        };
        // Clone the audit for the store (which persists it atomically with the SELECT); the original
        // is carried into the `PiiDisclosure` so the Worker has the committed record to log, and so
        // the disclosure type-level witnesses that an audit accompanied the read (I5).
        let Some(stored) = self
            .store
            .read_member_detail_audited(member_id, audit.clone())
            .await?
        else {
            return Ok(DetailRead::NotFound);
        };
        let (Ok(name), Ok(phone), Ok(address)) = (
            decrypt_field(&stored.name_encrypted, &group_key),
            decrypt_field(&stored.phone_encrypted, &group_key),
            decrypt_field(&stored.address_encrypted, &group_key),
        ) else {
            return Ok(DetailRead::GroupKeyMissing);
        };
        let detail = MemberDetail {
            member_id: stored.member_id,
            name: MemberName::new(decode_field(&name)),
            phone: PhoneNumber::new(decode_field(&phone)),
            address: Address::new(decode_field(&address)),
            roles: stored.roles,
            onboarding_status: stored.onboarding_status,
            updated_at: stored.updated_at,
        };
        Ok(DetailRead::Detail(Box::new(PiiDisclosure::new(
            audit,
            detail.to_wire(),
        ))))
    }

    /// Edit a member (AC11) — re-validate/re-encrypt changed fields; a phone change recomputes the
    /// keyed lookup hash (so the next sign-in matches) **and** re-encrypts the phone. Optimistic
    /// concurrency on `expected_updated_at`. Loads the Group key (fail closed) only if a PII field
    /// changed. Editing is **not** an audited read here — it returns only the outcome (the UI re-fetches
    /// detail via the audited `read_detail` if it wants to display the result).
    pub async fn edit_member(
        &mut self,
        member_id: MemberId,
        input: EditMemberInput,
        // Reserved for the T09 Worker's request-correlation / structured-log path; editing is not an
        // audited read here (it returns no detail), so the decision layer records no audit row and the
        // id is deliberately unused — kept on the signature for forward-stability with the audited paths.
        _request_id: String,
    ) -> Result<EditMemberOutcome, St::Error> {
        let now = self.clock.now();

        // Changing the role set to empty would leave a roleless member — invalid (AC13). `None` =
        // "don't change roles" is fine; only an explicit empty set is rejected.
        if matches!(&input.roles, Some(roles) if roles.is_empty()) {
            return Ok(EditMemberOutcome::Rejected(MemberError::RolesRequired));
        }
        // Normalize a changed phone in-core (AC4 hash match) before touching the store.
        let normalized_phone = match &input.phone {
            Some(raw) => match normalize_phone(raw.expose_secret()) {
                Ok(p) => Some(p),
                Err(_) => return Ok(EditMemberOutcome::Rejected(MemberError::PhoneInvalid)),
            },
            None => None,
        };
        if let Some(addr) = &input.address {
            if addr.expose_secret().trim().is_empty() {
                return Ok(EditMemberOutcome::Rejected(MemberError::AddressInvalid));
            }
        }

        // Encrypt the changed PII fields (only if any changed, so an unchanged-PII edit — e.g. role
        // only — does not require the Group key).
        let needs_key =
            input.name.is_some() || normalized_phone.is_some() || input.address.is_some();
        let group_key = if needs_key {
            match self.load_group_key().await? {
                Ok(k) => Some(k),
                Err(e) => return Ok(EditMemberOutcome::Rejected(e)),
            }
        } else {
            None
        };

        let name_encrypted = match (&input.name, &group_key) {
            (Some(name), Some(key)) => Some(self.encrypt(key, name.expose_secret().as_bytes())),
            _ => None,
        };
        let (phone_lookup, phone_encrypted) = match (&normalized_phone, &group_key) {
            (Some(phone), Some(key)) => (
                Some(phone_lookup_hash(&self.config.hmac_key, phone)),
                Some(self.encrypt(key, phone.expose_secret().as_bytes())),
            ),
            _ => (None, None),
        };
        let address_encrypted = match (&input.address, &group_key) {
            (Some(addr), Some(key)) => Some(self.encrypt(key, addr.expose_secret().as_bytes())),
            _ => None,
        };
        let roles = input
            .roles
            .map(|rs| rs.into_iter().map(Role::from).collect::<Vec<Role>>());

        let write = MemberEditWrite {
            name_encrypted,
            phone_lookup,
            phone_encrypted,
            address_encrypted,
            roles,
        };
        Ok(
            match self
                .store
                .edit_member(member_id, write, input.expected_updated_at, now)
                .await?
            {
                EditApplied::Updated => EditMemberOutcome::Updated,
                EditApplied::Stale => EditMemberOutcome::Stale,
            },
        )
    }

    /// Regenerate a member's Onboarding Code (AC6) — mint a fresh code, supersede the prior live one
    /// atomically (at most one live code per member). The plaintext is returned once with its expiry.
    pub async fn regenerate_onboarding_code(
        &mut self,
        member_id: MemberId,
    ) -> Result<RegenerateOutcome, St::Error> {
        let now = self.clock.now();
        let code = self.secrets.fresh_onboarding_code();
        let new_code_hash = onboarding_code_hash(&self.config.hmac_key, &code);
        let code_expires_at = now.saturating_add_secs(ONBOARDING_CODE_TTL_SECS);
        Ok(
            if self
                .store
                .regenerate_code(member_id, new_code_hash, code_expires_at, now)
                .await?
            {
                RegenerateOutcome::Regenerated {
                    onboarding_code: code,
                    code_expires_at,
                }
            } else {
                RegenerateOutcome::NotFound
            },
        )
    }

    /// Read the group's audit log (AC9), optionally filtered to one member. Field **names**, not
    /// values — reading it is not itself a recursive PII read.
    pub async fn read_audit_log(
        &mut self,
        member: Option<MemberId>,
    ) -> Result<Vec<AuditEntry>, St::Error> {
        self.store.list_audit_log(member).await
    }
}
