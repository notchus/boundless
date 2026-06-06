//! AC3 (scrubber leg) — the onboarding **log-scrubber replay** (P2/I10, spec 001 **T16**).
//!
//! Replays `fixtures/onboarding/log_lines.jsonl` — representative structured log events the
//! onboarding/auth flow emits, covering the error/offline branches the risk register calls out
//! (`PhoneNotOnFile`, `BindingFailed`, rate-limit, below-min, offline, manifest fallback,
//! refresh-replay, admin alerts) — through the I10 PII detector and asserts **every line is
//! clean** (zero findings). This is the testable form of I10's "a log line should never reach
//! the scrubber carrying PII to begin with": the lines are PII-free *by construction* (only
//! opaque UUIDs, version strings, error codes, enums, ISO timestamps).
//!
//! Division of proof: this replay proves the onboarding *flow* is clean; the *detector's*
//! coverage (that it would catch PII if any leaked) is proven by `scrub_redteam.rs`. Both are
//! needed — a clean flow through a blind detector would be a vacuous gate.

use boundless_logging::detect_pii;

const LOG_LINES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/onboarding/log_lines.jsonl"
));

#[test]
fn onboarding_log_fixtures_carry_no_pii() {
    let mut lines = 0usize;
    for (i, line) in LOG_LINES.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        lines += 1;
        let findings = detect_pii(line);
        assert!(
            findings.is_empty(),
            "PII reached the scrubber on fixture line {} (1-based): {:?}\n  line: {}",
            i + 1,
            findings,
            line,
        );
    }
    // Guard against an empty/missing fixture silently passing (the replay must actually run).
    assert!(
        lines >= 20,
        "expected the onboarding log fixture to cover the auth/error/offline branches (≥20 lines), got {lines}"
    );
}
