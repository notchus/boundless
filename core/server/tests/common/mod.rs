//! Shared in-memory test doubles + builders for the `core::server` auth engine (spec 001 T07).
//!
//! The orchestration is the functional core; these are the ports it calls. The store is a plain
//! single-threaded model — its "atomic" methods are trivially atomic — so the tests exercise the
//! *contract* the Postgres twin must honor (the true DB-level TOCTOU proof is a deferred T07-shell
//! integration test). `#![allow(dead_code)]`: this module is compiled into every test binary and
//! not every binary uses every helper — standard for `tests/common` (it is *in* tests, so the
//! forbidden-patterns "no allow(dead_code) outside tests" does not apply).
#![allow(dead_code)]

use std::collections::HashMap;

use boundless_auth::{
    DeviceBinding, FixedClock, RefreshPresentation, Session, SessionFamilyStatus, UnixSeconds,
    VersionRequirement,
};
use boundless_crypto::{
    onboarding_code_hash, phone_lookup_hash, recovery_code_hash, refresh_token_hash,
    refresh_token_matches, AdminInvitationTokenHash, CodeHash, GroupKey, HmacKey, Kek, Nonce,
    PhoneLookupHash, RefreshTokenHash, HASH_LEN, KEY_LEN, NONCE_LEN,
};
use boundless_domain::{
    AccessToken, AdminInvitationToken, AppVersion, ClientVersion, DeviceToken, MemberId,
    OnboardingCode, Platform, RecoveryCode, RefreshToken, Role, SessionFamilyId,
};
use boundless_server_core::{
    generate_group_key, normalize_phone, AdminAlert, AdminAlertSink, AdminProvisioningStore,
    AuditEntry, AuditStore, AuthConfig, AuthService, AuthStore, BindRequest, BindResponse,
    DelegatedKeyStore, DeviceStore, DuplicateDisclosureAudit, EditApplied, FamilyInfo,
    InsertMemberOutcome, ManifestPointer, MemberConfig, MemberEditWrite, MemberRecord,
    MemberService, MemberStore, NewMemberWrite, OnboardingCodeRow, OnboardingStatus,
    RecoveryCodeRow, RecoveryRequest, RecoveryResponse, RefreshClassification, RefreshRequest,
    RefreshResponse, SecretSource, SignInRequest, SignInResponse, StoreBackend, StoredMemberPii,
    StoredMemberSummary,
};
use std::future::Future;
use uuid::Uuid;

/// The fully-wired engine the tests drive.
pub type TestService = AuthService<MemStore, RecordingSink, SeqSecrets, FixedClock>;

/// A fixed per-instance HMAC secret (production reads this from Secrets Store).
pub fn key() -> HmacKey {
    HmacKey::from_bytes([0x42; 32])
}

/// A deterministic member id from a small integer.
pub fn member_id(n: u128) -> MemberId {
    MemberId::from_uuid(Uuid::from_u128(n))
}

/// The default version requirement: `client_min_version = 1.0.0`, recommended `1.2.0`.
pub fn requirement() -> VersionRequirement {
    VersionRequirement::new(AppVersion::new(1, 0, 0), AppVersion::new(1, 2, 0))
}

/// A `ClientVersion` for the given platform + semantic version.
pub fn client_version(platform: Platform, major: u32, minor: u32, patch: u32) -> ClientVersion {
    ClientVersion {
        platform,
        app_version: AppVersion::new(major, minor, patch),
    }
}

/// A supported iOS client at the recommended version.
pub fn ios_current() -> ClientVersion {
    client_version(Platform::Ios, 1, 2, 0)
}

/// An iOS client below `client_min_version` (O4 degradation).
pub fn ios_below_min() -> ClientVersion {
    client_version(Platform::Ios, 0, 9, 0)
}

/// Build the engine around a populated store at a fixed instant.
pub fn service(store: MemStore, clock_secs: i64) -> TestService {
    AuthService::new(
        store,
        RecordingSink::default(),
        SeqSecrets::default(),
        FixedClock::at_secs(clock_secs),
        AuthConfig::new(
            key(),
            requirement(),
            ManifestPointer::new("manifest:v1:index", "manifest:v1:"),
        ),
    )
}

