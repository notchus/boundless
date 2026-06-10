//! `MemberService` decision-layer tests (spec 008 T05): issuance, edit, regenerate, list, the
//! duplicate-phone surface-and-link, validation rejects, and the AC12 fail-closed path. Drives the
//! pure core over the in-memory member-store double (`common::MemMemberStore`); the real DB-level
//! atomicity / uniqueness proofs are T07.

mod common;

use boundless_crypto::{
    decrypt_field, onboarding_code_matches, phone_lookup_matches, MAC_LEN, NONCE_LEN,
};
use boundless_domain::{Address, MemberName, PhoneNumber, Role};
use boundless_server_core::{
    issuable_roles, AdminRoleForbidden, EditMemberInput, EditMemberOutcome, IssuableRole,
    IssueMemberInput, IssueMemberOutcome, MemberError, OnboardingStatus, RegenerateOutcome,
    ONBOARDING_CODE_TTL_SECS,
};
use common::{
    block_on, bootstrapped_store_with_key, key, member_id, member_service, MemMemberStore,
};

// A fixed server instant the FixedClock returns (the "admin's clock is wrong" edge is moot — the
// service uses this server time for the code TTL + audit timestamps).
const NOW: i64 = 1_700_000_000;

/// Build an issuance input from human-form fields (the phone is raw — the core normalizes it).
fn input(name: &str, phone: &str, address: &str, roles: Vec<IssuableRole>) -> IssueMemberInput {
    IssueMemberInput {
        name: MemberName::new(name),
        phone: PhoneNumber::new(phone),
        address: Address::new(address),
        roles,
    }
}

#[test]
fn member_service_issues_rider_and_driver() {
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);

    let rider = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got error_code {:?}", other.error_code()),
    };
    assert_eq!(rider.name, "Maria");
    assert_eq!(rider.roles, vec![Role::Rider]);
    assert_eq!(
        rider.onboarding_status,
        OnboardingStatus::IssuedNotOnboarded
    );

    let driver = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Daniel",
            "+15551230002",
            "5 Birch Rd",
            vec![IssuableRole::Driver],
        ),
        "req-2".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    assert_eq!(driver.roles, vec![Role::Driver]);

    assert_eq!(svc.store.member_count(), 2);
    assert_ne!(rider.member_id, driver.member_id);
    // Issuance is a write, not an audited read — no audit row.
    assert!(svc.store.recorded_audits().is_empty());
}

#[test]
fn member_service_accepts_multi_role_set() {
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let member = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Margaret",
            "+15551230003",
            "9 Cedar Ln",
            vec![IssuableRole::Rider, IssuableRole::Driver],
        ),
        "req-3".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    assert_eq!(member.roles, vec![Role::Rider, Role::Driver]);
}

#[test]
fn member_service_rejects_admin_role_on_issuance() {
    // Admin is unrepresentable in `IssueMemberInput.roles`; the one seam an Admin can be named is the
    // wire `Vec<Role>` → `Vec<IssuableRole>` conversion, which refuses it (I11/AC10).
    let err = issuable_roles(&[Role::Rider, Role::Admin]).unwrap_err();
    assert_eq!(err, AdminRoleForbidden);
    assert_eq!(err.error_code(), "ADMIN_MEMBER_ROLE_FORBIDDEN");
    // Driver-only at the front still fails if Admin appears anywhere.
    assert!(issuable_roles(&[Role::Admin]).is_err());
    // The issuable subset converts cleanly.
    assert_eq!(
        issuable_roles(&[Role::Rider, Role::Driver]).unwrap(),
        vec![IssuableRole::Rider, IssuableRole::Driver]
    );
}

