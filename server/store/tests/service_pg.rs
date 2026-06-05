//! Orchestration-level integration tests (spec 001 T07-shell-B, the async-port bridge): the
//! `core::server` [`AuthService`] driven end-to-end against **real Postgres** via [`PgAuthStore`].
//!
//! This is the genuinely-new proof of the bridge — slice A tested the store methods in isolation,
//! T07-core tested the orchestration over the in-memory stub; here the two run *together*. The
//! security-critical legs that touch Postgres — onboarding single-consume, session create / rotate /
//! revoke, and recovery consume-and-rotate — are exercised through the full endpoint logic.
//!
//! **Device binding lands on an in-memory half.** [`PgAuthStore`] implements [`AuthStore`] but not
//! `DeviceStore` (its `register_device` needs reversibly-encrypted token storage, deferred to spec
//! 008). So the test composes a [`PgService`] wrapper = real `PgAuthStore` (AuthStore) + an
//! in-memory device map (DeviceStore). Production (T07-shell-B Worker) will supply a real
//! `PgDeviceStore` once token encryption lands. Self-skips without `DATABASE_URL` (see `common`).

mod common;

use boundless_auth::{
    DeviceBinding, FixedClock, OnboardingCodeVerdict, RecoveryCodeVerdict, RefreshVerdict, Session,
    SignInResult, UnixSeconds, VersionRequirement,
};
use boundless_crypto::{CodeHash, HmacKey, PhoneLookupHash, RefreshTokenHash};
use boundless_domain::{
    AccessToken, AppVersion, ClientVersion, DeviceToken, MemberId, OnboardingCode, Platform,
    RecoveryCode, RefreshToken, SessionFamilyId,
};
use boundless_server_core::{
    normalize_phone, AdminAlert, AdminAlertSink, AlertKind, AuthConfig, AuthService, AuthStore,
    BindOutcome, BindRequest, DeviceStore, ManifestPointer, MemberRecord, OnboardingCodeRow,
    RecoveryCodeRow, RecoveryOutcome, RecoveryRequest, RefreshClassification, RefreshOutcome,
    RefreshRequest, SecretSource, SignInRequest, SourceKey, StoreBackend,
};
use boundless_server_store::{PgAuthStore, StoreError};
use uuid::Uuid;

use common::*;

// ===== test doubles for the non-store ports + the AuthStore/DeviceStore composition =============

/// Records emitted admin alerts (the `AdminAlertSink` port).
#[derive(Default)]
struct RecordingSink {
    alerts: Vec<AdminAlert>,
}
impl RecordingSink {
    fn count_kind(&self, kind: AlertKind) -> usize {
        self.alerts.iter().filter(|a| a.kind() == kind).count()
    }
}
impl AdminAlertSink for RecordingSink {
    fn emit(&mut self, alert: AdminAlert) {
        self.alerts.push(alert);
    }
}

/// Deterministic, distinct-per-call secrets (so a rotated credential differs from the prior).
#[derive(Default)]
struct SeqSecrets {
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
}

struct DeviceEntry {
    binding: DeviceBinding,
    invalidated: bool,
}

/// The composed store the service drives: a **real** `PgAuthStore` for the session/code/member port,
/// plus an in-memory `DeviceStore` (the Postgres device impl is deferred — see the module docs). The
/// shared `StoreError` lets the orchestration's `?` unify across both ports.
struct PgService {
    pg: PgAuthStore,
    devices: Vec<DeviceEntry>,
}

impl StoreBackend for PgService {
    type Error = StoreError;
}

