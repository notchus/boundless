//! Phone-number canonicalization for the auth lookup (T03/T04 carry-forward).
//!
//! The phone-lookup hash (I3) is computed over the **exact bytes** handed to `core::crypto`, so
//! the same human number must canonicalize *identically* every time or its hash won't match what
//! issuance stored. [`normalize_phone`] produces a canonical **E.164** string (`+`, then the
//! country code, then the subscriber number — digits only) from a number already in
//! international form, stripping the usual human separators (spaces, dashes, parentheses, dots).
//!
//! **Single-source contract (P4):** spec-008 admin issuance MUST canonicalize the
//! admin-entered number through *this* function too, so the number Sarah types at issuance and
//! the number the helper types at onboarding produce the same `phone_lookup_hash`.
//!
//! Scope: this canonicalizes a number already in `+CC…` international form — the proportionate
//! choice for the closed-group, admin-issued model (the admin enters the canonical number once).
//! Full libphonenumber-grade national-format parsing is deferred (`DEFERRED.md` → T07-shell).

use boundless_domain::PhoneNumber;

/// The largest number of digits a valid E.164 number can have (excluding the leading `+`).
const E164_MAX_DIGITS: usize = 15;

/// Why a phone string could not be canonicalized to E.164.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneNormalizeError {
    /// Empty input, or no digits remained after stripping separators.
    Empty,
    /// Not in international form — an E.164 number must begin with `+`.
    MissingPlus,
    /// A character other than a digit or an allowed separator was present.
    InvalidCharacter,
    /// More than 15 digits — not a valid E.164 number.
    TooLong,
}

/// Canonicalize `raw` to an E.164 [`PhoneNumber`] (`+` followed by 1–15 digits), or return why
/// it could not be. Separators stripped: ASCII space, NBSP, `-`, `(`, `)`, `.`.
///
/// The result is a tainted [`PhoneNumber`] (PII) — the caller hashes it via `core::crypto`; the
/// plaintext never crosses the wire or a log line (I3/P2).
pub fn normalize_phone(raw: &str) -> Result<PhoneNumber, PhoneNormalizeError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PhoneNormalizeError::Empty);
    }

    let mut chars = trimmed.chars();
    if chars.next() != Some('+') {
        return Err(PhoneNormalizeError::MissingPlus);
    }

    let mut digits = String::with_capacity(trimmed.len());
    for c in chars {
        match c {
            '0'..='9' => digits.push(c),
            // Allowed human separators — discarded.
            ' ' | '\u{00A0}' | '-' | '(' | ')' | '.' => {}
            _ => return Err(PhoneNormalizeError::InvalidCharacter),
        }
    }

    if digits.is_empty() {
        return Err(PhoneNormalizeError::Empty);
    }
    if digits.len() > E164_MAX_DIGITS {
        return Err(PhoneNormalizeError::TooLong);
    }

    let mut canonical = String::with_capacity(digits.len() + 1);
    canonical.push('+');
    canonical.push_str(&digits);
    Ok(PhoneNumber::new(canonical))
}
