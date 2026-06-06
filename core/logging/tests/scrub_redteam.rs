//! Red-team self-test for the I10 PII scrubber (P2, spec 001 **T16**, AC3).
//!
//! Two halves, and both matter:
//!
//! 1. **The detector has teeth** — every PII category is caught in its common shapes (E.164 /
//!    dotted / dashed / paren / spaced phones; emails; ≥40-char token blobs; title-case street
//!    addresses; paired, single, and separate-JSON-field GPS) — so the onboarding replay's "zero
//!    findings" in `onboarding_replay.rs` is a real guarantee, not a blind detector. (Known
//!    residual shapes — sub-40-char tokens, lowercase-only addresses — are tracked as deferred
//!    hardening in `DEFERRED.md`; they are latent until the live `emit()` sink ships at T07-shell-B.)
//! 2. **The false-positive guard** — the non-PII values the auth layer legitimately logs
//!    (opaque `MemberId` UUIDs, version strings, error codes, event names, ISO-8601
//!    timestamps, dotted-quad IPs) are *never* flagged. Without this, the scrubber would
//!    over-redact and (per I10's "CI fails on any redaction") break the gate for clean lines.

use boundless_logging::{contains_pii, detect_pii, PiiCategory};

fn categories(line: &str) -> Vec<PiiCategory> {
    detect_pii(line).into_iter().map(|f| f.category).collect()
}

// — 1. the detector catches real PII —

#[test]
fn catches_phone_numbers_in_common_formats() {
    for phone in [
        "+15551234567",     // E.164
        "(555) 123-4567",   // US formatted
        "555-123-4567",     // US dashed
        "555.123.4567",     // US dotted (the most common written format)
        "+1.555.123.4567",  // dotted with country code
        "+44 20 7946 0958", // intl with spaces
    ] {
        assert!(
            categories(phone).contains(&PiiCategory::Phone),
            "phone not detected: {phone:?}"
        );
        // …and embedded in a structured log line.
        let line = format!(r#"{{"event":"signin","phone":"{phone}"}}"#);
        assert!(contains_pii(&line), "phone not detected in line: {line:?}");
    }
}

#[test]
fn dotted_quad_ipv4_is_not_flagged_as_a_phone() {
    // A 10-digit dotted-quad IP must not be misread as a phone (the dotted-phone support's
    // structural exclusion). Whether an IP is itself PII is out of scope; it is simply not a phone.
    for ip in ["192.168.1.100", "10.0.0.1", "255.255.255.255"] {
        assert!(
            !categories(ip).contains(&PiiCategory::Phone),
            "IPv4 wrongly flagged as phone: {ip:?}"
        );
    }
}

#[test]
fn catches_email_addresses() {
    for email in [
        "maria@example.com",
        "sarah.admin+boundless@congregation.org",
    ] {
        assert!(
            categories(email).contains(&PiiCategory::Email),
            "email not detected: {email:?}"
        );
    }
}

#[test]
fn catches_token_and_secret_blobs() {
    // 64 hex chars (a 32-byte access/refresh credential or device token).
    let hex64 = "a".repeat(64);
    assert!(categories(&hex64).contains(&PiiCategory::TokenBlob));
    // base64url (a JWT-ish segment), ≥ 40 chars.
    let b64 = "eyJhbGciOiJFUzI1NiJ9_dGhpcy1pcy1hLXNlY3JldC10b2tlbi12YWx1ZQ";
    assert!(categories(b64).contains(&PiiCategory::TokenBlob));
}

#[test]
fn catches_street_addresses() {
    for addr in ["123 Main Street", "47 Willow Lane"] {
        assert!(
            categories(addr).contains(&PiiCategory::StreetAddress),
            "street not detected: {addr:?}"
        );
    }
}

#[test]
fn catches_gps_coordinates() {
    for coord in [
        "37.7749,-122.4194",                  // bare comma-joined pair
        "51.5074, -0.1278",                   // pair with space
        r#"{"lat":37.7749,"lng":-122.4194}"#, // separate JSON fields (the realistic shape)
        "37.774929",                          // a single high-precision component
    ] {
        assert!(
            categories(coord).contains(&PiiCategory::GpsCoordinate),
            "gps not detected: {coord:?}"
        );
    }
}

// — 2. the false-positive guard: non-PII values are NEVER flagged —

#[test]
fn opaque_member_uuids_are_not_flagged() {
    // Canonical UUIDs, including the worst case: an all-numeric final 12-hex group, whose
    // 12-digit run would otherwise look like a phone number. Suppression must catch it.
    for uuid in [
        "550e8400-e29b-41d4-a716-446655440000",
        "f47ac10b-58cc-4372-a567-0e02b2c3d479",
        "00000000-0000-0000-0000-000000000000",
        "12345678-1234-1234-1234-446655440000",
    ] {
        assert!(
            detect_pii(uuid).is_empty(),
            "UUID wrongly flagged as PII: {uuid:?} → {:?}",
            detect_pii(uuid)
        );
        let line = format!(r#"{{"member":"{uuid}","event":"onboarding.transition"}}"#);
        assert!(
            detect_pii(&line).is_empty(),
            "UUID-in-line wrongly flagged: {line:?} → {:?}",
            detect_pii(&line)
        );
    }
}

#[test]
fn version_strings_are_not_flagged() {
    for v in ["3.1.0", "4.12.7", "10.0.0"] {
        assert!(detect_pii(v).is_empty(), "version wrongly flagged: {v:?}");
    }
}

#[test]
fn error_codes_and_event_names_are_not_flagged() {
    for s in [
        "AUTH_BELOW_MIN_VERSION",
        "AUTH_ONBOARDING_CODE_RATE_LIMITED",
        "ADMIN_WEBAUTHN_VERIFICATION_FAILED",
        "onboarding.transition",
        "device_binding",
        "phone_not_on_file",
    ] {
        assert!(
            detect_pii(s).is_empty(),
            "non-PII token wrongly flagged: {s:?}"
        );
    }
}

#[test]
fn iso8601_timestamps_are_not_flagged() {
    for ts in ["2026-06-06T12:00:00Z", "2026-06-06T12:34:56.789Z"] {
        assert!(
            detect_pii(ts).is_empty(),
            "timestamp wrongly flagged: {ts:?}"
        );
    }
}
