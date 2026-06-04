//! The `GroupHub` Durable Object's **pure decision state** (plan §10-E): one per Group, holding
//! the ephemeral rate-limit and alert-dedup counters that back AC8/AC15/AC17.
//!
//! The deployable Worker's DO (T07-shell) owns the *persistence and concurrency* of this state;
//! this module owns the *logic* — the fixed attempt window, the per-member-per-day alert dedup,
//! and the rejected-refresh throttle counter — so they are single-sourced and testable (P4).
//! Everything is keyed by [`MemberId`] (globally unique): per glossary/personas a Boundless
//! install is a single Group, so no group id is needed here (the multi-group tenant scoping in
//! the T06 schema is enforced at the shell via RLS, `DEFERRED.md`).

use std::collections::{HashMap, HashSet};

use boundless_auth::UnixSeconds;
use boundless_domain::MemberId;

use crate::alerts::AlertKind;
use crate::ports::SourceKey;

/// The Onboarding-Code attempt window: at most 5 attempts per **15 minutes** per member, then
/// the code locks and the admin is alerted (plan §10-D / AC17). A fixed (tumbling) window — a
/// new 15-minute bucket resets the count.
pub const CODE_ATTEMPT_WINDOW_SECS: i64 = 15 * 60;

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

/// The tumbling-window bucket index for `t` at the given `width` (seconds). `div_euclid` so a
/// pre-epoch instant still buckets monotonically (no integer-math surprise around zero).
fn bucket(t: UnixSeconds, width_secs: i64) -> i64 {
    t.as_secs().div_euclid(width_secs)
}

/// The UTC day index for `t` — the dedup window for "one admin alert per member per day".
fn day(t: UnixSeconds) -> i64 {
    bucket(t, SECONDS_PER_DAY)
}

/// The per-Group ephemeral counters held in the `GroupHub` DO.
///
/// Default-constructed empty. The Worker persists/restores it; here it is an in-memory model.
/// Pruning stale buckets/days is the Worker's concern (a DO can compact on alarm) — for the
/// pure logic, old entries are simply never matched again.
#[derive(Default)]
pub struct GroupHubState {
    /// member → (current window bucket, attempts already made in it).
    code_attempts: HashMap<MemberId, (i64, u32)>,
    /// source → (current window bucket, rejected refreshes in it).
    refresh_rejections: HashMap<SourceKey, (i64, u32)>,
    /// the set of `(member, kind, day)` for which an alert has already been emitted.
    alerts_sent: HashSet<(MemberId, AlertKind, i64)>,
}

impl GroupHubState {
    /// A fresh, empty hub state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an Onboarding-Code bind attempt and return the count of **prior** attempts in the
    /// current window. Returning the prior count makes the caller's `max_attempts` gate fire
    /// after exactly `max` tries: prior `0..max-1` are allowed, prior `>= max` locks (matching
    /// `boundless_auth::evaluate_onboarding_code`'s `recent_attempts >= max_attempts`). A new
    /// 15-minute bucket resets the count to zero.
    pub fn register_code_attempt(&mut self, member: MemberId, now: UnixSeconds) -> u32 {
        let b = bucket(now, CODE_ATTEMPT_WINDOW_SECS);
        let entry = self.code_attempts.entry(member).or_insert((b, 0));
        if entry.0 != b {
            *entry = (b, 0);
        }
        let prior = entry.1;
        entry.1 = prior.saturating_add(1);
        prior
    }

    /// Whether to emit an alert of `kind` for `member` now — **true at most once per
    /// `(member, kind, day)`** (O4/AC8/AC15: "one admin alert per member per day"). Marks it as
    /// sent when it returns `true`, so callers should call this only when about to emit.
    pub fn should_alert(&mut self, member: MemberId, kind: AlertKind, now: UnixSeconds) -> bool {
        // `HashSet::insert` returns true iff the key was newly added (i.e. not yet alerted today).
        self.alerts_sent.insert((member, kind, day(now)))
    }

    /// Record a rejected/unknown refresh from `source` and return the count **including** this
    /// one in the current window. The Worker (T07-shell) throttles a source above a threshold
    /// (429), mirroring the Onboarding-Code rate limit (carry-forward); the network-layer
    /// enforcement is the shell's (Cloudflare WAF + this counter), the bookkeeping is here.
    pub fn note_refresh_rejection(&mut self, source: SourceKey, now: UnixSeconds) -> u32 {
        let b = bucket(now, CODE_ATTEMPT_WINDOW_SECS);
        let entry = self.refresh_rejections.entry(source).or_insert((b, 0));
        if entry.0 != b {
            *entry = (b, 0);
        }
        entry.1 = entry.1.saturating_add(1);
        entry.1
    }
}
