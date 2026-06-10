//! Property tests (P9, seeds committed under `proptest-regressions/` on failure): the atomic
//! single-bind invariant and the alert-dedup invariant.

mod common;

use boundless_auth::UnixSeconds;
use boundless_crypto::phone_lookup_matches;
use boundless_domain::{
    Address, ClientVersion, DeviceToken, MemberName, OnboardingCode, PhoneNumber, Role,
};
use boundless_server_core::{
    normalize_phone, AlertKind, BindOutcome, BindRequest, EditMemberInput, EditMemberOutcome,
    GroupHubState, IssuableRole, IssueMemberInput, IssueMemberOutcome,
};
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
                svc.bind_device_ok(bind_req("ONB-GOOD", ios_current())).outcome,
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

proptest! {
    /// AC8: the member-LIST summary carries no PII beyond the (intentionally-plain) name — for any
    /// name, the issued `MemberSummary` serializes to exactly {member_id, name, roles,
    /// onboarding_status} (so no phone/address field could have crept in) with the name in the clear.
    #[test]
    fn prop_member_summary_never_carries_pii(name in "[A-Za-z .'-]{1,30}") {
        let mut svc = member_service(MemMemberStore::bootstrapped(), 1_000);
        let outcome = block_on(svc.issue_member(
            member_id(1),
            IssueMemberInput {
                name: MemberName::new(name.clone()),
                phone: PhoneNumber::new("+15550000000"),
                address: Address::new("1 Test St"),
                roles: vec![IssuableRole::Rider],
            },
            "req".into(),
        )).unwrap();
        let member = match outcome {
            IssueMemberOutcome::Issued { member, .. } => member,
            _ => return Err(TestCaseError::fail("expected Issued")),
        };
        let json = serde_json::to_value(&member).unwrap();
        let obj = json.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        prop_assert_eq!(keys, vec!["member_id", "name", "onboarding_status", "roles"]);
        prop_assert_eq!(obj["name"].as_str().unwrap(), name);
    }

    /// AC7/I5: every detail read of an existing member emits exactly one audit row (and a not-found
    /// read emits none) — so the audit count equals the number of successful PII reads.
    #[test]
    fn prop_every_pii_detail_read_emits_audit(reads in 1usize..20) {
        let mut svc = member_service(MemMemberStore::bootstrapped(), 1_000);
        let id = match block_on(svc.issue_member(
            member_id(1),
            IssueMemberInput {
                name: MemberName::new("Maria"),
                phone: PhoneNumber::new("+15550000000"),
                address: Address::new("1 Test St"),
                roles: vec![IssuableRole::Rider],
            },
            "req".into(),
        )).unwrap() {
            IssueMemberOutcome::Issued { member, .. } => member.member_id,
            _ => return Err(TestCaseError::fail("expected Issued")),
        };
        // A not-found read writes no audit.
        block_on(svc.read_detail(member_id(2), member_id(987_654), "miss".into())).unwrap();
        for i in 0..reads {
            block_on(svc.read_detail(member_id(2), id, format!("req-{i}"))).unwrap();
        }
        prop_assert_eq!(svc.store.recorded_audits().len(), reads);
    }

    /// AC11: after an edit changes the phone, the stored lookup hash matches the NEW number and no
    /// longer the old one (so the member's next sign-in matches what was just set).
    #[test]
    fn prop_phone_change_recomputes_matching_hash(n in 0u64..1_000_000_000_000) {
        let new_phone = format!("+1{n:012}");
        let old_phone = "+15550000000";
        prop_assume!(new_phone != old_phone);

        let mut svc = member_service(MemMemberStore::bootstrapped(), 1_000);
        let id = match block_on(svc.issue_member(
            member_id(1),
            IssueMemberInput {
                name: MemberName::new("Maria"),
                phone: PhoneNumber::new(old_phone),
                address: Address::new("1 Test St"),
                roles: vec![IssuableRole::Rider],
            },
            "req".into(),
        )).unwrap() {
            IssueMemberOutcome::Issued { member, .. } => member.member_id,
            _ => return Err(TestCaseError::fail("expected Issued")),
        };
        let updated_at = svc.store.stored_updated_at(id).unwrap();
        let edited = block_on(svc.edit_member(
            id,
            EditMemberInput {
                name: None,
                phone: Some(PhoneNumber::new(new_phone.clone())),
                address: None,
                roles: None,
                expected_updated_at: updated_at,
            },
            "edit".into(),
        )).unwrap();
        prop_assert_eq!(edited, EditMemberOutcome::Updated);

        let stored = svc.store.stored_phone_lookup(id).unwrap();
        prop_assert!(phone_lookup_matches(&key(), &PhoneNumber::new(new_phone), &stored));
        prop_assert!(!phone_lookup_matches(&key(), &PhoneNumber::new(old_phone), &stored));
    }
}
