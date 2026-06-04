//! The I/O boundary of the auth engine: **port traits** + the plain record types they move.
//!
//! The orchestration ([`crate::AuthService`]) is the *functional core* — it composes the
//! `core::auth` decisions and calls these ports to load state and commit. Tests supply
//! in-memory ports; the deployable Worker (T07-shell) supplies Postgres/Cloudflare ports.
//!
//! **Atomicity is a port contract.** Each mutating method the security model needs to be atomic
//! carries an atomicity note in its doc, and the orchestration relies on it (e.g. it treats a
//! `false` from [`AuthStore::consume_onboarding_if_live`] as "lost the race → already consumed",
//! never a second bind). The in-memory stub is trivially atomic (single-threaded); the Postgres
//! twin must be one `UPDATE … WHERE … RETURNING` / one transaction (the true DB-level TOCTOU
//! proof is a deferred T07-shell integration test — `DEFERRED.md`).

use boundless_auth::{
    DeviceBinding, RefreshPresentation, Session, SessionFamilyStatus, UnixSeconds,
};
use boundless_crypto::{CodeHash, HmacKey, PhoneLookupHash, RefreshTokenHash};
use boundless_domain::{
    AccessToken, DeviceToken, MemberId, RecoveryCode, RefreshToken, Role, SessionFamilyId,
};

use crate::alerts::AdminAlert;

/// A member as the auth paths need it — **PII-free**: the phone exists only as a lookup hash in
/// the store, never here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRecord {
    /// The opaque member identity (a (Group, Person) pair; never displayed).
    pub member_id: MemberId,
    /// The roles the member holds (a person may be both Rider and Driver).
    pub roles: Vec<Role>,
}

impl MemberRecord {
    /// Whether the member holds the Driver role (gates self-serve recovery, AC19).
    pub fn is_driver(&self) -> bool {
        self.roles.contains(&Role::Driver)
    }

    /// The role used to gate self-serve recovery (AC19): [`Role::Driver`] iff the member drives,
    /// so a Rider-only member yields `NotAvailable` before any secret comparison.
    pub fn recovery_role(&self) -> Role {
        if self.is_driver() {
            Role::Driver
        } else {
            Role::Rider
        }
    }
}

/// The persisted bits of a member's **live** Onboarding Code — `load_live_onboarding` returns
/// only the non-consumed, non-superseded row (the partial-unique index guarantees at most one),
/// so the consumed/superseded flags are not carried here; single-use is enforced atomically by
/// [`AuthStore::consume_onboarding_if_live`]. The challenge's `recent_attempts` field is also not
/// here — it is supplied per-request by the GroupHub attempt window ([`crate::hub`]), so the
/// rate-limit count lives in one place. A live row may still be **expired** (server-time gated by
/// `evaluate_onboarding_code`), so the TTL is carried.
#[derive(Clone)]
pub struct OnboardingCodeRow {
    /// HMAC-SHA256 of the issued code (`core::crypto`).
    pub code_hash: CodeHash,
    /// Server-side TTL boundary (default 72h).
    pub expires_at: UnixSeconds,
    /// The per-window attempt ceiling (default 5).
    pub max_attempts: u32,
}

/// The persisted bits of a Driver's **live** Recovery Code (no TTL — rotated on use, ADR-0016
/// D3). As with [`OnboardingCodeRow`], only the live row is returned; single-use is enforced
/// atomically by [`AuthStore::consume_and_rotate_recovery`].
#[derive(Clone)]
pub struct RecoveryCodeRow {
    /// HMAC-SHA256 of the issued Recovery Code (`core::crypto`).
    pub code_hash: CodeHash,
}

/// What the session store learned about a session family while classifying a presented refresh
/// credential. `None` when the credential matched no live family ([`RefreshPresentation::Unknown`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyInfo {
    /// The family the presented credential belongs to.
    pub id: SessionFamilyId,
    /// The family's lifecycle status.
    pub status: SessionFamilyStatus,
    /// The member who owns the family (for the AC15 admin alert on replay).
    pub member: MemberId,
}

/// The result of classifying a presented refresh credential (the input to the rotation policy
/// `boundless_auth::evaluate_refresh`). The constant-time hash compare lives in the store impl
/// (`refresh_token_matches`); the *policy* on the classification lives in the core (P4).
pub struct RefreshClassification {
    /// How the credential relates to its family (current / superseded / unknown).
    pub presentation: RefreshPresentation,
    /// The family, when one was found (absent only for [`RefreshPresentation::Unknown`]).
    pub family: Option<FamilyInfo>,
}

/// Freshly-minted session material returned to the client. **Holds tainted secrets**, so it is
/// deliberately not `Debug`/`Serialize`: the Worker reveals them only at the wire boundary
/// (`expose_secret`), and the platforms store the refresh credential in the secure store
/// (§10-F via `boundless_auth::required_refresh_store`).
pub struct SessionMaterial {
    /// The PII-free session descriptor (safe to log/serialize on its own).
    pub session: Session,
    /// The short-lived bearer access token (~15 min).
    pub access: AccessToken,
    /// The long-lived, rotating refresh credential.
    pub refresh: RefreshToken,
}