impl AuthStore for PgService {
    async fn find_member_by_phone(
        &mut self,
        hash: &PhoneLookupHash,
    ) -> Result<Option<MemberRecord>, StoreError> {
        self.pg.find_member_by_phone(hash).await
    }
    async fn load_live_onboarding(
        &mut self,
        member: MemberId,
    ) -> Result<Option<OnboardingCodeRow>, StoreError> {
        self.pg.load_live_onboarding(member).await
    }
    async fn consume_onboarding_if_live(
        &mut self,
        member: MemberId,
        now: UnixSeconds,
    ) -> Result<bool, StoreError> {
        self.pg.consume_onboarding_if_live(member, now).await
    }
    async fn classify_refresh(
        &mut self,
        presented: &RefreshToken,
        key: &HmacKey,
    ) -> Result<RefreshClassification, StoreError> {
        self.pg.classify_refresh(presented, key).await
    }
    async fn rotate_session(
        &mut self,
        family: SessionFamilyId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<Session, StoreError> {
        self.pg
            .rotate_session(family, new_refresh_hash, access_expires_at, now)
            .await
    }
    async fn revoke_family(
        &mut self,
        family: SessionFamilyId,
        now: UnixSeconds,
    ) -> Result<(), StoreError> {
        self.pg.revoke_family(family, now).await
    }
    async fn create_session_family(
        &mut self,
        member: MemberId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<Session, StoreError> {
        self.pg
            .create_session_family(member, new_refresh_hash, access_expires_at, now)
            .await
    }
    async fn load_live_recovery(
        &mut self,
        member: MemberId,
    ) -> Result<Option<RecoveryCodeRow>, StoreError> {
        self.pg.load_live_recovery(member).await
    }
    async fn consume_and_rotate_recovery(
        &mut self,
        member: MemberId,
        fresh_hash: CodeHash,
        now: UnixSeconds,
    ) -> Result<bool, StoreError> {
        self.pg
            .consume_and_rotate_recovery(member, fresh_hash, now)
            .await
    }
}

// Canonical reference for this in-memory device half: `MemStore`'s `DeviceStore` impl in
// `core/server/tests/common/mod.rs` (same upsert-on-(member,platform,app_version) + invalidate-all
// semantics). Keep the two in step until the real `PgDeviceStore` lands (spec 008).
impl DeviceStore for PgService {
    async fn current_device_bindings(
        &mut self,
        member: MemberId,
    ) -> Result<Vec<DeviceBinding>, StoreError> {
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
    ) -> Result<(), StoreError> {
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
        _token: &DeviceToken,
        _now: UnixSeconds,
    ) -> Result<(), StoreError> {
        self.devices.retain(|d| &d.binding != binding);
        self.devices.push(DeviceEntry {
            binding: *binding,
            invalidated: false,
        });
        Ok(())
    }
}

type Svc = AuthService<PgService, RecordingSink, SeqSecrets, FixedClock>;

fn requirement() -> VersionRequirement {
    VersionRequirement::new(AppVersion::new(1, 0, 0), AppVersion::new(1, 2, 0))
}
fn ios_current() -> ClientVersion {
    ClientVersion {
        platform: Platform::Ios,
        app_version: AppVersion::new(1, 2, 0),
    }
}
fn ios_below_min() -> ClientVersion {
    ClientVersion {
        platform: Platform::Ios,
        app_version: AppVersion::new(0, 9, 0),
    }
}

/// Build the engine over a real `PgAuthStore` (as the non-superuser app role) + the in-memory
/// device half, with the SAME HMAC key the harness seeds with (so lookup hashes match).
async fn service(url: &str, schema: &str, group: Uuid, clock_secs: i64) -> Svc {
    let pg = app_store(url, schema, group).await;
    AuthService::new(
        PgService {
            pg,
            devices: Vec::new(),
        },
        RecordingSink::default(),
        SeqSecrets::default(),
        FixedClock::at_secs(clock_secs),
        AuthConfig::new(
            key(),
            requirement(),
            ManifestPointer::new("manifest:v1:index"),
        ),
    )
}

fn bind_req(phone: &str, code: &str, reported: ClientVersion) -> BindRequest {
    BindRequest {
        phone: normalize_phone(phone).expect("valid E.164"),
        code: OnboardingCode::new(code),
        reported,
        device_token: DeviceToken::new("dev-token"),
    }
}
fn refresh_req(token: RefreshToken, reported: ClientVersion) -> RefreshRequest {
    RefreshRequest {
        presented: token,
        reported,
        source: SourceKey(7),
    }
}
fn recovery_req(phone: &str, code: &str) -> RecoveryRequest {
    RecoveryRequest {
        phone: normalize_phone(phone).expect("valid E.164"),
        code: RecoveryCode::new(code),
        reported: ios_current(),
        device_token: DeviceToken::new("new-device-token"),
    }
}
fn signin_req(phone: &str, reported: ClientVersion) -> SignInRequest {
    SignInRequest {
        phone: normalize_phone(phone).expect("valid E.164"),
        reported,
    }
}

// ===== tests ====================================================================================

#[tokio::test]
async fn sign_in_matches_and_misses_through_service() {
    let url = url_or_skip!();
    let su = setup(&url, "svc_signin").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    seed_member(
        &su,
        g,
        mid(1).as_uuid(),
        &["rider"],
        Some(phone_hash("+15550000001")),
    )
    .await;

    let mut svc = service(&url, "svc_signin", g, 1_000).await;
    let matched = svc
        .sign_in(signin_req("+15550000001", ios_current()))
        .await
        .unwrap();
    let missed = svc
        .sign_in(signin_req("+15559999999", ios_current()))
        .await
        .unwrap();

    assert_eq!(matched.result, SignInResult::MemberMatched);
    assert_eq!(missed.result, SignInResult::PhoneNotOnFile);
    // Uniform response shape — only the result discriminant differs (no existence leak).
    assert_eq!(matched.version, missed.version);
    assert_eq!(matched.manifest_pointer, missed.manifest_pointer);
}

#[tokio::test]
async fn bind_device_atomic_single_consume_through_service() {
    let url = url_or_skip!();
    let su = setup(&url, "svc_bind").await;
    let g = Uuid::from_u128(G);
    let m = mid(1);
    seed_group(&su, g).await;
    seed_member(
        &su,
        g,
        m.as_uuid(),
        &["rider"],
        Some(phone_hash("+15550000001")),
    )
    .await;
    seed_onboarding(&su, g, m.as_uuid(), onb_hash("ONB-GOOD"), 100_000, 5).await;

    let mut svc = service(&url, "svc_bind", g, 1_000).await;
    let first = svc
        .bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()))
        .await
        .unwrap();
    let second = svc
        .bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()))
        .await
        .unwrap();

    // First binds; the second presentation of the now-consumed code is `Consumed`, never a second
    // session (the atomic consume in real Postgres — carry-forward (a), AC17).
    assert!(matches!(first.outcome, BindOutcome::Bound(_)));
    assert!(matches!(
        second.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::Consumed)
    ));

    // The onboarding code is genuinely consumed in Postgres (load_live → None).
    let mut store = app_store(&url, "svc_bind", g).await;
    assert!(store.load_live_onboarding(m).await.unwrap().is_none());
}

