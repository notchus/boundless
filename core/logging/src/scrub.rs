//! PII detection over a raw log line (privacy invariant **I10**, **P2**).
//!
//! [`detect_pii`] scans a line for the PII shapes that must never be persisted in a log. It
//! is **conservative on purpose** — a PII gate that occasionally over-flags is far safer than
//! one that misses — but it is tuned so the two values the auth layer legitimately *does* put
//! in structured logs are never flagged:
//!
//! - **`MemberId` UUIDs** — the opaque, non-PII member identifier (the I12 deletion
//!   stand-in; `core/server/src/alerts.rs` documents that it is *not* PII), and
//! - **version strings** (e.g. `"3.1.0"`, for the O5 stragglers signal).
//!
//! Both facts are pinned by `tests/scrub_redteam.rs` (the false-positive guard), so the
//! onboarding replay's "zero findings" (`tests/onboarding_replay.rs`) is meaningful: the
//! detector has teeth (it catches real PII) *and* the onboarding flow's lines are clean.
//!
//! All PII shapes here are ASCII, so the scanner works over bytes and lets any non-ASCII byte
//! (e.g. UTF-8 accented or bracketed pseudo-locale text) simply terminate a run. Matches are
//! therefore always ASCII byte ranges, which fall on `char` boundaries — slicing is safe.

/// A category of personally identifiable information the scrubber recognizes (P2 lists the
/// tainted shapes: phone, address, exact coordinates, device/auth tokens, email).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiiCategory {
    /// An email address (`local@domain.tld`).
    Email,
    /// A phone number — a run of digits + phone separators (incl. `.`, so dotted US format is
    /// caught) with 10–15 digits (E.164 range); a dotted-quad IPv4 is excluded.
    Phone,
    /// A token/secret blob — a long contiguous hex / base64 / base64url run (≥ 40 chars), the
    /// shape of a device push token, access/refresh credential, or signature. A canonical
    /// UUID (36 chars, even counting its `-`) is shorter than the threshold, so it is excluded.
    TokenBlob,
    /// A street address — a 1–5 digit house number followed by a capitalized word
    /// (`123 Main`).
    StreetAddress,
    /// An exact GPS coordinate — a decimal `lat,long` pair (`37.7749,-122.4194`) or a single
    /// high-precision component (|int| ≤ 180, ≥ 4 fraction digits), incl. the separate-JSON-field
    /// shape `"lat":37.7749,"lng":-122.4194`.
    GpsCoordinate,
}

/// One detected PII occurrence: its [category](PiiCategory), the byte offset where it starts
/// in the scanned line, and the matched substring (for redaction and for loud test failures).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// What kind of PII was matched.
    pub category: PiiCategory,
    /// Byte offset of the match start within the scanned line.
    pub start: usize,
    /// The matched substring.
    pub matched: String,
}

impl Finding {
    fn new(category: PiiCategory, start: usize, matched: &str) -> Self {
        Self {
            category,
            start,
            matched: matched.to_string(),
        }
    }
}

/// Scan `line` for every PII shape and return all findings (possibly overlapping across
/// categories — e.g. an all-digit 64-char blob is both a [`Phone`](PiiCategory::Phone) and a
/// [`TokenBlob`](PiiCategory::TokenBlob)). Empty result ⇔ the line is clean.
///
/// Canonical `MemberId`/`SessionFamilyId` UUIDs are **not** PII (the opaque, post-deletion
/// identifiers — see `core/server/src/alerts.rs`), so findings that fall entirely inside a
/// canonical UUID are suppressed. Without this, a UUID whose final 12-hex group happens to be
/// all decimal digits would be misread as a phone number — over-redacting a non-PII id and
/// (per I10's "fail on any redaction" CI step) failing the gate for no reason.
pub fn detect_pii(line: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    find_emails(line, &mut out);
    find_phones(line, &mut out);
    find_token_blobs(line, &mut out);
    find_street_addresses(line, &mut out);
    find_gps_coordinates(line, &mut out);
    find_high_precision_coords(line, &mut out);

    let uuids = uuid_spans(line);
    out.retain(|f| {
        let end = f.start + f.matched.len();
        !uuids.iter().any(|&(us, ue)| us <= f.start && end <= ue)
    });
    out
}

/// `true` iff [`detect_pii`] finds any PII in `line`.
pub fn contains_pii(line: &str) -> bool {
    !detect_pii(line).is_empty()
}

// — email —

fn is_email_local(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-')
}

fn is_email_domain(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-')
}

