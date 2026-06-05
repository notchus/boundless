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
//! twin is one `UPDATE … WHERE … RETURNING` / one transaction, proven at the store level
//! (`server/store/tests/integration.rs`) and end-to-end through the orchestration
//! (`server/store/tests/service_pg.rs`).
//!
//! The ports are `async` + fallible (each carries an associated [`StoreBackend::Error`]): a real
//! backend is asynchronous and can fail on transport, so [`crate::AuthService`]'s endpoint methods
//! are themselves `async` and return `Result<_, St::Error>`. Device-token persistence is a separate
//! port ([`DeviceStore`]) — see its docs for why.

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

/// The shared error of a store backend. Declared once on this supertrait so a composed
/// `St: AuthStore + DeviceStore` exposes a **single** `Error` and the orchestration's `?` unifies
/// across both ports (the in-memory stub uses [`core::convert::Infallible`]; the Postgres backend a
/// `StoreError`).
pub trait StoreBackend {
    /// The infrastructure error a store call can fail with — a DB/transport failure, **never** a
    /// business outcome (those are encoded in the engine's response types). Carries no PII (P2).
    type Error;
}

/// The persistence boundary for the member-auth engine's **session / Onboarding-Code / Recovery-
/// Code / member-lookup** state. One implementation per backend (in-memory for tests; Postgres-
/// over-Hyperdrive in the Worker). See the module docs for the atomicity contract on the mutating
/// methods.
///
/// Methods are `async` + fallible: a real backend is asynchronous and can fail on transport, and
/// the Worker drives these futures on the wasm event loop. Reads take `&mut self` because the
/// Postgres twin scopes every statement inside a per-request transaction (RLS), which needs
/// `&mut`; the in-memory twin ignores it. **Device-token persistence is a separate port**
/// ([`DeviceStore`]).
// We own every impl; the returned futures are intentionally NOT `Send`-bound, so the wasm `?Send`
// Worker can drive them and the host tests use a single-threaded executor (`pollster::block_on`).
// No code spawns these futures across threads, so the `async fn in trait` ergonomics are exactly
// what we want here — hence the targeted allow (cf. forbidden-patterns, which bans only
// `#[allow(dead_code)]`).
#[allow(async_fn_in_trait)]
pub trait AuthStore: StoreBackend {
    /// Look up a member by phone-lookup hash. The implementation must take **uniform time**
    /// whether or not a match exists (no existence/timing leak — the in-memory stub does a full
    /// constant-time scan; the Postgres twin is an indexed equality whose timing is data-
    /// independent at the application layer).
    async fn find_member_by_phone(
        &mut self,
        hash: &PhoneLookupHash,
    ) -> Result<Option<MemberRecord>, Self::Error>;

    /// The member's live Onboarding Code, if one is outstanding (`None` = none / consumed /
    /// superseded — the partial-unique index guarantees at most one live row).
    async fn load_live_onboarding(
        &mut self,
        member: MemberId,
    ) -> Result<Option<OnboardingCodeRow>, Self::Error>;

    /// Atomically consume the member's live Onboarding Code **iff still live**; returns `true`
    /// iff THIS call consumed it (`false` = already consumed/superseded — lost the race; the
    /// caller treats `false` as Consumed, never a second bind — carry-forward (a)). Postgres
    /// twin: `UPDATE onboarding_codes SET consumed_at=$now WHERE member_id=$1 AND consumed_at IS
    /// NULL AND superseded_at IS NULL RETURNING id`.
    async fn consume_onboarding_if_live(
        &mut self,
        member: MemberId,
        now: UnixSeconds,
    ) -> Result<bool, Self::Error>;

    /// Classify a presented refresh credential within its lineage, comparing constant-time
    /// against the stored hashes (`refresh_token_matches`) — never `==` (R6).
    async fn classify_refresh(
        &mut self,
        presented: &RefreshToken,
        key: &HmacKey,
    ) -> Result<RefreshClassification, Self::Error>;

    /// Atomically rotate: supersede the family's current credential and install `new_refresh_hash`
    /// as the new current one, returning the fresh [`Session`]. Valid only after a
    /// `Current`-on-`Active` classification. Postgres twin: one txn that supersedes the presented
    /// row and inserts the new current row (the partial-unique index enforces "one current per
    /// family"; supersede-then-insert — carry-forward).
    async fn rotate_session(
        &mut self,
        family: SessionFamilyId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<Session, Self::Error>;

    /// Atomically revoke the entire family (replay detected, or an admin-mediated event). A
    /// revoked family never rotates again.
    async fn revoke_family(
        &mut self,
        family: SessionFamilyId,
        now: UnixSeconds,
    ) -> Result<(), Self::Error>;

    /// Create a brand-new session family (device bind / recovery re-bind), returning its
    /// [`Session`]. The new family is `Active` with `new_refresh_hash` as its current credential.
    async fn create_session_family(
        &mut self,
        member: MemberId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<Session, Self::Error>;

    /// The member's live Recovery Code, if one is outstanding (Drivers only in practice).
    async fn load_live_recovery(
        &mut self,
        member: MemberId,
    ) -> Result<Option<RecoveryCodeRow>, Self::Error>;

    /// Atomically consume the live Recovery Code **and** install the fresh one (`fresh_hash`) as
    /// the member's new live code — rotated on use, ADR-0016 D3 — iff still live; `true` iff THIS
    /// call did it. Postgres twin: one txn (supersede prior + insert fresh).
    async fn consume_and_rotate_recovery(
        &mut self,
        member: MemberId,
        fresh_hash: CodeHash,
        now: UnixSeconds,
    ) -> Result<bool, Self::Error>;
}

/// The persistence boundary for **device-token bindings** (I4) — split out of [`AuthStore`]
/// because the Postgres `register_device` must persist a *reversibly-encrypted* device token (push
/// needs the plaintext back, so a one-way hash will not do), and that at-rest encryption primitive
/// is deferred to spec 008. Keeping it a separate port lets the Postgres [`AuthStore`] ship now
/// while its `DeviceStore` impl lands with issuance; the in-memory twin implements both.
// Same future-ergonomics rationale as `AuthStore` above.
#[allow(async_fn_in_trait)]
pub trait DeviceStore: StoreBackend {
    /// All of a member's current (non-invalidated) device bindings (I4).
    async fn current_device_bindings(
        &mut self,
        member: MemberId,
    ) -> Result<Vec<DeviceBinding>, Self::Error>;

    /// Invalidate a device token (silent — `AUTH_DEVICE_TOKEN_INVALIDATED`, never client-facing,
    /// carry-forward (e)).
    async fn invalidate_device(
        &mut self,
        binding: &DeviceBinding,
        now: UnixSeconds,
    ) -> Result<(), Self::Error>;

    /// Register (insert/replace) a device token under a binding.
    async fn register_device(
        &mut self,
        binding: &DeviceBinding,
        token: &DeviceToken,
        now: UnixSeconds,
    ) -> Result<(), Self::Error>;
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
    /// A fresh short-lived access token (**256-bit opaque-random bearer**, in production — *not*
    /// signed; verified by a constant-time keyed-HMAC store lookup, ADR-0021).
    fn fresh_access(&mut self) -> AccessToken;
    /// A fresh Driver Recovery Code (rotated on use, ADR-0016 D3).
    fn fresh_recovery_code(&mut self) -> RecoveryCode;
}
