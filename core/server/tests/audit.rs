//! Audit + projection tests (spec 008 T05, I5/AC7/AC8/AC9): the audited detail read emits exactly one
//! `AuditEntry` (field names, server time, actor, request id), the audit-log read carries no values,
//! the wire projection casing is pinned, and the two-type `MemberDetail` split is compile-enforced.

mod common;

use boundless_server_core::{
    AuditField, AuditedResponse, DetailRead, IssueMemberOutcome, MemberDetail, MemberDetailView,
    MemberSummary, OnboardingStatus, PiiDisclosure,
};
use common::{block_on, member_id, member_service, MemMemberStore};
use static_assertions::{assert_impl_all, assert_not_impl_any};

const NOW: i64 = 1_700_000_000;

// AC8 / the two-type split + the T06 I5 gate — all compile-time:
// - `MemberSummary` IS `Serialize` (so — since the tainted PII types are NOT `Serialize` — a tainted
//   field could not compile in, the AC8 guarantee) AND is an allowlisted `AuditedResponse` (listing
//   is not an audited read).
// - the CORE `MemberDetail` is NOT serializable (tainted PII; P2 by construction).
// - the WIRE `MemberDetailView` keeps `Serialize` (only so the disclosure can emit the wire body) but
//   is NOT an `AuditedResponse` on its own (T06): the bare view cannot pass `admin_response_body`, and
//   its fields are private + un-constructible outside the crate. The only sendable carrier is
//   `PiiDisclosure<MemberDetailView>`, mintable only after an audit row is committed (the I5 gate).
assert_impl_all!(MemberSummary: serde::Serialize, AuditedResponse);
assert_not_impl_any!(MemberDetail: serde::Serialize);
assert_impl_all!(MemberDetailView: serde::Serialize);
assert_not_impl_any!(MemberDetailView: AuditedResponse);
assert_impl_all!(PiiDisclosure<MemberDetailView>: serde::Serialize, AuditedResponse);

use boundless_domain::{Address, MemberName, PhoneNumber};
use boundless_server_core::{IssuableRole, IssueMemberInput};

fn input(name: &str, phone: &str, address: &str) -> IssueMemberInput {
    IssueMemberInput {
        name: MemberName::new(name),
        phone: PhoneNumber::new(phone),
        address: Address::new(address),
        roles: vec![IssuableRole::Rider],
    }
}

fn issue(
    svc: &mut common::TestMemberService,
    name: &str,
    phone: &str,
    address: &str,
) -> boundless_domain::MemberId {
    match block_on(svc.issue_member(
        member_id(1),
        input(name, phone, address),
        "req-issue".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member.member_id,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    }
}

#[test]
fn member_summary_holds_no_tainted_type() {
    // The substantive guard is the module-scope `assert_impl_all!(MemberSummary: Serialize)` above —
    // a tainted field would fail to compile. This runtime check confirms a summary serializes to a
    // plain object with the name in the clear (AC3) and the canonical fields.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let id = issue(&mut svc, "Maria", "+15551230001", "12 Olive St");
    let summary = MemberSummary {
        member_id: id,
        name: "Maria".into(),
        roles: vec![boundless_domain::Role::Rider],
        onboarding_status: OnboardingStatus::IssuedNotOnboarded,
    };
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["name"], "Maria");
    assert_eq!(json["onboarding_status"], "issued_not_onboarded");
}

#[test]
fn audit_entry_carries_field_names_ts_admin_member_request() {
    // AC7/I5: a detail read emits an AuditEntry with timestamp (SERVER time), admin id, member id,
    // field NAMES, and the request id.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let id = issue(&mut svc, "Maria", "+15551230001", "12 Olive St");

    let disclosure = match block_on(svc.read_detail(member_id(2), id, "req-detail".into())).unwrap()
    {
        DetailRead::Detail(disclosure) => disclosure,
        other => panic!("expected Detail, got error_code {:?}", other.error_code()),
    };
    // The decrypted wire view is correct — serialized through the audited disclosure (T06: the bare
    // `MemberDetailView` has private fields, reachable on the wire only via `PiiDisclosure`, which an
    // audit minted; the disclosure's `Serialize` delegates to the view).
    let view = serde_json::to_value(&disclosure).unwrap();
    assert_eq!(view["name"], "Maria");
    assert_eq!(view["phone"], "+15551230001");
    assert_eq!(view["address"], "12 Olive St");
    // The disclosure carries the committed audit (the I5 binding) — the same record the store recorded.
    assert_eq!(disclosure.audit().request_id, "req-detail");
    assert_eq!(
        disclosure.audit().fields,
        vec![AuditField::Name, AuditField::Phone, AuditField::Address]
    );

    let audits = svc.store.recorded_audits();
    assert_eq!(audits.len(), 1, "exactly one audit per detail read");
    let a = &audits[0];
    assert_eq!(
        a.timestamp.0, NOW,
        "audit timestamp is server time, not device time"
    );
    assert_eq!(a.admin_id, member_id(2));
    assert_eq!(a.member_id, id);
    assert_eq!(a.request_id, "req-detail");
    assert_eq!(
        a.fields,
        vec![AuditField::Name, AuditField::Phone, AuditField::Address]
    );
    // The audit row carries field NAMES, never values (AC9): its serialized form contains no PII.
    let json = serde_json::to_string(a).unwrap();
    assert!(
        !json.contains("Maria") && !json.contains("12 Olive St") && !json.contains("+15551230001")
    );
    assert!(
        json.contains("\"name\"") && json.contains("\"phone\"") && json.contains("\"address\"")
    );
}