/// Block the current thread until `fut` completes — the test driver for the now-`async` endpoint
/// methods. `MemStore`'s futures are always ready (no real I/O), so `pollster`'s minimal executor
/// is sufficient and a full async runtime is unnecessary in the host unit tests.
pub fn block_on<F: Future>(fut: F) -> F::Output {
    pollster::block_on(fut)
}

/// Test-only blocking wrappers over the now-`async` endpoint methods (the async-port bridge,
/// ADR-0020). `MemStore`'s store futures are always ready, so these [`block_on`] + `unwrap` the
/// `Result` (whose error is `Infallible` for the in-memory backend) — keeping the synchronous
/// `#[test]` bodies readable. The `_ok` suffix marks "drive the endpoint to completion, expect Ok".
pub trait EndpointsBlocking {
    /// Run [`AuthService::sign_in`] to completion.
    fn sign_in_ok(&mut self, req: SignInRequest) -> SignInResponse;
    /// Run [`AuthService::bind_device`] to completion.
    fn bind_device_ok(&mut self, req: BindRequest) -> BindResponse;
    /// Run [`AuthService::refresh`] to completion.
    fn refresh_ok(&mut self, req: RefreshRequest) -> RefreshResponse;
    /// Run [`AuthService::recovery_rebind`] to completion.
    fn recovery_rebind_ok(&mut self, req: RecoveryRequest) -> RecoveryResponse;
}

impl EndpointsBlocking for TestService {
    fn sign_in_ok(&mut self, req: SignInRequest) -> SignInResponse {
        block_on(self.sign_in(req)).unwrap()
    }
    fn bind_device_ok(&mut self, req: BindRequest) -> BindResponse {
        block_on(self.bind_device(req)).unwrap()
    }
    fn refresh_ok(&mut self, req: RefreshRequest) -> RefreshResponse {
        block_on(self.refresh(req)).unwrap()
    }
    fn recovery_rebind_ok(&mut self, req: RecoveryRequest) -> RecoveryResponse {
        block_on(self.recovery_rebind(req)).unwrap()
    }
}

// === AdminAlertSink ===============================================================

/// Records every emitted alert for assertions.
#[derive(Default)]
pub struct RecordingSink {
    pub alerts: Vec<AdminAlert>,
}

impl RecordingSink {
    /// How many alerts of the given kind were emitted.
    pub fn count_kind(&self, kind: boundless_server_core::AlertKind) -> usize {
        self.alerts.iter().filter(|a| a.kind() == kind).count()
    }
}

impl AdminAlertSink for RecordingSink {
    fn emit(&mut self, alert: AdminAlert) {
        self.alerts.push(alert);
    }
}

// === SecretSource ================================================================

/// A deterministic secret source — distinct values per call (so a rotated credential differs
/// from the prior). Production supplies a CSPRNG (T07-shell).
#[derive(Default)]
pub struct SeqSecrets {
    n: u64,
}

impl SecretSource for SeqSecrets {
    fn fresh_refresh(&mut self) -> RefreshToken {
        self.n += 1;
        RefreshToken::new(format!("refresh-{}", self.n))
    }
    fn fresh_access(&mut self) -> AccessToken {
        self.n += 1;
        AccessToken::new(format!("access-{}", self.n))
    }
    fn fresh_recovery_code(&mut self) -> RecoveryCode {
        self.n += 1;
        RecoveryCode::new(format!("recovery-{}", self.n))
    }
    fn fresh_admin_invitation(&mut self) -> AdminInvitationToken {
        self.n += 1;
        AdminInvitationToken::new(format!("admin-invite-{}", self.n))
    }
    fn fresh_onboarding_code(&mut self) -> OnboardingCode {
        self.n += 1;
        OnboardingCode::new(format!("onboarding-{}", self.n))
    }
    fn fresh_nonce(&mut self) -> Nonce {
        // Deterministic but distinct per call (the counter in the low bytes) so a test never reuses
        // a nonce; production draws from a CSPRNG (RngSecretSource).
        self.n += 1;
        let mut bytes = [0u8; NONCE_LEN];
        bytes[..8].copy_from_slice(&self.n.to_le_bytes());
        Nonce::from_bytes(bytes)
    }
    fn fresh_group_key(&mut self) -> GroupKey {
        // Deterministic but distinct per call (the counter in the low bytes); production draws a
        // CSPRNG key (RngSecretSource). The test double never wraps a real Group, so this is purely
        // a distinct-key stand-in.
        self.n += 1;
        let mut bytes = [0u8; KEY_LEN];
        bytes[..8].copy_from_slice(&self.n.to_le_bytes());
        GroupKey::from_bytes(bytes)
    }
}

