//! Time, as an **injected** value — never `SystemTime::now` (forbidden-patterns: "Direct
//! `SystemTime::now()` → Inject a `Clock` trait").
//!
//! `core::auth` is pure: every time-dependent decision (Onboarding Code TTL, session
//! expiry) reads the current time from an injected [`Clock`]. In production the server
//! supplies **server time** (so a wrong *device* clock can neither grant nor deny access,
//! and binding cannot complete offline — spec edge case "User's clock is wrong"). Tests and
//! deterministic callers use [`FixedClock`].
//!
//! Why a homegrown [`UnixSeconds`] instead of `chrono`/`time`: those crates are still
//! unpinned in `docs/stack-matrix.md` and gated on an ADR ("pick one — file ADR if both
//! used"). A one-field integer newtype used only for `<` comparisons is the proportionate,
//! dependency-free choice; the chrono-vs-time decision stays deferred (`DEFERRED.md`).

use serde::{Deserialize, Serialize};

/// A point in time as whole seconds since the Unix epoch, **UTC**.
///
/// Always UTC, so it is inherently timezone-unambiguous (forbidden-patterns: "Date/time
/// without timezone → Always TZ-aware"). Signed so that subtraction and pre-epoch test
/// fixtures are representable without underflow surprises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnixSeconds(pub i64);

impl UnixSeconds {
    /// Construct from whole seconds since the Unix epoch (UTC).
    pub const fn new(secs: i64) -> Self {
        Self(secs)
    }

    /// The underlying seconds-since-epoch value.
    pub const fn as_secs(self) -> i64 {
        self.0
    }

    /// This instant plus `secs` seconds (saturating — a TTL arithmetic overflow clamps to
    /// `i64::MAX` rather than panicking or wrapping; callers use sane TTLs).
    pub const fn saturating_add_secs(self, secs: i64) -> Self {
        Self(self.0.saturating_add(secs))
    }
}

/// An injected source of the current time. Implemented outside the core in production (the
/// server passes server time); `core::auth` itself never reads the wall clock.
pub trait Clock {
    /// The current time, UTC seconds since the Unix epoch.
    fn now(&self) -> UnixSeconds;
}

/// A [`Clock`] frozen at a fixed instant — for deterministic tests (the `TestClock` the
/// task names) and any caller that already holds an authoritative timestamp.
///
/// Pure: it holds a value and never touches `SystemTime`, so it is safe in `core` and on
/// `wasm32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock(pub UnixSeconds);

impl FixedClock {
    /// A clock frozen at `secs` seconds since the Unix epoch (UTC).
    pub const fn at_secs(secs: i64) -> Self {
        Self(UnixSeconds::new(secs))
    }
}

impl Clock for FixedClock {
    fn now(&self) -> UnixSeconds {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_returns_its_instant() {
        let clock = FixedClock::at_secs(1_750_000_000);
        assert_eq!(clock.now(), UnixSeconds::new(1_750_000_000));
    }

    #[test]
    fn ordering_is_chronological() {
        assert!(UnixSeconds::new(10) < UnixSeconds::new(11));
        assert!(UnixSeconds::new(-1) < UnixSeconds::new(0));
    }

    #[test]
    fn saturating_add_does_not_overflow() {
        assert_eq!(
            UnixSeconds::new(i64::MAX).saturating_add_secs(10),
            UnixSeconds::new(i64::MAX)
        );
        assert_eq!(
            UnixSeconds::new(100).saturating_add_secs(50),
            UnixSeconds::new(150)
        );
    }
}