#[test]
fn member_detail_read_emits_exactly_one_audit_per_read() {
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let id = issue(&mut svc, "Maria", "+15551230001", "12 Olive St");
    block_on(svc.read_detail(member_id(2), id, "r1".into())).unwrap();
    block_on(svc.read_detail(member_id(2), id, "r2".into())).unwrap();
    assert_eq!(svc.store.recorded_audits().len(), 2);
}

#[test]
fn member_detail_read_not_found_writes_no_audit() {
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let outcome = block_on(svc.read_detail(member_id(2), member_id(999), "r1".into())).unwrap();
    assert!(matches!(outcome, DetailRead::NotFound));
    assert_eq!(outcome.error_code(), None); // a plain 404, no body code
    assert!(
        svc.store.recorded_audits().is_empty(),
        "a not-found read discloses no PII, so it writes no audit row"
    );
}

#[test]
fn member_detail_read_fails_closed_without_group_key() {
    // No Group key ⇒ the read fails closed BEFORE any audited SELECT (no audit for a read that can't
    // complete). Empty, un-bootstrapped store.
    let mut svc = member_service(MemMemberStore::new(), NOW);
    let outcome = block_on(svc.read_detail(member_id(2), member_id(1), "r1".into())).unwrap();
    assert!(matches!(outcome, DetailRead::GroupKeyMissing));
    assert_eq!(outcome.error_code(), Some("ADMIN_GROUP_KEY_MISSING"));
    assert!(svc.store.recorded_audits().is_empty());
}

#[test]
fn read_audit_log_filters_by_member_and_carries_no_values() {
    // AC9: the audit-log read returns field names, filterable by member, and is not itself a PII read.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let a = issue(&mut svc, "Maria", "+15551230001", "12 Olive St");
    let b = issue(&mut svc, "Daniel", "+15551230002", "5 Birch Rd");
    block_on(svc.read_detail(member_id(2), a, "ra".into())).unwrap();
    block_on(svc.read_detail(member_id(2), b, "rb".into())).unwrap();

    let all = block_on(svc.read_audit_log(None)).unwrap();
    assert_eq!(all.len(), 2);
    let just_a = block_on(svc.read_audit_log(Some(a))).unwrap();
    assert_eq!(just_a.len(), 1);
    assert_eq!(just_a[0].member_id, a);
    // The whole log serializes with no PII values.
    let json = serde_json::to_string(&all).unwrap();
    assert!(!json.contains("Maria") && !json.contains("Daniel"));
}

#[test]
fn onboarding_status_wire_casing() {
    // The frozen wire spelling T08 mirrors into the OpenAPI `OnboardingStatus` enum.
    assert_eq!(
        serde_json::to_string(&OnboardingStatus::IssuedNotOnboarded).unwrap(),
        "\"issued_not_onboarded\""
    );
    assert_eq!(
        serde_json::to_string(&OnboardingStatus::Onboarded).unwrap(),
        "\"onboarded\""
    );
    assert_eq!(
        serde_json::to_string(&OnboardingStatus::CodeExpiredOrLost).unwrap(),
        "\"code_expired_or_lost\""
    );
    assert_eq!(
        serde_json::to_string(&OnboardingStatus::NeedsReonboarding).unwrap(),
        "\"needs_reonboarding\""
    );
}

#[test]
fn audit_field_as_str_casing() {
    // `as_str()` (the DB `text[]` token) and the serde wire form must be the SAME token (single source).
    for f in [AuditField::Name, AuditField::Phone, AuditField::Address] {
        assert_eq!(
            serde_json::to_string(&f).unwrap(),
            format!("\"{}\"", f.as_str())
        );
    }
    assert_eq!(AuditField::Name.as_str(), "name");
    assert_eq!(AuditField::Phone.as_str(), "phone");
    assert_eq!(AuditField::Address.as_str(), "address");
}

#[test]
fn member_detail_view_wire_keys_are_pinned() {
    // Lock the wire `MemberDetailView` key set in-core now (mirrors `prop_member_summary_never_carries_pii`
    // for the summary), so a field rename is caught here — not only at T08's OpenAPI contract test, and
    // never via an unguarded hand-rolled Worker projection (the ManifestPointer-miss class).
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let id = issue(&mut svc, "Maria", "+15551230001", "12 Olive St");
    let view = match block_on(svc.read_detail(member_id(2), id, "req".into())).unwrap() {
        DetailRead::Detail(view) => view,
        other => panic!("expected Detail, got {:?}", other.error_code()),
    };
    let json = serde_json::to_value(&view).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "address",
            "member_id",
            "name",
            "onboarding_status",
            "phone",
            "roles",
            "updated_at"
        ]
    );
}
