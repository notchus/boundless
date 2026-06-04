//! Golden serialization snapshots for the wire value types, plus snapshots of the
//! tainted types' `redacted_summary()` forms (spec 001 T02 test deliverable).
//!
//! The JSON snapshots in `tests/snapshots/` ARE the canonical wire format for these
//! types — review them when they change. The redacted-summary snapshots prove the P2
//! surface renders a fixed, leak-free string.

use boundless_domain::{
    AccessToken, AppVersion, ClientVersion, DeviceToken, MemberId, OnboardingCode, PhoneNumber,
    Platform, RecoveryCode, RefreshToken, Role,
};
use uuid::Uuid;

/// A fixed, well-known UUID so the `MemberId` snapshot is deterministic.
fn fixed_member_id() -> MemberId {
    MemberId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap())
}

#[test]
fn member_id_serializes_transparently_as_uuid_string() {
    insta::assert_json_snapshot!(fixed_member_id());
}

#[test]
fn role_serializes_snake_case_all_variants() {
    insta::assert_json_snapshot!([Role::Rider, Role::Driver, Role::Admin]);
}

#[test]
fn platform_serializes_canonical_all_variants() {
    insta::assert_json_snapshot!([
        Platform::Ios,
        Platform::IpadOs,
        Platform::WatchOs,
        Platform::MacOs,
        Platform::Android,
        Platform::WearOs,
        Platform::Web,
    ]);
}

#[test]
fn app_version_serializes_as_string() {
    insta::assert_json_snapshot!(AppVersion::new(1, 2, 0));
}

#[test]
fn client_version_serializes_as_platform_and_version() {
    insta::assert_json_snapshot!(ClientVersion {
        platform: Platform::Ios,
        app_version: AppVersion::new(1, 2, 0),
    });
}

// --- Tainted types: redacted_summary() forms (inline snapshots; fixed, leak-free) ---

#[test]
fn tainted_redacted_summaries_are_fixed_and_leak_free() {
    insta::assert_snapshot!(PhoneNumber::new("+1-555-867-5309").redacted_summary(), @"PhoneNumber(redacted)");
    insta::assert_snapshot!(DeviceToken::new("apns-token-abc123").redacted_summary(), @"DeviceToken(redacted)");
    insta::assert_snapshot!(OnboardingCode::new("ONB-7F3K").redacted_summary(), @"OnboardingCode(redacted)");
    insta::assert_snapshot!(RecoveryCode::new("REC-9QW2").redacted_summary(), @"RecoveryCode(redacted)");
    insta::assert_snapshot!(AccessToken::new("eyJhbGc.access").redacted_summary(), @"AccessToken(redacted)");
    insta::assert_snapshot!(RefreshToken::new("opaque-256-bit-refresh").redacted_summary(), @"RefreshToken(redacted)");
}

/// Inbound coverage, symmetric with the serialize snapshots above: a regression in
/// `MemberId`'s `#[serde(transparent)]` or in `ClientVersion`'s field mapping would slip
/// past a serialize-only snapshot, so we round-trip every value type through JSON.
#[test]
fn value_types_round_trip_through_json() {
    fn round_trip<T>(value: T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(&value).unwrap();
        let back: T = serde_json::from_str(&json).unwrap();
        assert_eq!(back, value, "round-trip mismatch via {json}");
    }

    round_trip(fixed_member_id());
    round_trip(Role::Driver);
    round_trip(Platform::IpadOs);
    round_trip(AppVersion::new(1, 2, 0));
    round_trip(ClientVersion {
        platform: Platform::Ios,
        app_version: AppVersion::new(1, 2, 0),
    });
}