/// An opaque per-source rate-limit key the Worker derives from the connection (e.g. a hash of
/// the client IP) — **never PII**. Used to throttle rejected refreshes ([`crate::hub`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceKey(pub u64);

/// The persistence boundary for the member-auth engine. One trait, one implementation per
/// backend (in-memory for tests; Postgres-over-Hyperdrive in the Worker, T07-shell). See the
/// module docs for the atomicity contract on the mutating methods.
pub trait AuthStore {
    /// Look up a member by phone-lookup hash. The implementation must take **uniform time**
    /// whether or not a match exists (no existence/timing leak — the in-memory stub does a full
    /// constant-time scan; the Postgres twin is an indexed equality whose timing is data-
    /// independent at the application layer).
    fn find_member_by_phone(&self, hash: &PhoneLookupHash) -> Option<MemberRecord>;

    /// The member's live Onboarding Code, if one is outstanding (`None` = none / consumed /
    /// superseded — the partial-unique index guarantees at most one live row).
    fn load_live_onboarding(&self, member: MemberId) -> Option<OnboardingCodeRow>;

    /// Atomically consume the member's live Onboarding Code **iff still live**; returns `true`
    /// iff THIS call consumed it (`false` = already consumed/superseded — lost the race; the
    /// caller treats `false` as Consumed, never a second bind — carry-forward (a)). Postgres
    /// twin: `UPDATE onboarding_codes SET consumed_at=$now WHERE member_id=$1 AND consumed_at IS
    /// NULL AND superseded_at IS NULL RETURNING id`.
    fn consume_onboarding_if_live(&mut self, member: MemberId, now: UnixSeconds) -> bool;

    /// Classify a presented refresh credential within its lineage, comparing constant-time
    /// against the stored hashes (`refresh_token_matches`) — never `==` (R6).
    fn classify_refresh(&self, presented: &RefreshToken, key: &HmacKey) -> RefreshClassification;

    /// Atomically rotate: supersede the family's current credential and install `new_refresh_hash`
    /// as the new current one, returning the fresh [`Session`]. Valid only after a
    /// `Current`-on-`Active` classification. Postgres twin: one txn that supersedes the presented
    /// row and inserts the new current row (the partial-unique index enforces "one current per
    /// family"; supersede-then-insert — carry-forward).
    fn rotate_session(
        &mut self,
        family: SessionFamilyId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Session;

    /// Atomically revoke the entire family (replay detected, or an admin-mediated event). A
    /// revoked family never rotates again.
    fn revoke_family(&mut self, family: SessionFamilyId, now: UnixSeconds);

    /// Create a brand-new session family (device bind / recovery re-bind), returning its
    /// [`Session`]. The new family is `Active` with `new_refresh_hash` as its current credential.
    fn create_session_family(
        &mut self,
        member: MemberId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Session;

    /// The member's live Recovery Code, if one is outstanding (Drivers only in practice).
    fn load_live_recovery(&self, member: MemberId) -> Option<RecoveryCodeRow>;

    /// Atomically consume the live Recovery Code **and** install the fresh one (`fresh_hash`) as
    /// the member's new live code — rotated on use, ADR-0016 D3 — iff still live; `true` iff THIS
    /// call did it. Postgres twin: one txn (supersede prior + insert fresh).
    fn consume_and_rotate_recovery(
        &mut self,
        member: MemberId,
        fresh_hash: CodeHash,
        now: UnixSeconds,
    ) -> bool;

    /// All of a member's current (non-invalidated) device bindings (I4).
    fn current_device_bindings(&self, member: MemberId) -> Vec<DeviceBinding>;

    /// Invalidate a device token (silent — `AUTH_DEVICE_TOKEN_INVALIDATED`, never client-facing,
    /// carry-forward (e)).
    fn invalidate_device(&mut self, binding: &DeviceBinding, now: UnixSeconds);

    /// Register (insert/replace) a device token under a binding.
    fn register_device(&mut self, binding: &DeviceBinding, token: &DeviceToken, now: UnixSeconds);
}

/// Where admin alerts are delivered (Cloudflare Queues in the Worker; a `Vec` in tests).
pub trait AdminAlertSink {
    /// Emit one (already-deduped, PII-free) admin alert.
    fn emit(&mut self, alert: AdminAlert);
}

/// The source of freshly-minted secrets. The core forbids ambient randomness, so credential
/// generation is injected: the Worker supplies a CSPRNG (T07-shell), tests a deterministic
/// sequence. Each call must return a fresh, unguessable value in production.
pub trait SecretSource {
    /// A fresh refresh credential (256-bit opaque, in production).
    fn fresh_refresh(&mut self) -> RefreshToken;
    /// A fresh short-lived access token (signed, in production).
    fn fresh_access(&mut self) -> AccessToken;
    /// A fresh Driver Recovery Code (rotated on use, ADR-0016 D3).
    fn fresh_recovery_code(&mut self) -> RecoveryCode;
}
