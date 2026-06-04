//! Property tests (P9, seeds committed under `proptest-regressions/` on failure): the atomic
//! single-bind invariant and the alert-dedup invariant.

mod common;

use boundless_auth::UnixSeconds;
use boundless_domain::{ClientVersion, DeviceToken, OnboardingCode, Role};
use boundless_server_core::{normalize_phone, AlertKind, BindOutcome, BindRequest, GroupHubState};
use common::*;
use proptest::prelude::*;
use std::collections::HashSet;

const FAR_FUTURE: i64 = 1_000_000;

fn bind_req(code: &str, reported: ClientVersion) -> BindRequest {
    BindRequest {
        phone: normalize_phone("+15550000001").unwrap(),
        code: OnboardingCode::new(code),
        reported,
        device_token: DeviceToken::new("dev-token"),
    }
}

proptest! {
    /// No matter how many times one live Onboarding Code is presented, it binds **at most once**
    /// (the atomic consume — carry-forward (a)). The first presentation binds; every later one is
    /// `Consumed`. Exactly one session/device results.
    #[test]
    fn prop_bind_never_double_consumes(attempts in 1usize..12) {
        let mut store = MemStore::new();
        store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
        store.add_onboarding(member_id(1), "ONB-GOOD", FAR_FUTURE, 100);
        let mut svc = service(store, 1_000);

        let mut bound = 0usize;
        for _ in 0..attempts {
            if matches!(
                svc.bind_device(bind_req("ONB-GOOD", ios_current())).outcome,
                BindOutcome::Bound(_)
            ) {
                bound += 1;
            }
        }
        prop_assert_eq!(bound, 1);
        prop_assert_eq!(svc.store.active_device_count(member_id(1)), 1);
    }

    /// Across any sequence of alert requests within a single UTC day, each `(member, kind)` pair
    /// fires **at most once** (O4/AC8/AC15 dedup). An independent `HashSet` oracle confirms it.
    #[test]
    fn prop_alert_at_most_once_per_member_per_day(
        seq in proptest::collection::vec((0u128..4, 0usize..4), 0..64)
    ) {
        let mut hub = GroupHubState::new();
        let kinds = [
            AlertKind::BelowMinVersion,
            AlertKind::SessionInvalidated,
            AlertKind::OnboardingCodeLocked,
            AlertKind::NotificationsNotEnabled,
        ];
        // Same day throughout; vary only the within-day second so the day bucket is constant.
        let day_base = 50 * 24 * 60 * 60;
        let mut fired: HashSet<(u128, usize)> = HashSet::new();

        for (i, (m, k)) in seq.iter().enumerate() {
            let now = UnixSeconds::new(day_base + (i as i64 % 1000));
            let emitted = hub.should_alert(member_id(*m), kinds[*k], now);
            let first_time = fired.insert((*m, *k));
            // `should_alert` returns true exactly the first time a (member, kind) is seen that day.
            prop_assert_eq!(emitted, first_time);
        }
    }
}