#[test]
fn member_service_stores_phone_hash_and_ciphertext() {
    // AC4 (phone two-fold) at the decision level + AC1 normalization. Issue with a HUMAN-form phone;
    // assert the stored lookup hash is over the CANONICAL form (so the next sign-in matches) and the
    // stored ciphertext decrypts back to the canonical phone.
    let (store, group_key) = bootstrapped_store_with_key();
    let mut svc = member_service(store, NOW);
    let member = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+1 (555) 123-0001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    let canonical = PhoneNumber::new("+15551230001");
    let stored = svc.store.stored_phone_lookup(member.member_id).unwrap();
    assert!(
        phone_lookup_matches(&key(), &canonical, &stored),
        "lookup hash must be over the canonical E.164 form (AC4 — sign-in match)"
    );
    assert!(
        !phone_lookup_matches(&key(), &PhoneNumber::new("+1 (555) 123-0001"), &stored),
        "the raw human form must not be what was hashed (normalization happened in-core)"
    );
    // The encrypted phone round-trips to the canonical form, and is genuinely ciphertext.
    let phone_ct = svc.store.stored_phone_encrypted(member.member_id).unwrap();
    assert_eq!(phone_ct.len(), NONCE_LEN + MAC_LEN + "+15551230001".len());
    let recovered = decrypt_field(&phone_ct, &group_key).unwrap();
    assert_eq!(recovered, b"+15551230001");
    // Name + address are encrypted at rest too (I1).
    let name_ct = svc.store.stored_name_encrypted(member.member_id).unwrap();
    assert_eq!(decrypt_field(&name_ct, &group_key).unwrap(), b"Maria");
    let addr_ct = svc
        .store
        .stored_address_encrypted(member.member_id)
        .unwrap();
    assert_eq!(decrypt_field(&addr_ct, &group_key).unwrap(), b"12 Olive St");
}

#[test]
fn member_service_mints_one_live_onboarding_code() {
    // AC5 decision: exactly one code, returned in the clear once, hashed at rest, server-time TTL.
    // (The "at most one live" DB invariant is T07's partial-unique index.)
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let (member, code, expires) = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued {
            member,
            onboarding_code,
            code_expires_at,
        } => (member, onboarding_code, code_expires_at),
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    assert_eq!(expires.0, NOW + ONBOARDING_CODE_TTL_SECS);
    let (stored_hash, stored_expires) = svc.store.live_code(member.member_id).unwrap();
    assert_eq!(stored_expires, expires);
    assert!(
        onboarding_code_matches(&key(), &code, &stored_hash),
        "the at-rest hash must verify the returned plaintext code (AC5)"
    );
}

#[test]
fn member_service_regenerate_supersedes_decision() {
    // AC6 decision: regenerate mints a fresh code and supersedes the prior (the prior no longer
    // verifies against the live hash). The atomic supersede-then-insert SQL is T07.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let (member, first_code) = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued {
            member,
            onboarding_code,
            ..
        } => (member, onboarding_code),
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    let fresh = match block_on(svc.regenerate_onboarding_code(member.member_id)).unwrap() {
        RegenerateOutcome::Regenerated {
            onboarding_code, ..
        } => onboarding_code,
        RegenerateOutcome::NotFound => panic!("member exists"),
    };
    let (live_hash, _) = svc.store.live_code(member.member_id).unwrap();
    assert!(
        onboarding_code_matches(&key(), &fresh, &live_hash),
        "the fresh code must verify against the new live hash"
    );
    assert!(
        !onboarding_code_matches(&key(), &first_code, &live_hash),
        "the superseded code must no longer verify (AC6)"
    );
    // Regenerating for an unknown member is a no-op NotFound.
    assert!(matches!(
        block_on(svc.regenerate_onboarding_code(member_id(999))).unwrap(),
        RegenerateOutcome::NotFound
    ));
}

#[test]
fn member_list_emits_no_audit_event() {
    // AC8: listing decrypts only names (the P2-sensitive unit is the name+address PAIR), so it is not
    // an audited read — and the names round-trip to the issued plaintext.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap();
    block_on(svc.issue_member(
        member_id(1),
        input(
            "Daniel",
            "+15551230002",
            "5 Birch Rd",
            vec![IssuableRole::Driver],
        ),
        "req-2".into(),
    ))
    .unwrap();

    let list = block_on(svc.list_members()).unwrap().unwrap();
    let mut names: Vec<String> = list.iter().map(|m| m.name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["Daniel".to_string(), "Maria".to_string()]);
    assert!(
        svc.store.recorded_audits().is_empty(),
        "the list path must emit no audit event (AC8)"
    );
}