fn find_emails(line: &str, out: &mut Vec<Finding>) {
    let bytes = line.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if b != b'@' {
            continue;
        }
        // Expand a local part to the left and a domain to the right of the `@`.
        let mut start = idx;
        while start > 0 && is_email_local(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = idx + 1;
        while end < bytes.len() && is_email_domain(bytes[end]) {
            end += 1;
        }
        if start == idx || end == idx + 1 {
            continue; // no local part, or no domain
        }
        // Require a dot-separated TLD of ≥ 2 alphabetic chars, so `a@b` / `@x` don't match.
        let domain = &line[idx + 1..end];
        if let Some(dot) = domain.rfind('.') {
            let tld = &domain[dot + 1..];
            if tld.len() >= 2 && tld.bytes().all(|c| c.is_ascii_alphabetic()) {
                out.push(Finding::new(PiiCategory::Email, start, &line[start..end]));
            }
        }
    }
}

// — phone —

/// A digit or a common phone separator — incl. `.` so the ubiquitous dotted US format
/// (`555.123.4567`) is caught. The two non-phone things a `.`-bearing digit run could be are
/// excluded structurally in [`find_phones`]: a dotted-quad IP (via [`looks_like_ipv4`]) and a
/// version string (always < 10 digits, so below the phone threshold anyway, e.g. `3.1.0`).
fn is_phone_byte(b: u8) -> bool {
    b.is_ascii_digit() || matches!(b, b' ' | b'+' | b'(' | b')' | b'-' | b'.')
}

/// `true` if `run` is a dotted-quad IPv4 (`a.b.c.d`, each octet 0–255) — not a phone.
fn looks_like_ipv4(run: &str) -> bool {
    let parts: Vec<&str> = run.trim().split('.').collect();
    parts.len() == 4
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && p.parse::<u32>().is_ok_and(|n| n <= 255)
        })
}

fn find_phones(line: &str, out: &mut Vec<Finding>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !is_phone_byte(bytes[i]) {
            i += 1;
            continue;
        }
        let start = i;
        let mut digits = 0usize;
        while i < bytes.len() && is_phone_byte(bytes[i]) {
            if bytes[i].is_ascii_digit() {
                digits += 1;
            }
            i += 1;
        }
        // E.164 is 1–15 digits; a real, loggable phone is at least ~10. Below 10 we avoid
        // flagging dates (`2026-06-06`, 8 digits) and short numeric fields. A 10–15 digit
        // dotted run that is actually an IPv4 address is not a phone.
        let run = &line[start..i];
        if (10..=15).contains(&digits) && !looks_like_ipv4(run) {
            out.push(Finding::new(PiiCategory::Phone, start, run));
        }
    }
}

// — token / secret blob —

/// A char that can appear in a hex / base64 / base64url token. Note `-` and `_` are included
/// (base64url), which means a dashed UUID is one contiguous run — but only 36 chars, below
/// the 40-char threshold, so it is excluded.
fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'_' | b'-')
}

const TOKEN_BLOB_MIN: usize = 40;

fn find_token_blobs(line: &str, out: &mut Vec<Finding>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !is_token_byte(bytes[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && is_token_byte(bytes[i]) {
            i += 1;
        }
        if i - start >= TOKEN_BLOB_MIN {
            out.push(Finding::new(PiiCategory::TokenBlob, start, &line[start..i]));
        }
    }
}

// — street address —

fn find_street_addresses(line: &str, out: &mut Vec<Finding>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let mut digits = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
        // A 1–5 digit house number, one space, then a capitalized word of ≥ 3 letters.
        if (1..=5).contains(&digits) && i < bytes.len() && bytes[i] == b' ' {
            let word_start = i + 1;
            if word_start < bytes.len() && bytes[word_start].is_ascii_uppercase() {
                let mut j = word_start + 1;
                while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j - word_start >= 3 {
                    out.push(Finding::new(
                        PiiCategory::StreetAddress,
                        start,
                        &line[start..j],
                    ));
                }
            }
        }
    }
}

// — GPS coordinate —

/// Parse an optionally-signed decimal with a 1–3 digit integer part and a ≥ 3 digit fraction
/// (the shape of a lat/long component). Returns the end offset on success. A version like
/// `3.1.0` fails (its fraction `1` is < 3 digits).
fn parse_decimal(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    if j < bytes.len() && bytes[j] == b'-' {
        j += 1;
    }
    let int_start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    let int_digits = j - int_start;
    if !(1..=3).contains(&int_digits) {
        return None;
    }
    if j >= bytes.len() || bytes[j] != b'.' {
        return None;
    }
    j += 1;
    let frac_start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j - frac_start < 3 {
        return None;
    }
    Some(j)
}