// === AuthStore (in-memory) =======================================================

struct OnbState {
    code_hash: CodeHash,
    expires_at: UnixSeconds,
    max_attempts: u32,
}

struct Family {
    id: SessionFamilyId,
    member: MemberId,
    status: SessionFamilyStatus,
    current: RefreshTokenHash,
    superseded: Vec<RefreshTokenHash>,
    access_expires_at: UnixSeconds,
}

struct DeviceEntry {
    binding: DeviceBinding,
    token: DeviceToken,
    invalidated: bool,
}

/// An in-memory Admin registration invitation — models an `admin_invitations` row: at most one
/// **live** (non-`consumed`) per admin, superseded (consumed) on re-issue. Single-threaded ⇒ the
/// supersede-then-insert is trivially atomic (it models the Postgres contract).
struct InviteRow {
    admin_id: MemberId,
    token_hash: AdminInvitationTokenHash,
    expires_at: UnixSeconds,
    consumed: bool,
}

/// An in-memory [`AuthStore`]. Single-threaded, so its `*_if_live` / rotate methods are trivially
/// atomic — they model the contract the Postgres twin must honor.
#[derive(Default)]
pub struct MemStore {
    members: HashMap<[u8; HASH_LEN], MemberRecord>,
    onboarding: HashMap<MemberId, OnbState>,
    recovery: HashMap<MemberId, CodeHash>,
    families: Vec<Family>,
    devices: Vec<DeviceEntry>,
    next_family: u128,
    admins: Vec<MemberId>,
    invitations: Vec<InviteRow>,
    next_admin: u128,
}

impl MemStore {
    /// A fresh, empty store.
    pub fn new() -> Self {
        Self {
            next_family: 1_000,
            next_admin: 9_000,
            ..Self::default()
        }
    }

    // --- builders (called before the service takes ownership) ---

    /// Register a member reachable by `raw_phone` (canonicalized through `normalize_phone`, the
    /// same path the request uses, so the lookup hashes match).
    pub fn add_member(&mut self, member: MemberId, raw_phone: &str, roles: Vec<Role>) {
        let phone = normalize_phone(raw_phone).expect("test phone is valid E.164");
        let hash = phone_lookup_hash(&key(), &phone);
        self.members.insert(
            *hash.as_bytes(),
            MemberRecord {
                member_id: member,
                roles,
            },
        );
    }

    /// Give `member` a live Onboarding Code (`raw_code`) with a TTL and attempt ceiling.
    pub fn add_onboarding(&mut self, member: MemberId, raw_code: &str, expires_at: i64, max: u32) {
        let code_hash = onboarding_code_hash(&key(), &OnboardingCode::new(raw_code));
        self.onboarding.insert(
            member,
            OnbState {
                code_hash,
                expires_at: UnixSeconds::new(expires_at),
                max_attempts: max,
            },
        );
    }

    /// Give `member` a live Recovery Code (`raw_code`).
    pub fn add_recovery(&mut self, member: MemberId, raw_code: &str) {
        let code_hash = recovery_code_hash(&key(), &RecoveryCode::new(raw_code));
        self.recovery.insert(member, code_hash);
    }

    /// Create a session family with a current refresh credential and zero or more rotated-away
    /// (superseded) ones, returning its id.
    pub fn add_family(
        &mut self,
        member: MemberId,
        current_raw: &str,
        superseded_raw: &[&str],
        status: SessionFamilyStatus,
        access_expires_at: i64,
    ) -> SessionFamilyId {
        let id = SessionFamilyId::from_uuid(Uuid::from_u128(self.next_family));
        self.next_family += 1;
        let current = refresh_token_hash(&key(), &RefreshToken::new(current_raw));
        let superseded = superseded_raw
            .iter()
            .map(|r| refresh_token_hash(&key(), &RefreshToken::new(*r)))
            .collect();
        self.families.push(Family {
            id,
            member,
            status,
            current,
            superseded,
            access_expires_at: UnixSeconds::new(access_expires_at),
        });
        id
    }