#[test]
fn member_service_edit_reencrypts_and_recomputes_phone_hash() {
    // AC11: a phone change recomputes the lookup hash (next sign-in matches the NEW number, not the
    // old) and re-encrypts the phone; a changed address gets a FRESH nonce (R1) and new ciphertext.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let member = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    let original_addr_ct = svc
        .store
        .stored_address_encrypted(member.member_id)
        .unwrap();
    let updated_at = svc.store.stored_updated_at(member.member_id).unwrap();

    let outcome = block_on(svc.edit_member(
        member.member_id,
        EditMemberInput {
            name: None,
            phone: Some(PhoneNumber::new("+15559990000")),
            address: Some(Address::new("99 Maple Ave")),
            roles: None,
            expected_updated_at: updated_at,
        },
        "req-edit".into(),
    ))
    .unwrap();
    assert_eq!(outcome, EditMemberOutcome::Updated);

    // The lookup hash moved to the NEW phone; the OLD phone no longer matches (AC11 sign-in match).
    let stored = svc.store.stored_phone_lookup(member.member_id).unwrap();
    assert!(phone_lookup_matches(
        &key(),
        &PhoneNumber::new("+15559990000"),
        &stored
    ));
    assert!(!phone_lookup_matches(
        &key(),
        &PhoneNumber::new("+15551230001"),
        &stored
    ));
    // The address was re-encrypted (fresh nonce ⇒ different ciphertext bytes).
    let new_addr_ct = svc
        .store
        .stored_address_encrypted(member.member_id)
        .unwrap();
    assert_ne!(
        new_addr_ct, original_addr_ct,
        "changed address must be re-encrypted (R1 fresh nonce)"
    );
}

#[test]
fn member_service_edit_role_only_needs_no_group_key() {
    // A role-only edit changes no PII, so it must not require the Group key (it is never loaded).
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let member = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Daniel",
            "+15551230002",
            "5 Birch Rd",
            vec![IssuableRole::Driver],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    let updated_at = svc.store.stored_updated_at(member.member_id).unwrap();
    let outcome = block_on(svc.edit_member(
        member.member_id,
        EditMemberInput {
            name: None,
            phone: None,
            address: None,
            roles: Some(vec![IssuableRole::Rider, IssuableRole::Driver]),
            expected_updated_at: updated_at,
        },
        "req-edit".into(),
    ))
    .unwrap();
    assert_eq!(outcome, EditMemberOutcome::Updated);
    assert_eq!(
        svc.store.stored_roles(member.member_id).unwrap(),
        vec![Role::Rider, Role::Driver]
    );
}

#[test]
fn member_service_stale_edit_rejected() {
    // AC11: an edit loaded against a stale `updated_at` is rejected with no partial write.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let member = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    let outcome = block_on(svc.edit_member(
        member.member_id,
        EditMemberInput {
            name: Some(MemberName::new("Maria R.")),
            phone: None,
            address: None,
            roles: None,
            expected_updated_at: boundless_auth::UnixSeconds::new(NOW - 1), // wrong token
        },
        "req-edit".into(),
    ))
    .unwrap();
    assert_eq!(outcome, EditMemberOutcome::Stale);
    assert_eq!(outcome.error_code(), Some("ADMIN_MEMBER_EDIT_STALE"));
}

#[test]
fn member_service_rejects_invalid_phone_and_address() {
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    // No leading '+' → not E.164.
    let bad_phone = block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "5551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap();
    assert!(matches!(
        bad_phone,
        IssueMemberOutcome::Rejected(MemberError::PhoneInvalid)
    ));
    assert_eq!(bad_phone.error_code(), Some("ADMIN_MEMBER_PHONE_INVALID"));
    // Empty address.
    let bad_addr = block_on(svc.issue_member(
        member_id(1),
        input("Maria", "+15551230001", "   ", vec![IssuableRole::Rider]),
        "req-2".into(),
    ))
    .unwrap();
    assert!(matches!(
        bad_addr,
        IssueMemberOutcome::Rejected(MemberError::AddressInvalid)
    ));
    assert_eq!(bad_addr.error_code(), Some("ADMIN_MEMBER_ADDRESS_INVALID"));
    // No member was written on either reject.
    assert_eq!(svc.store.member_count(), 0);
}

