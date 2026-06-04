//! Device-token binding & invalidation tests (privacy invariant I4; spec AC4/AC18; ADR-0016 D2).
//!
//! `i4_tokens_invalidated_on_reonboarding` (AC4) and `i4_tokens_invalidated_on_logout` are the
//! two named I4 enforcement tests; `ac18_invalidation_triggers_exactly` pins that the session
//! ends on **exactly** the three admin-mediated triggers and on nothing time-based.

use boundless_auth::{
    invalidation_for, reonboarding_invalidation, required_refresh_store, DeviceBinding,
    InvalidationTrigger, SecureStoreClass, TokenInvalidationScope, DEVICE_TOKEN_INVALIDATED_CODE,
};
use boundless_domain::{AppVersion, MemberId, Platform};
use proptest::prelude::*;
use uuid::Uuid;

fn member(n: u128) -> MemberId {
    MemberId::from_uuid(Uuid::from_u128(n))
}

fn platform_for_index(i: u8) -> Platform {
    match i % 7 {
        0 => Platform::Ios,
        1 => Platform::IpadOs,
        2 => Platform::WatchOs,
        3 => Platform::MacOs,
        4 => Platform::Android,
        5 => Platform::WearOs,
        _ => Platform::Web,
    }
}

#[test]
fn i4_tokens_invalidated_on_reonboarding() {
    // The member re-onboards a new device (here a newer app version on the same platform). The
    // prior device's token is invalidated (AC4); the new device registers a fresh one.
    let prior = DeviceBinding::new(member(1), Platform::Ios, AppVersion::new(1, 1, 0));
    let new = DeviceBinding::new(member(1), Platform::Ios, AppVersion::new(1, 2, 0));

    let outcome = reonboarding_invalidation(&prior, &new).expect("same member is invalidated");
    assert!(outcome.is_session_ended());
    assert_eq!(outcome.token_scope, TokenInvalidationScope::PriorDevice);

    // Re-onboarding onto a different platform still supersedes the member's prior device.
    let cross_platform = DeviceBinding::new(member(1), Platform::Android, AppVersion::new(1, 2, 0));
    assert!(reonboarding_invalidation(&prior, &cross_platform).is_some());

    // But a different member's device is never touched (cross-member isolation).
    let other = DeviceBinding::new(member(2), Platform::Ios, AppVersion::new(1, 2, 0));
    assert_eq!(reonboarding_invalidation(&prior, &other), None);
}

#[test]
fn i4_tokens_invalidated_on_logout() {
    // Admin revoke / member logout invalidates ALL of the member's device tokens (I4 "any auth
    // change"), distinct from re-onboarding's prior-device-only scope.
    let outcome = invalidation_for(InvalidationTrigger::AdminRevokeOrLogout);
    assert!(outcome.is_session_ended());
    assert_eq!(outcome.token_scope, TokenInvalidationScope::AllForMember);
    assert_eq!(
        DEVICE_TOKEN_INVALIDATED_CODE,
        "AUTH_DEVICE_TOKEN_INVALIDATED"
    );
}

#[test]
fn ac18_invalidation_triggers_exactly() {
    // The three admin-mediated triggers — and only these — end a session (ADR-0016 D2). The
    // wildcard-free match is a compile-time exhaustiveness guard: a fourth trigger would fail
    // to build here and force its scope to be declared.
    let all = [
        InvalidationTrigger::AdminRevokeOrLogout,
        InvalidationTrigger::NewDeviceReonboarding,
        InvalidationTrigger::AccountDeletion,
    ];
    for trigger in all {
        let outcome = invalidation_for(trigger);
        assert!(
            outcome.is_session_ended(),
            "{trigger:?} must end the session"
        );
        let expected_scope = match trigger {
            InvalidationTrigger::AdminRevokeOrLogout => TokenInvalidationScope::AllForMember,
            InvalidationTrigger::AccountDeletion => TokenInvalidationScope::AllForMember,
            InvalidationTrigger::NewDeviceReonboarding => TokenInvalidationScope::PriorDevice,
        };
        assert_eq!(outcome.token_scope, expected_scope, "{trigger:?} scope");
    }
}

#[test]
fn required_refresh_store_maps_every_platform_to_a_secure_store() {
    // §10-F: each platform's refresh credential lives in its hardware-backed / server-side
    // secret store. The mapping is single-sourced in core so the UI tasks read it.
    let cases = [
        (Platform::Ios, SecureStoreClass::AppleKeychain),
        (Platform::IpadOs, SecureStoreClass::AppleKeychain),
        (Platform::WatchOs, SecureStoreClass::AppleKeychain),
        (Platform::MacOs, SecureStoreClass::AppleKeychain),
        (Platform::Android, SecureStoreClass::AndroidKeystore),
        (Platform::WearOs, SecureStoreClass::AndroidKeystore),
        (Platform::Web, SecureStoreClass::ServerSideHttpOnlyCookie),
    ];
    for (platform, expected) in cases {
        assert_eq!(required_refresh_store(platform), expected, "{platform:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, .. ProptestConfig::default() })]

    /// I4 cross-member isolation as a property: re-onboarding invalidates the prior device
    /// **iff** the two bindings are the same member — for *any* platform/app-version legs — and
    /// when it does, it is always the prior-device scope. A different member's device is never
    /// touched by another member's re-onboarding.
    #[test]
    fn prop_reonboarding_isolates_by_member(
        m_prior in any::<u128>(),
        m_new in any::<u128>(),
        p_prior in 0u8..7,
        p_new in 0u8..7,
        v_prior in 0u32..5,
        v_new in 0u32..5,
    ) {
        let prior = DeviceBinding::new(
            MemberId::from_uuid(Uuid::from_u128(m_prior)),
            platform_for_index(p_prior),
            AppVersion::new(1, v_prior, 0),
        );
        let new = DeviceBinding::new(
            MemberId::from_uuid(Uuid::from_u128(m_new)),
            platform_for_index(p_new),
            AppVersion::new(1, v_new, 0),
        );

        let outcome = reonboarding_invalidation(&prior, &new);
        prop_assert_eq!(outcome.is_some(), m_prior == m_new);
        if let Some(inv) = outcome {
            prop_assert!(inv.is_session_ended());
            prop_assert_eq!(inv.token_scope, TokenInvalidationScope::PriorDevice);
        }
    }
}
