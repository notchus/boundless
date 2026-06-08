//! Scaffold in-memory ports for the Worker skeleton (spec 001 **T07-shell-B slice 1**) — **NOT
//! production**.
//!
//! A deliberately-minimal `wrangler dev` / test stand-in so the skeleton can drive the *real*
//! [`AuthService::sign_in`](boundless_server_core::AuthService) end-to-end (P4) before the
//! Postgres-over-Hyperdrive `PgAuthStore` + the CSPRNG [`RngSecretSource`] are wired (DEFERRED.md →
//! T07-shell-B). Only [`ScaffoldStore::find_member_by_phone`] carries real data — one seeded demo
//! member — because this slice wires only the sign-in route (and the GroupHub rate-limit route,
//! which goes through the DO, not the store). The other store methods are inert: this scaffold holds
//! no codes/sessions, so they return "nothing"/"lost the race", and the two session-*minting*
//! methods return [`ScaffoldError`] (this stand-in never mints sessions — the wired routes never
//! call them). The slice that swaps in `PgAuthStore` + `RngSecretSource` **deletes this file.**
//!
//! [`RngSecretSource`]: boundless_server_core::RngSecretSource

use std::collections::HashMap;

use boundless_auth::{Clock, RefreshPresentation, Session, UnixSeconds};
use boundless_crypto::{phone_lookup_hash, CodeHash, HmacKey, PhoneLookupHash, RefreshTokenHash};
use boundless_domain::{
    AccessToken, AdminInvitationToken, MemberId, RecoveryCode, RefreshToken, Role, SessionFamilyId,
};
use boundless_server_core::{
    normalize_phone, AdminAlert, AdminAlertSink, AuthStore, MemberRecord, OnboardingCodeRow,
    RecoveryCodeRow, RefreshClassification, SecretSource, StoreBackend,
};
use uuid::Uuid;

/// A fixed dev HMAC key. **Not a secret** — this scaffold serves only the seeded demo member;
/// production loads the per-instance key from Secrets Store (forbidden-patterns: no hardcoded
/// secrets). The same key seeds the demo member and configures [`AuthConfig`], so the lookup
/// hashes match.
///
/// [`AuthConfig`]: boundless_server_core::AuthConfig
const SCAFFOLD_HMAC_KEY: [u8; 32] = [0x7b; 32];

/// The dev HMAC key, shared by the seeded member and the live [`AuthConfig`].
pub fn scaffold_key() -> HmacKey {
    HmacKey::from_bytes(SCAFFOLD_HMAC_KEY)
}

/// The seeded demo member's phone — the miniflare test drives a deterministic `member_matched`
/// with this number and `phone_not_on_file` with any other. Uses NANP's reserved fictional block
/// `555-01XX` (guaranteed non-routable; sec-audit F3), matching the `fixtures/compat/**` phones.
pub const DEMO_PHONE: &str = "+15555550100";

/// The seeded demo member's opaque id (arbitrary fixed UUID — never surfaced to the client, I6).
const DEMO_MEMBER_U128: u128 = 0x0000_0000_0000_0000_0000_0000_0000_0001;

/// The scaffold's infrastructure error. Returned only by the session-minting methods this
/// stand-in does not support; the wired routes (sign-in) never call them, so it never reaches the
/// wire.
#[derive(Debug)]
pub struct ScaffoldError;

/// An in-memory [`AuthStore`] holding a single seeded member (keyed by phone-lookup-hash bytes,
/// the same shape the Postgres `bytea` lookup column uses — [`PhoneLookupHash`] has no `PartialEq`
/// by design, so matching is on its `as_bytes()`).
pub struct ScaffoldStore {
    members: HashMap<[u8; 32], MemberRecord>,
}

impl ScaffoldStore {
    /// A fresh store with the one demo member seeded.
    pub fn new() -> Self {
        let mut members = HashMap::new();
        let phone = normalize_phone(DEMO_PHONE).expect("demo phone is valid E.164");
        let hash = phone_lookup_hash(&scaffold_key(), &phone);
        members.insert(
            *hash.as_bytes(),
            MemberRecord {
                member_id: MemberId::from_uuid(Uuid::from_u128(DEMO_MEMBER_U128)),
                roles: vec![Role::Rider],
            },
        );
        Self { members }
    }
}

