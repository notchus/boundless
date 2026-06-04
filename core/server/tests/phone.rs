//! Phone canonicalization (T03/T04 carry-forward): the same human number always yields the same
//! E.164 form (so its lookup hash matches what issuance stored), and non-E.164 input is rejected.

use boundless_server_core::{normalize_phone, PhoneNormalizeError};

#[test]
fn same_human_number_different_formats_canonicalize_identically() {
    let canonical = "+15550101234";
    for raw in [
        "+15550101234",
        "+1 555 010 1234",
        "+1 (555) 010-1234",
        "+1.555.010.1234",
        "  +1-555-010-1234  ",
    ] {
        let n = normalize_phone(raw).expect("valid E.164");
        assert_eq!(
            n.expose_secret(),
            canonical,
            "`{raw}` should canonicalize to {canonical}"
        );
    }
}

#[test]
fn normalize_is_idempotent() {
    let once = normalize_phone("+1 (555) 010-1234").unwrap();
    let twice = normalize_phone(once.expose_secret()).unwrap();
    assert_eq!(once.expose_secret(), twice.expose_secret());
}

#[test]
fn rejects_non_e164() {
    // `PhoneNumber` has no `PartialEq`/`Debug` (P2), so the `Ok` side can't be `assert_eq!`'d —
    // match the error variant directly instead.
    assert!(matches!(
        normalize_phone(""),
        Err(PhoneNormalizeError::Empty)
    ));
    assert!(matches!(
        normalize_phone("   "),
        Err(PhoneNormalizeError::Empty)
    ));
    // No leading `+` — not international form.
    assert!(matches!(
        normalize_phone("15550101234"),
        Err(PhoneNormalizeError::MissingPlus)
    ));
    // A non-digit, non-separator character.
    assert!(matches!(
        normalize_phone("+1-555-01a-1234"),
        Err(PhoneNormalizeError::InvalidCharacter)
    ));
    // Only a `+` and separators — no digits.
    assert!(matches!(
        normalize_phone("+ - ()"),
        Err(PhoneNormalizeError::Empty)
    ));
    // More than 15 digits.
    assert!(matches!(
        normalize_phone("+1234567890123456"),
        Err(PhoneNormalizeError::TooLong)
    ));
}