#[tokio::test]
async fn refresh_rotates_then_replay_kills_family_through_service() {
    let url = url_or_skip!();
    let su = setup(&url, "svc_refresh").await;
    let g = Uuid::from_u128(G);
    let m = mid(1);
    seed_group(&su, g).await;
    seed_member(
        &su,
        g,
        m.as_uuid(),
        &["rider"],
        Some(phone_hash("+15550000001")),
    )
    .await;
    seed_onboarding(&su, g, m.as_uuid(), onb_hash("ONB-GOOD"), 100_000, 5).await;

    let mut svc = service(&url, "svc_refresh", g, 1_000).await;

    // Bind to create a real session family in Postgres; capture its first refresh credential R0.
    let bound = svc
        .bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()))
        .await
        .unwrap();
    let (r0, fam) = match bound.outcome {
        BindOutcome::Bound(material) => (material.refresh.clone(), material.session.family_id),
        _ => panic!("bind should succeed (got a non-Bound outcome)"),
    };

    // Present the current credential → silent rotation to R1 (AC18).
    let rotated = svc
        .refresh(refresh_req(r0.clone(), ios_current()))
        .await
        .unwrap();
    let r1 = match rotated.outcome {
        RefreshOutcome::Rotated(material) => material.refresh.clone(),
        _ => panic!("refresh of the current credential should rotate"),
    };

    // Replay the rotated-away R0 → uniform `Invalidated`, the family killed in Postgres, AC15 alert.
    let replay = svc.refresh(refresh_req(r0, ios_current())).await.unwrap();
    assert!(matches!(replay.outcome, RefreshOutcome::Invalidated));
    assert_eq!(
        replay.server_verdict,
        Some(RefreshVerdict::ReplayDetectedKillFamily)
    );
    assert_eq!(svc.alerts.count_kind(AlertKind::SessionInvalidated), 1);
    assert!(
        revoked_rows(&su, fam.as_uuid()).await >= 1,
        "the family must be persisted revoked in Postgres"
    );

    // The legitimate current credential R1 is now rejected too — the family is dead (AC18).
    let legit = svc.refresh(refresh_req(r1, ios_current())).await.unwrap();
    assert!(matches!(legit.outcome, RefreshOutcome::Invalidated));
}