#[test]
fn member_service_rejects_empty_roles() {
    // AC13: a member must hold at least one role; an empty set is rejected before any write.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let outcome = block_on(svc.issue_member(
        member_id(1),
        input("Maria", "+15551230001", "12 Olive St", vec![]),
        "req-1".into(),
    ))
    .unwrap();
    assert!(matches!(
        outcome,
        IssueMemberOutcome::Rejected(MemberError::RolesRequired)
    ));
    assert_eq!(outcome.error_code(), Some("ADMIN_MEMBER_ROLES_REQUIRED"));
    assert_eq!(svc.store.member_count(), 0, "no roleless member is written");
}

#[test]
fn member_service_edit_to_empty_roles_rejected() {
    // AC13: an edit that would clear all roles is rejected (None = leave roles unchanged is fine).
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let member = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Daniel",
            "+15551230002",
            "5 Birch Rd",
            vec![IssuableRole::Driver],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    let updated_at = svc.store.stored_updated_at(member.member_id).unwrap();
    let outcome = block_on(svc.edit_member(
        member.member_id,
        EditMemberInput {
            name: None,
            phone: None,
            address: None,
            roles: Some(vec![]), // clear all roles → invalid
            expected_updated_at: updated_at,
        },
        "req-edit".into(),
    ))
    .unwrap();
    assert_eq!(
        outcome,
        EditMemberOutcome::Rejected(MemberError::RolesRequired)
    );
    // The member's role set is untouched (no partial write).
    assert_eq!(
        svc.store.stored_roles(member.member_id).unwrap(),
        vec![Role::Driver]
    );
}

#[test]
fn member_service_issuance_fails_closed_without_group_key() {
    // AC12 at the service level: no Group key ⇒ no member is written, no address stored unencrypted.
    let mut svc = member_service(MemMemberStore::new(), NOW); // NOT bootstrapped — no wrapped key
    let outcome = block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap();
    assert!(matches!(
        outcome,
        IssueMemberOutcome::Rejected(MemberError::GroupKeyMissing)
    ));
    assert_eq!(outcome.error_code(), Some("ADMIN_GROUP_KEY_MISSING"));
    assert_eq!(
        svc.store.member_count(),
        0,
        "fail closed — no member row written"
    );
    assert!(svc.store.recorded_audits().is_empty());
}

#[test]
fn member_service_duplicate_phone_links_existing_and_audits() {
    // AC1 edge / I5: a second member with the same phone surfaces-and-links the existing member
    // (name only) and the disclosure is audited. Never modeled as an error.
    let mut svc = member_service(MemMemberStore::bootstrapped(), NOW);
    let first = match block_on(svc.issue_member(
        member_id(1),
        input(
            "Maria",
            "+15551230001",
            "12 Olive St",
            vec![IssuableRole::Rider],
        ),
        "req-1".into(),
    ))
    .unwrap()
    {
        IssueMemberOutcome::Issued { member, .. } => member,
        other => panic!("expected Issued, got {:?}", other.error_code()),
    };
    // Same phone in a human form (normalizes to the same canonical → same lookup hash).
    let outcome = block_on(svc.issue_member(
        member_id(7),
        input(
            "Imposter",
            "+1 (555) 123-0001",
            "1 Fake St",
            vec![IssuableRole::Rider],
        ),
        "req-dup".into(),
    ))
    .unwrap();
    let existing = match outcome {
        IssueMemberOutcome::DuplicatePhone { existing } => existing,
        other => panic!("expected DuplicatePhone, got {:?}", other.error_code()),
    };
    assert_eq!(existing.member_id, first.member_id);
    assert_eq!(existing.name, "Maria"); // links the EXISTING member, not the new submission
    assert_eq!(
        svc.store.member_count(),
        1,
        "the duplicate must not create a second member"
    );
    // The disclosure was audited (I5): one entry, name-only, against the existing member, by the actor.
    let audits = svc.store.recorded_audits();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].member_id, first.member_id);
    assert_eq!(audits[0].admin_id, member_id(7));
    assert_eq!(audits[0].request_id, "req-dup");
    assert_eq!(
        audits[0].fields,
        vec![boundless_server_core::AuditField::Name]
    );
}