    /// Bind a device for `member` directly (test setup of a "prior device").
    pub fn add_device(&mut self, member: MemberId, platform: Platform, version: AppVersion) {
        self.devices.push(DeviceEntry {
            binding: DeviceBinding::new(member, platform, version),
            token: DeviceToken::new("prior-token"),
            invalidated: false,
        });
    }

    // --- assertions ---

    /// The lifecycle status of a family, if present.
    pub fn family_status(&self, id: SessionFamilyId) -> Option<SessionFamilyStatus> {
        self.families.iter().find(|f| f.id == id).map(|f| f.status)
    }

    /// The number of a member's still-active (non-invalidated) device bindings.
    pub fn active_device_count(&self, member: MemberId) -> usize {
        self.devices
            .iter()
            .filter(|d| d.binding.member_id == member && !d.invalidated)
            .count()
    }

    /// Whether a specific binding has been invalidated.
    pub fn is_invalidated(&self, binding: &DeviceBinding) -> bool {
        self.devices
            .iter()
            .any(|d| &d.binding == binding && d.invalidated)
    }

    /// Whether a pending Admin with this id was provisioned.
    pub fn admin_exists(&self, admin: MemberId) -> bool {
        self.admins.contains(&admin)
    }

    /// The number of a member's **live** (non-consumed) Admin invitations — the one-live invariant
    /// (AC16): exactly 1 after a create or a re-issue, 0 for an unknown admin.
    pub fn live_invitations(&self, admin: MemberId) -> usize {
        self.invitations
            .iter()
            .filter(|i| i.admin_id == admin && !i.consumed)
            .count()
    }

    /// Total invitation rows ever minted for an admin (live + superseded) — to prove a re-issue
    /// adds a row and supersedes (rather than mutating in place).
    pub fn total_invitations(&self, admin: MemberId) -> usize {
        self.invitations
            .iter()
            .filter(|i| i.admin_id == admin)
            .count()
    }

    /// The at-rest hash of the admin's live invitation, if any (the type has no `PartialEq`, so a
    /// test verifies a minted token against it with the constant-time `*_matches`).
    pub fn live_invitation_hash(&self, admin: MemberId) -> Option<AdminInvitationTokenHash> {
        self.invitations
            .iter()
            .find(|i| i.admin_id == admin && !i.consumed)
            .map(|i| i.token_hash.clone())
    }

    /// The server-time expiry of the admin's live invitation, if any (AC16 TTL).
    pub fn live_invitation_expiry(&self, admin: MemberId) -> Option<UnixSeconds> {
        self.invitations
            .iter()
            .find(|i| i.admin_id == admin && !i.consumed)
            .map(|i| i.expires_at)
    }
}

impl StoreBackend for MemStore {
    // The in-memory stub cannot fail (single-threaded, no I/O), so its store error is uninhabited;
    // the orchestration's `?` on a `Result<_, Infallible>` is the identity (no real error path).
    type Error = std::convert::Infallible;
}

impl AuthStore for MemStore {
    async fn find_member_by_phone(
        &mut self,
        hash: &PhoneLookupHash,
    ) -> Result<Option<MemberRecord>, Self::Error> {
        Ok(self.members.get(hash.as_bytes()).cloned())
    }

    async fn load_live_onboarding(
        &mut self,
        member: MemberId,
    ) -> Result<Option<OnboardingCodeRow>, Self::Error> {
        Ok(self.onboarding.get(&member).map(|s| OnboardingCodeRow {
            code_hash: s.code_hash.clone(),
            expires_at: s.expires_at,
            max_attempts: s.max_attempts,
        }))
    }

    async fn consume_onboarding_if_live(
        &mut self,
        member: MemberId,
        _now: UnixSeconds,
    ) -> Result<bool, Self::Error> {
        // Atomic compare-and-remove: present ⇒ consume (true); absent ⇒ lost the race (false).
        Ok(self.onboarding.remove(&member).is_some())
    }