#[tokio::test]
async fn recovery_rebind_consumes_and_rotates_through_service() {
    let url = url_or_skip!();
    let su = setup(&url, "svc_recovery").await;
    let g = Uuid::from_u128(G);
    let m = mid(1);
    seed_group(&su, g).await;
    seed_member(
        &su,
        g,
        m.as_uuid(),
        &["driver"],
        Some(phone_hash("+15550000001")),
    )
    .await;
    seed_recovery(&su, g, m.as_uuid(), rec_hash_bytes("REC-GOOD")).await;

    let mut svc = service(&url, "svc_recovery", g, 1_000).await;
    let rebound = svc
        .recovery_rebind(recovery_req("+15550000001", "REC-GOOD"))
        .await
        .unwrap();
    assert!(matches!(rebound.outcome, RecoveryOutcome::Rebound { .. }));
    // Consumed + rotated in real Postgres → still exactly one live code (ADR-0016 D3).
    assert_eq!(live_recovery(&su, m.as_uuid()).await, 1);

    // Replaying the old code no longer matches (rotated on use).
    let replay = svc
        .recovery_rebind(recovery_req("+15550000001", "REC-GOOD"))
        .await
        .unwrap();
    assert!(matches!(
        replay.outcome,
        RecoveryOutcome::Rejected(RecoveryCodeVerdict::Invalid)
    ));
}

#[tokio::test]
async fn below_min_degrades_without_touching_session_through_service() {
    let url = url_or_skip!();
    let su = setup(&url, "svc_belowmin").await;
    let g = Uuid::from_u128(G);
    let m = mid(1);
    seed_group(&su, g).await;
    seed_member(
        &su,
        g,
        m.as_uuid(),
        &["rider"],
        Some(phone_hash("+15550000001")),
    )
    .await;
    seed_onboarding(&su, g, m.as_uuid(), onb_hash("ONB-GOOD"), 100_000, 5).await;

    let mut svc = service(&url, "svc_belowmin", g, 1_000).await;
    let resp = svc
        .bind_device(bind_req("+15550000001", "ONB-GOOD", ios_below_min()))
        .await
        .unwrap();
    assert!(matches!(resp.outcome, BindOutcome::BelowMinVersion));

    // The app is merely too old — the onboarding code is NOT consumed in Postgres (O4/O8).
    let mut store = app_store(&url, "svc_belowmin", g).await;
    assert!(
        store.load_live_onboarding(m).await.unwrap().is_some(),
        "below-min must not consume the code"
    );
}