impl StoreBackend for ScaffoldStore {
    type Error = ScaffoldError;
}

impl AuthStore for ScaffoldStore {
    async fn find_member_by_phone(
        &mut self,
        hash: &PhoneLookupHash,
    ) -> Result<Option<MemberRecord>, Self::Error> {
        Ok(self.members.get(hash.as_bytes()).cloned())
    }

    // — below: inert for this slice (no code/session data in the scaffold) —

    async fn load_live_onboarding(
        &mut self,
        _member: MemberId,
    ) -> Result<Option<OnboardingCodeRow>, Self::Error> {
        Ok(None)
    }

    async fn consume_onboarding_if_live(
        &mut self,
        _member: MemberId,
        _now: UnixSeconds,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    async fn classify_refresh(
        &mut self,
        _presented: &RefreshToken,
        _key: &HmacKey,
    ) -> Result<RefreshClassification, Self::Error> {
        Ok(RefreshClassification {
            presentation: RefreshPresentation::Unknown,
            family: None,
        })
    }

    async fn rotate_session(
        &mut self,
        _family: SessionFamilyId,
        _new_refresh_hash: RefreshTokenHash,
        _access_expires_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<Session, Self::Error> {
        Err(ScaffoldError)
    }

    async fn revoke_family(
        &mut self,
        _family: SessionFamilyId,
        _now: UnixSeconds,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn create_session_family(
        &mut self,
        _member: MemberId,
        _new_refresh_hash: RefreshTokenHash,
        _access_expires_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<Session, Self::Error> {
        Err(ScaffoldError)
    }

    async fn load_live_recovery(
        &mut self,
        _member: MemberId,
    ) -> Result<Option<RecoveryCodeRow>, Self::Error> {
        Ok(None)
    }

    async fn consume_and_rotate_recovery(
        &mut self,
        _member: MemberId,
        _fresh_hash: CodeHash,
        _now: UnixSeconds,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

/// Buffers emitted admin alerts so the sign-in handler can drain them to the `ADMIN_ALERTS` Queue
/// after the call (the deduped, PII-free alerts the core decides to emit, §10-E).
#[derive(Default)]
pub struct BufferSink {
    alerts: Vec<AdminAlert>,
}

impl BufferSink {
    /// Take the alerts emitted during the last endpoint call (the handler forwards them to the
    /// `ADMIN_ALERTS` Queue), leaving the buffer empty.
    pub fn drain(&mut self) -> Vec<AdminAlert> {
        core::mem::take(&mut self.alerts)
    }
}

impl AdminAlertSink for BufferSink {
    fn emit(&mut self, alert: AdminAlert) {
        self.alerts.push(alert);
    }
}

/// A fixed [`SecretSource`] — never exercised by the wired sign-in route (which mints no
/// credentials). Production injects [`RngSecretSource`] over a getrandom-backed CSPRNG (ADR-0021).
///
/// [`RngSecretSource`]: boundless_server_core::RngSecretSource
#[derive(Default)]
pub struct ScaffoldSecrets;

impl SecretSource for ScaffoldSecrets {
    fn fresh_refresh(&mut self) -> RefreshToken {
        RefreshToken::new("scaffold-refresh")
    }
    fn fresh_access(&mut self) -> AccessToken {
        AccessToken::new("scaffold-access")
    }
    fn fresh_recovery_code(&mut self) -> RecoveryCode {
        RecoveryCode::new("scaffold-recovery")
    }
    fn fresh_admin_invitation(&mut self) -> AdminInvitationToken {
        AdminInvitationToken::new("scaffold-invite")
    }
}

/// Server time from the JS runtime clock (`Date.now()`) — **never a device clock** (so a wrong
/// device clock can neither grant nor deny; binding cannot complete offline, plan §10).
pub struct WorkerClock;

impl Clock for WorkerClock {
    fn now(&self) -> UnixSeconds {
        UnixSeconds::new((worker::Date::now().as_millis() / 1000) as i64)
    }
}