fn find_gps_coordinates(line: &str, out: &mut Vec<Finding>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let Some(e1) = parse_decimal(bytes, i) else {
            i += 1;
            continue;
        };
        // A `lat , long` pair, comma-separated (optional surrounding spaces).
        let mut k = e1;
        while k < bytes.len() && bytes[k] == b' ' {
            k += 1;
        }
        if k < bytes.len() && bytes[k] == b',' {
            k += 1;
            while k < bytes.len() && bytes[k] == b' ' {
                k += 1;
            }
            if let Some(e2) = parse_decimal(bytes, k) {
                out.push(Finding::new(PiiCategory::GpsCoordinate, i, &line[i..e2]));
                i = e2;
                continue;
            }
        }
        i = e1;
    }
}

/// Detect a *single* high-precision decimal coordinate component (`37.7749`, `-122.4194`). This
/// catches the realistic structured-log shape where lat/lng are **separate JSON fields**
/// (`"lat":37.7749,"lng":-122.4194`) — which the comma-adjacent pair scan in
/// [`find_gps_coordinates`] misses — and a lone high-precision value (most sensitive: I9). Tuned
/// against false positives: requires |int part| ≤ 180 (the lat/long range) AND ≥ 4 fraction
/// digits, anchored at a token boundary so it can't fire on a substring of a longer number
/// (`1234.5678`). Version strings (fraction = 1 digit) and millisecond timestamps (`…:56.789`,
/// 3 fraction digits) are below the fraction threshold and so are not flagged.
fn find_high_precision_coords(line: &str, out: &mut Vec<Finding>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Only start at a token boundary (the char before is not a digit or `.`), so a coord
        // substring of a longer number is never matched.
        let at_boundary = i == 0 || !matches!(bytes[i - 1], b'0'..=b'9' | b'.');
        if at_boundary {
            if let Some(end) = parse_high_precision_coord(bytes, i) {
                out.push(Finding::new(PiiCategory::GpsCoordinate, i, &line[i..end]));
                i = end;
                continue;
            }
        }
        i += 1;
    }
}

fn parse_high_precision_coord(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i;
    if j < bytes.len() && bytes[j] == b'-' {
        j += 1;
    }
    let int_start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    let int_str = core::str::from_utf8(&bytes[int_start..j]).ok()?;
    if !(1..=3).contains(&int_str.len()) || int_str.parse::<u32>().ok()? > 180 {
        return None;
    }
    if j >= bytes.len() || bytes[j] != b'.' {
        return None;
    }
    j += 1;
    let frac_start = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j - frac_start < 4 {
        return None;
    }
    // Reject a continued dotted number (`1.2345.6`); a trailing sentence period is fine.
    if j + 1 < bytes.len() && bytes[j] == b'.' && bytes[j + 1].is_ascii_digit() {
        return None;
    }
    Some(j)
}

// — canonical UUID (non-PII identifier) suppression —

/// Byte ranges of canonical `8-4-4-4-12` hex UUIDs in `line`. These are the opaque, non-PII
/// `MemberId`/`SessionFamilyId` identifiers (the I12 deletion stand-in); findings fully inside
/// one are suppressed in [`detect_pii`]. A 64-hex token blob has no dashes, so it never
/// matches this shape — `TokenBlob` detection is unaffected.
fn uuid_spans(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i + 36 <= bytes.len() {
        if is_uuid_at(bytes, i) {
            spans.push((i, i + 36));
            i += 36;
        } else {
            i += 1;
        }
    }
    spans
}

fn is_uuid_at(bytes: &[u8], i: usize) -> bool {
    // 8-4-4-4-12 hex with dashes at offsets 8, 13, 18, 23.
    const HEX_GROUPS: [(usize, usize); 5] = [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)];
    const DASHES: [usize; 4] = [8, 13, 18, 23];
    // Must be a standalone token, not a slice of a longer hex/identifier run.
    if i > 0 && is_token_byte(bytes[i - 1]) {
        return false;
    }
    if i + 36 < bytes.len() && is_token_byte(bytes[i + 36]) {
        return false;
    }
    for (s, e) in HEX_GROUPS {
        for k in s..e {
            if !bytes[i + k].is_ascii_hexdigit() {
                return false;
            }
        }
    }
    DASHES.iter().all(|&d| bytes[i + d] == b'-')
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn empty_and_clean_lines_have_no_findings() {
        assert!(detect_pii("").is_empty());
        assert!(detect_pii(r#"{"event":"onboarding.transition","to":"DeviceBinding"}"#).is_empty());
    }

    #[test]
    fn categories_are_distinguished() {
        assert_eq!(detect_pii("a@b.co")[0].category, PiiCategory::Email);
        assert_eq!(detect_pii("+15551234567")[0].category, PiiCategory::Phone);
    }
}