    async fn classify_refresh(
        &mut self,
        presented: &RefreshToken,
        key: &HmacKey,
    ) -> Result<RefreshClassification, Self::Error> {
        for fam in &self.families {
            if refresh_token_matches(key, presented, &fam.current) {
                return Ok(RefreshClassification {
                    presentation: RefreshPresentation::Current,
                    family: Some(FamilyInfo {
                        id: fam.id,
                        status: fam.status,
                        member: fam.member,
                    }),
                });
            }
            if fam
                .superseded
                .iter()
                .any(|h| refresh_token_matches(key, presented, h))
            {
                return Ok(RefreshClassification {
                    presentation: RefreshPresentation::Superseded,
                    family: Some(FamilyInfo {
                        id: fam.id,
                        status: fam.status,
                        member: fam.member,
                    }),
                });
            }
        }
        Ok(RefreshClassification {
            presentation: RefreshPresentation::Unknown,
            family: None,
        })
    }

    async fn rotate_session(
        &mut self,
        family: SessionFamilyId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<Session, Self::Error> {
        let fam = self
            .families
            .iter_mut()
            .find(|f| f.id == family)
            .expect("rotate target exists");
        let old = std::mem::replace(&mut fam.current, new_refresh_hash);
        fam.superseded.push(old);
        fam.access_expires_at = access_expires_at;
        Ok(Session {
            member_id: fam.member,
            family_id: fam.id,
            access_token_expires_at: access_expires_at,
            family_status: fam.status,
        })
    }

    async fn revoke_family(
        &mut self,
        family: SessionFamilyId,
        _now: UnixSeconds,
    ) -> Result<(), Self::Error> {
        if let Some(fam) = self.families.iter_mut().find(|f| f.id == family) {
            fam.status = SessionFamilyStatus::Revoked;
        }
        Ok(())
    }

    async fn create_session_family(
        &mut self,
        member: MemberId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<Session, Self::Error> {
        let id = SessionFamilyId::from_uuid(Uuid::from_u128(self.next_family));
        self.next_family += 1;
        self.families.push(Family {
            id,
            member,
            status: SessionFamilyStatus::Active,
            current: new_refresh_hash,
            superseded: Vec::new(),
            access_expires_at,
        });
        Ok(Session {
            member_id: member,
            family_id: id,
            access_token_expires_at: access_expires_at,
            family_status: SessionFamilyStatus::Active,
        })
    }

    async fn load_live_recovery(
        &mut self,
        member: MemberId,
    ) -> Result<Option<RecoveryCodeRow>, Self::Error> {
        Ok(self.recovery.get(&member).map(|h| RecoveryCodeRow {
            code_hash: h.clone(),
        }))
    }

    async fn consume_and_rotate_recovery(
        &mut self,
        member: MemberId,
        fresh_hash: CodeHash,
        _now: UnixSeconds,
    ) -> Result<bool, Self::Error> {
        // Atomic consume-and-rotate: present ⇒ replace with the fresh code (true); absent ⇒ false.
        Ok(
            if let std::collections::hash_map::Entry::Occupied(mut e) = self.recovery.entry(member)
            {
                e.insert(fresh_hash);
                true
            } else {
                false
            },
        )
    }
}

impl DeviceStore for MemStore {
    async fn current_device_bindings(
        &mut self,
        member: MemberId,
    ) -> Result<Vec<DeviceBinding>, Self::Error> {
        Ok(self
            .devices
            .iter()
            .filter(|d| d.binding.member_id == member && !d.invalidated)
            .map(|d| d.binding)
            .collect())
    }

    async fn invalidate_device(
        &mut self,
        binding: &DeviceBinding,
        _now: UnixSeconds,
    ) -> Result<(), Self::Error> {
        for d in &mut self.devices {
            if &d.binding == binding {
                d.invalidated = true;
            }
        }
        Ok(())
    }

    async fn register_device(
        &mut self,
        binding: &DeviceBinding,
        token: &DeviceToken,
        _now: UnixSeconds,
    ) -> Result<(), Self::Error> {
        // Upsert on the `(member, platform, app_version)` tuple (the Postgres twin's composite
        // PK, I4): replace any existing entry for this exact binding rather than duplicating
        // (sec-audit M1 — the in-memory stub now honors the "insert/replace" port contract).
        self.devices.retain(|d| &d.binding != binding);
        self.devices.push(DeviceEntry {
            binding: *binding,
            token: token.clone(),
            invalidated: false,
        });
        Ok(())
    }
}

impl AdminProvisioningStore for MemStore {
    async fn create_pending_admin_with_invitation(
        &mut self,
        token_hash: AdminInvitationTokenHash,
        expires_at: UnixSeconds,
    ) -> Result<MemberId, Self::Error> {
        let admin = MemberId::from_uuid(Uuid::from_u128(self.next_admin));
        self.next_admin += 1;
        self.admins.push(admin);
        self.invitations.push(InviteRow {
            admin_id: admin,
            token_hash,
            expires_at,
            consumed: false,
        });
        Ok(admin)
    }

    async fn reissue_admin_invitation(
        &mut self,
        admin_id: MemberId,
        token_hash: AdminInvitationTokenHash,
        expires_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<bool, Self::Error> {
        if !self.admins.contains(&admin_id) {
            return Ok(false); // unknown admin → no-op, never a stray invitation
        }
        // Atomic supersede-then-insert: consume the prior live invite(s), then add the fresh one,
        // so at most one stays live (the Postgres partial-unique index, modeled).
        for inv in &mut self.invitations {
            if inv.admin_id == admin_id && !inv.consumed {
                inv.consumed = true;
            }
        }
        self.invitations.push(InviteRow {
            admin_id,
            token_hash,
            expires_at,
            consumed: false,
        });
        Ok(true)
    }
}

// === Member-management store (in-memory) =========================================================

/// A fixed KEK for the member tests (production loads this from Secrets Store, ADR-0025 R3). A fresh
/// instance each call — `Kek` is move-only + zeroizes on drop, but two instances with the same bytes
/// wrap/unwrap interchangeably, so the bootstrap KEK and the `MemberConfig` KEK can be separate.
pub fn member_kek() -> Kek {
    Kek::from_bytes([0x55; KEY_LEN])
}

/// The fully-wired member-management engine the tests drive.
pub type TestMemberService = MemberService<MemMemberStore, SeqSecrets, FixedClock>;

/// Build the member engine around a store at a fixed instant, with [`member_kek`] as the KEK and the
/// shared per-instance HMAC [`key`].
pub fn member_service(store: MemMemberStore, clock_secs: i64) -> TestMemberService {
    MemberService::new(
        store,
        SeqSecrets::default(),
        FixedClock::at_secs(clock_secs),
        MemberConfig {
            hmac_key: key(),
            kek: member_kek(),
        },
    )
}

/// A bootstrapped member store **plus** the plaintext per-Group key it was seeded with — so a test
/// can decrypt exactly what the service stored. The KEK is [`member_kek`], so a [`member_service`]
/// over the returned store unwraps the *same* key (`GroupKey` is move-only, returned by value).
pub fn bootstrapped_store_with_key() -> (MemMemberStore, GroupKey) {
    let mut store = MemMemberStore::new();
    let mut boot_secrets = SeqSecrets::default();
    let boot = generate_group_key(&mut boot_secrets, &member_kek());
    store.wrapped_group_key = Some(boot.wrapped_key);
    (store, boot.group_key)
}

/// A stored member row (the in-memory analog of the `members` PII columns + status + concurrency token).
struct StoredMemberRow {
    name_encrypted: Vec<u8>,
    phone_encrypted: Vec<u8>,
    address_encrypted: Vec<u8>,
    phone_lookup: [u8; HASH_LEN],
    roles: Vec<Role>,
    onboarding_status: OnboardingStatus,
    updated_at: UnixSeconds,
}

/// An in-memory [`MemberStore`] + [`AuditStore`] + [`DelegatedKeyStore`]. Single-threaded, so its
/// "one transaction" methods are trivially atomic — they model the contract the Postgres twin must
/// honor (the true DB-level proof is T07). The audit **write** is folded into the PII-read /
/// duplicate-disclosure methods (I5/§7), and `recorded_audits()` is the observable channel the tests
/// assert on (and `member_list_emits_no_audit_event` asserts is untouched on the list path).
#[derive(Default)]
pub struct MemMemberStore {
    wrapped_group_key: Option<Vec<u8>>,
    members: HashMap<MemberId, StoredMemberRow>,
    phone_index: HashMap<[u8; HASH_LEN], MemberId>,
    codes: HashMap<MemberId, (CodeHash, UnixSeconds)>,
    audits: Vec<AuditEntry>,
    next_member: u128,
}

impl MemMemberStore {
    /// A fresh, empty store with a deterministic member-id base (clear of the small `member_id(n)` ids).
    pub fn new() -> Self {
        Self {
            next_member: 50_000,
            ..Self::default()
        }
    }

    /// A store seeded with a valid wrapped per-Group key (bootstrapped under [`member_kek`]) — the
    /// common starting point so issuance can encrypt. Use [`MemMemberStore::new`] (no key) to drive
    /// the AC12 fail-closed path.
    pub fn bootstrapped() -> Self {
        let mut store = Self::new();
        let mut boot_secrets = SeqSecrets::default();
        let boot = generate_group_key(&mut boot_secrets, &member_kek());
        store.wrapped_group_key = Some(boot.wrapped_key);
        store
    }

    // --- assertions / accessors ---

    /// Every recorded audit row (the observable I5 channel).
    pub fn recorded_audits(&self) -> &[AuditEntry] {
        &self.audits
    }

    /// The number of members currently stored.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// The member's live Onboarding Code hash + expiry, if any (the type has no `PartialEq`, so a
    /// test verifies a minted code against it with the constant-time `onboarding_code_matches`).
    pub fn live_code(&self, member: MemberId) -> Option<(CodeHash, UnixSeconds)> {
        self.codes.get(&member).map(|(h, e)| (h.clone(), *e))
    }

    /// The member's stored phone-lookup hash, rebuilt for a constant-time `phone_lookup_matches` check
    /// (AC4 — the next sign-in matches).
    pub fn stored_phone_lookup(&self, member: MemberId) -> Option<PhoneLookupHash> {
        self.members
            .get(&member)
            .map(|m| PhoneLookupHash::from_bytes(m.phone_lookup))
    }

    /// The member's stored `name_encrypted` blob (to assert re-encryption changed the nonce/ciphertext).
    pub fn stored_name_encrypted(&self, member: MemberId) -> Option<Vec<u8>> {
        self.members.get(&member).map(|m| m.name_encrypted.clone())
    }

    /// The member's stored `phone_encrypted` blob.
    pub fn stored_phone_encrypted(&self, member: MemberId) -> Option<Vec<u8>> {
        self.members.get(&member).map(|m| m.phone_encrypted.clone())
    }

    /// The member's stored `address_encrypted` blob.
    pub fn stored_address_encrypted(&self, member: MemberId) -> Option<Vec<u8>> {
        self.members
            .get(&member)
            .map(|m| m.address_encrypted.clone())
    }

    /// The member's stored role set.
    pub fn stored_roles(&self, member: MemberId) -> Option<Vec<Role>> {
        self.members.get(&member).map(|m| m.roles.clone())
    }

    /// The member's optimistic-concurrency token (to pass as `expected_updated_at` on a clean edit).
    pub fn stored_updated_at(&self, member: MemberId) -> Option<UnixSeconds> {
        self.members.get(&member).map(|m| m.updated_at)
    }

    /// Force a member's onboarding status (to drive list/detail round-trips at a non-default status).
    pub fn set_member_status(&mut self, member: MemberId, status: OnboardingStatus) {
        if let Some(m) = self.members.get_mut(&member) {
            m.onboarding_status = status;
        }
    }
}

impl StoreBackend for MemMemberStore {
    type Error = std::convert::Infallible;
}

impl MemberStore for MemMemberStore {
    async fn insert_member(
        &mut self,
        write: NewMemberWrite,
        disclosure: DuplicateDisclosureAudit,
        now: UnixSeconds,
    ) -> Result<InsertMemberOutcome, Self::Error> {
        // Phone conflict: surface the existing member's summary AND write the disclosure audit in the
        // same (trivially atomic) step — the surface-and-link disclosure can never occur unaudited.
        if let Some(&existing_id) = self.phone_index.get(write.phone_lookup.as_bytes()) {
            let existing = &self.members[&existing_id];
            self.audits.push(AuditEntry {
                timestamp: now,
                admin_id: write.created_by,
                member_id: existing_id,
                fields: disclosure.fields,
                request_id: disclosure.request_id,
            });
            return Ok(InsertMemberOutcome::DuplicatePhone(StoredMemberSummary {
                member_id: existing_id,
                name_encrypted: existing.name_encrypted.clone(),
                roles: existing.roles.clone(),
                onboarding_status: existing.onboarding_status,
            }));
        }
        let member_id = MemberId::from_uuid(Uuid::from_u128(self.next_member));
        self.next_member += 1;
        self.phone_index
            .insert(*write.phone_lookup.as_bytes(), member_id);
        self.members.insert(
            member_id,
            StoredMemberRow {
                name_encrypted: write.name_encrypted,
                phone_encrypted: write.phone_encrypted,
                address_encrypted: write.address_encrypted,
                phone_lookup: *write.phone_lookup.as_bytes(),
                roles: write.roles,
                onboarding_status: OnboardingStatus::IssuedNotOnboarded,
                updated_at: now,
            },
        );
        self.codes.insert(
            member_id,
            (write.onboarding_code_hash, write.code_expires_at),
        );
        Ok(InsertMemberOutcome::Created(member_id))
    }

    async fn list_members(&mut self) -> Result<Vec<StoredMemberSummary>, Self::Error> {
        // Exclude Admin-role members (not managed on this surface, I11). No audit (name-only).
        Ok(self
            .members
            .iter()
            .filter(|(_, m)| !m.roles.contains(&Role::Admin))
            .map(|(id, m)| StoredMemberSummary {
                member_id: *id,
                name_encrypted: m.name_encrypted.clone(),
                roles: m.roles.clone(),
                onboarding_status: m.onboarding_status,
            })
            .collect())
    }

    async fn read_member_detail_audited(
        &mut self,
        member_id: MemberId,
        audit: AuditEntry,
    ) -> Result<Option<StoredMemberPii>, Self::Error> {
        // Atomic SELECT + audit-INSERT: write the audit row only if the member exists (a not-found
        // read discloses no PII, so it writes no audit).
        let Some(m) = self.members.get(&member_id) else {
            return Ok(None);
        };
        let pii = StoredMemberPii {
            member_id,
            name_encrypted: m.name_encrypted.clone(),
            phone_encrypted: m.phone_encrypted.clone(),
            address_encrypted: m.address_encrypted.clone(),
            roles: m.roles.clone(),
            onboarding_status: m.onboarding_status,
            updated_at: m.updated_at,
        };
        self.audits.push(audit);
        Ok(Some(pii))
    }

    async fn edit_member(
        &mut self,
        member_id: MemberId,
        write: MemberEditWrite,
        expected_updated_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<EditApplied, Self::Error> {
        let Some(m) = self.members.get_mut(&member_id) else {
            return Ok(EditApplied::Stale); // gone → treated as "someone changed this" (no member deletion in v1)
        };
        if m.updated_at != expected_updated_at {
            return Ok(EditApplied::Stale);
        }
        if let Some(name) = write.name_encrypted {
            m.name_encrypted = name;
        }
        if let Some(phone) = write.phone_encrypted {
            m.phone_encrypted = phone;
        }
        if let Some(addr) = write.address_encrypted {
            m.address_encrypted = addr;
        }
        if let Some(roles) = write.roles {
            m.roles = roles;
        }
        if let Some(lookup) = write.phone_lookup {
            // Re-index: the phone changed, so the lookup hash moved (AC11 — next sign-in matches).
            let old = m.phone_lookup;
            m.phone_lookup = *lookup.as_bytes();
            self.phone_index.remove(&old);
            self.phone_index.insert(*lookup.as_bytes(), member_id);
        }
        // Re-borrow to bump the concurrency token (the `phone_index` mutation released the &mut).
        self.members
            .get_mut(&member_id)
            .expect("member present (just edited)")
            .updated_at = now;
        Ok(EditApplied::Updated)
    }

    async fn regenerate_code(
        &mut self,
        member_id: MemberId,
        new_code_hash: CodeHash,
        code_expires_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<bool, Self::Error> {
        if !self.members.contains_key(&member_id) {
            return Ok(false);
        }
        // Supersede-then-insert: replacing the live code is the in-memory analog of the partial-unique
        // "one live code per member" supersede (the real atomic SQL is T07).
        self.codes
            .insert(member_id, (new_code_hash, code_expires_at));
        Ok(true)
    }
}

impl AuditStore for MemMemberStore {
    async fn list_audit_log(
        &mut self,
        member: Option<MemberId>,
    ) -> Result<Vec<AuditEntry>, Self::Error> {
        Ok(self
            .audits
            .iter()
            .filter(|a| member.is_none_or(|m| a.member_id == m))
            .cloned()
            .collect())
    }
}

impl DelegatedKeyStore for MemMemberStore {
    async fn current_wrapped_key(&mut self) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.wrapped_group_key.clone())
    }
}
