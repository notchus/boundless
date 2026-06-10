//! Real-Postgres integration tests for [`PgMemberStore`] at the store level (spec 008 **T07**).
//!
//! These prove against a live PG18 the member-management contracts the in-memory `MemMemberStore`
//! stub can only *model* (single-threaded ⇒ trivially atomic): atomic member + code mint, the
//! duplicate-phone surface-and-link audit, atomic regenerate (supersede-then-insert), optimistic
//! concurrency on `updated_at`, the I1 ciphertext round-trip through the `bytea` columns, the I3
//! two-fold phone (incl. the AC4 sign-in match), and RLS tenant isolation (incl. a proptest over
//! random two-group configs).
//!
//! Harness (connect / role / schema / seeding / assertions + `url_or_skip!`) lives in `common`.

mod common;

use boundless_auth::UnixSeconds;
use boundless_crypto::{onboarding_code_matches, CodeHash};
use boundless_domain::{MemberId, OnboardingCode, Role};
use boundless_server_core::{
    AuditEntry, AuditField, AuthStore, DuplicateDisclosureAudit, EditApplied, InsertMemberOutcome,
    MemberEditWrite, MemberStore, NewMemberWrite, OnboardingStatus,
};
use proptest::prelude::*;
use uuid::Uuid;

use common::*;

/// A whole-second server-time instant well in the future of the DB clock, so a freshly-minted code's
/// TTL is not "past" (the status derivation is TTL-agnostic, but a realistic `now` keeps the data sane).
const NOW: i64 = 1_700_000_000;
const CODE_TTL: i64 = 72 * 60 * 60;

/// Build a `NewMemberWrite` with real ciphertext (encrypted under [`group_key`]) so the round-trip
/// tests decrypt exactly what the store persisted. `created_by` is the acting admin.
fn write_for(
    created_by: MemberId,
    roles: Vec<Role>,
    raw_phone: &str,
    name: &str,
    address: &str,
    code: &str,
) -> NewMemberWrite {
    NewMemberWrite {
        created_by,
        roles,
        phone_lookup: phone_lookup(raw_phone),
        phone_encrypted: enc(raw_phone, 1),
        name_encrypted: enc(name, 2),
        address_encrypted: enc(address, 3),
        onboarding_code_hash: onb_code_hash(code),
        code_expires_at: UnixSeconds::new(NOW + CODE_TTL),
    }
}

/// The disclosure-audit context the duplicate-phone path stamps (name only).
fn dup_disclosure(req: &str) -> DuplicateDisclosureAudit {
    DuplicateDisclosureAudit {
        request_id: req.to_string(),
        fields: vec![AuditField::Name],
    }
}

#[tokio::test]
async fn pg_member_store_persists_member_with_roles_and_created_by() {
    let url = url_or_skip!();
    let su = setup(&url, "s_m_persist").await;
    let g = Uuid::from_u128(G);
    let admin = mid(7);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_persist", g).await;
    let outcome = store
        .insert_member(
            write_for(
                admin,
                vec![Role::Rider],
                "+15550000201",
                "Maria",
                "1 Oak St",
                "CODE-1",
            ),
            dup_disclosure("unused"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap();
    let InsertMemberOutcome::Created(m) = outcome else {
        panic!("expected a clean create");
    };

    // The row exists in this Group with the role + created_by (audit write-side, I5).
    let mu = m.as_uuid();
    let row = su
        .query_one(
            "SELECT roles::text[] AS roles, created_by FROM members WHERE id=$1 AND group_id=$2",
            &[&mu, &g],
        )
        .await
        .expect("member row");
    let roles: Vec<String> = row.get("roles");
    assert_eq!(roles, vec!["rider".to_string()]);
    assert_eq!(row.get::<_, Uuid>("created_by"), admin.as_uuid());
}

#[tokio::test]
async fn pg_member_store_address_encrypted_round_trip() {
    // AC2: the stored address column is bytea ciphertext (ciphertext != plaintext), and the round-trip
    // requires the per-Group key — proven by encrypting under group_key(), persisting via the store,
    // reading back through the audited detail read, and decrypting to the original.
    let url = url_or_skip!();
    let su = setup(&url, "s_m_addr").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_addr", g).await;
    let InsertMemberOutcome::Created(m) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Rider],
                "+15550000202",
                "Maria",
                "47 Willow Lane",
                "C",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };

    // The stored column is ciphertext, not the plaintext.
    let mu = m.as_uuid();
    let stored: Vec<u8> = su
        .query_one("SELECT address_encrypted FROM members WHERE id=$1", &[&mu])
        .await
        .unwrap()
        .get("address_encrypted");
    assert_ne!(
        stored,
        b"47 Willow Lane".to_vec(),
        "address stored as ciphertext, not plaintext"
    );

    // The audited detail read returns the ciphertext; decrypting with the Group key recovers it.
    let audit = AuditEntry {
        timestamp: UnixSeconds::new(NOW),
        admin_id: mid(7),
        member_id: m,
        fields: vec![AuditField::Name, AuditField::Phone, AuditField::Address],
        request_id: "r".into(),
    };
    let pii = store
        .read_member_detail_audited(m, audit)
        .await
        .unwrap()
        .expect("member found");
    assert_eq!(dec(&pii.address_encrypted), "47 Willow Lane");
    assert_eq!(dec(&pii.name_encrypted), "Maria");
}

#[tokio::test]
async fn pg_member_store_phone_two_fold() {
    // AC4 / I3: phone is stored two-fold — the keyed lookup hash (for the constant-time sign-in
    // lookup) AND the display ciphertext. Proven by reading both columns back AND by `PgAuthStore`
    // (the sign-in path) finding the member by that lookup hash — so issuance feeds sign-in.
    let url = url_or_skip!();
    let su = setup(&url, "s_m_phone").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_phone", g).await;
    let InsertMemberOutcome::Created(m) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Rider],
                "+15550000203",
                "Maria",
                "1 Oak St",
                "C",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };

    // Both columns are present and the encrypted phone decrypts to the E.164 number.
    let mu = m.as_uuid();
    let row = su
        .query_one(
            "SELECT phone_lookup_hash, phone_encrypted FROM members WHERE id=$1",
            &[&mu],
        )
        .await
        .unwrap();
    assert_eq!(
        row.get::<_, Vec<u8>>("phone_lookup_hash"),
        phone_lookup("+15550000203").as_bytes().to_vec()
    );
    assert_eq!(
        dec(&row.get::<_, Vec<u8>>("phone_encrypted")),
        "+15550000203"
    );

    // The sign-in path (PgAuthStore::find_member_by_phone) now matches this member by the lookup hash.
    let mut auth = app_store(&url, "s_m_phone", g).await;
    let found = auth
        .find_member_by_phone(&phone_lookup("+15550000203"))
        .await
        .unwrap()
        .expect("issuance feeds the sign-in lookup");
    assert_eq!(found.member_id, m);
}

#[tokio::test]
async fn pg_member_store_roles_array_round_trip() {
    // AC13: a member may hold more than one role; the role set round-trips through the member_role[].
    let url = url_or_skip!();
    let su = setup(&url, "s_m_roles").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_roles", g).await;
    let InsertMemberOutcome::Created(m) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Rider, Role::Driver],
                "+15550000204",
                "Dan",
                "2 Elm",
                "C",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };

    let listed = store.list_members().await.unwrap();
    let summary = listed.iter().find(|s| s.member_id == m).expect("listed");
    assert!(summary.roles.contains(&Role::Rider) && summary.roles.contains(&Role::Driver));
    assert_eq!(summary.roles.len(), 2);
    assert_eq!(dec(&summary.name_encrypted), "Dan");
    assert_eq!(
        summary.onboarding_status,
        OnboardingStatus::IssuedNotOnboarded
    );
}

#[tokio::test]
async fn pg_member_store_issue_is_atomic() {
    // The member row and its Onboarding Code are created together in one transaction (R13): after a
    // clean issue, exactly one member and one live code exist.
    let url = url_or_skip!();
    let su = setup(&url, "s_m_atomic").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_atomic", g).await;
    let InsertMemberOutcome::Created(m) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Rider],
                "+15550000205",
                "Maria",
                "1 Oak",
                "C",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };
    assert_eq!(members_in_group(&su, g).await, 1, "exactly one member");
    assert_eq!(
        live_codes(&su, m.as_uuid()).await,
        1,
        "exactly one live code, minted atomically"
    );
}

#[tokio::test]
async fn pg_member_store_duplicate_phone_links_existing_and_audits() {
    // The duplicate-phone path surfaces-and-links the existing member (name only) and writes the I5
    // disclosure audit in the same transaction — and creates NO new member, mints NO new code.
    let url = url_or_skip!();
    let su = setup(&url, "s_m_dup").await;
    let g = Uuid::from_u128(G);
    let admin = mid(7);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_dup", g).await;
    let InsertMemberOutcome::Created(first) = store
        .insert_member(
            write_for(
                admin,
                vec![Role::Rider],
                "+15550000206",
                "Maria",
                "1 Oak",
                "C1",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };

    // Second issue with the SAME phone → DuplicatePhone linking the existing member, audited.
    let outcome = store
        .insert_member(
            write_for(
                admin,
                vec![Role::Driver],
                "+15550000206",
                "SomeoneElse",
                "9 Pine",
                "C2",
            ),
            dup_disclosure("dup-req"),
            UnixSeconds::new(NOW + 10),
        )
        .await
        .unwrap();
    let InsertMemberOutcome::DuplicatePhone(existing) = outcome else {
        panic!("expected a duplicate-phone link");
    };
    assert_eq!(existing.member_id, first);
    assert_eq!(
        dec(&existing.name_encrypted),
        "Maria",
        "links the EXISTING member, name only"
    );

    // No new member; the first member's original code is untouched; exactly one disclosure audit row.
    assert_eq!(
        members_in_group(&su, g).await,
        1,
        "no new member on conflict"
    );
    assert_eq!(
        live_codes(&su, first.as_uuid()).await,
        1,
        "conflict mints no new code"
    );
    assert_eq!(
        audit_rows(&su, first.as_uuid()).await,
        1,
        "the disclosure read is audited (I5)"
    );
    let arow = su
        .query_one(
            "SELECT admin_id, array_to_string(fields, ',') AS f, request_id FROM audit_log WHERE member_id=$1",
            &[&first.as_uuid()],
        )
        .await
        .unwrap();
    assert_eq!(arow.get::<_, Uuid>("admin_id"), admin.as_uuid());
    assert_eq!(
        arow.get::<_, String>("f"),
        "name",
        "name only, never address/phone"
    );
    assert_eq!(arow.get::<_, String>("request_id"), "dup-req");
}

#[tokio::test]
async fn pg_member_store_regenerate_supersede_then_insert_atomic() {
    // AC6: regenerate supersedes the prior live code and installs the fresh one atomically; at most
    // one live code per member at any time, and the new live code is the regenerated one.
    let url = url_or_skip!();
    let su = setup(&url, "s_m_regen").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_regen", g).await;
    let InsertMemberOutcome::Created(m) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Rider],
                "+15550000207",
                "Maria",
                "1 Oak",
                "ORIG",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };
    assert_eq!(live_codes(&su, m.as_uuid()).await, 1);

    let regenerated = store
        .regenerate_code(
            m,
            onb_code_hash("FRESH"),
            UnixSeconds::new(NOW + CODE_TTL),
            UnixSeconds::new(NOW + 5),
        )
        .await
        .unwrap();
    assert!(regenerated, "member exists → regenerated");

    // Exactly one live code (the fresh one), two total rows (orig superseded + fresh live).
    assert_eq!(
        live_codes(&su, m.as_uuid()).await,
        1,
        "one live code after regenerate"
    );
    assert_eq!(
        total_codes(&su, m.as_uuid()).await,
        2,
        "supersede-then-INSERT (orig retained, superseded)"
    );
    let live = live_code_hash(&su, m.as_uuid()).await.expect("a live code");
    let live_hash = CodeHash::from_bytes(live.try_into().unwrap());
    assert!(
        onboarding_code_matches(&key(), &OnboardingCode::new("FRESH"), &live_hash),
        "the live code is the regenerated one"
    );
    assert!(
        !onboarding_code_matches(&key(), &OnboardingCode::new("ORIG"), &live_hash),
        "the prior code is no longer live"
    );

    // An unknown member regenerates nothing.
    assert!(!store
        .regenerate_code(
            mid(99),
            onb_code_hash("X"),
            UnixSeconds::new(NOW),
            UnixSeconds::new(NOW)
        )
        .await
        .unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pg_member_store_concurrent_regenerate_keeps_one_live() {
    // The advisory lock (the whole point of regenerate's supersede-then-insert) is load-bearing: two
    // concurrent regenerates must BOTH succeed (serialized, no `onboarding_codes_one_live_per_member`
    // unique violation) and leave EXACTLY one live code — the regenerate twin of
    // `recovery_consume_concurrency` (without the lock, one would error on the partial-unique index).
    let url = url_or_skip!();
    let su = setup(&url, "s_m_regenrace").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    for i in 0..5u128 {
        let mut s = app_member_store(&url, "s_m_regenrace", g).await;
        let InsertMemberOutcome::Created(m) = s
            .insert_member(
                write_for(
                    mid(7),
                    vec![Role::Rider],
                    &format!("+15550003{i:03}"),
                    "M",
                    "addr",
                    "ORIG",
                ),
                dup_disclosure("u"),
                UnixSeconds::new(NOW),
            )
            .await
            .unwrap()
        else {
            panic!("created");
        };
        drop(s);

        let mut a = app_member_store(&url, "s_m_regenrace", g).await;
        let mut b = app_member_store(&url, "s_m_regenrace", g).await;
        let (ca, cb) = (
            onb_code_hash(&format!("A{i}")),
            onb_code_hash(&format!("B{i}")),
        );
        let ha = tokio::spawn(async move {
            a.regenerate_code(
                m,
                ca,
                UnixSeconds::new(NOW + CODE_TTL),
                UnixSeconds::new(NOW + 1),
            )
            .await
            .unwrap()
        });
        let hb = tokio::spawn(async move {
            b.regenerate_code(
                m,
                cb,
                UnixSeconds::new(NOW + CODE_TTL),
                UnixSeconds::new(NOW + 2),
            )
            .await
            .unwrap()
        });
        let (ra, rb) = tokio::join!(ha, hb);
        assert!(
            ra.unwrap() && rb.unwrap(),
            "both concurrent regenerates succeed — serialized by the advisory lock, no unique violation (iter {i})"
        );
        assert_eq!(
            live_codes(&su, m.as_uuid()).await,
            1,
            "exactly one live code after two concurrent regenerates (iter {i})"
        );
    }
}

#[tokio::test]
async fn pg_member_store_optimistic_concurrency_stale_reject() {
    // AC11: an edit whose `expected_updated_at` no longer matches the row's token is rejected with no
    // write (Stale); a matching token applies (Updated). Seeding a controlled `updated_at` makes the
    // whole-second token deterministic (the INSERT seed fires no set_updated_at trigger).
    let url = url_or_skip!();
    let su = setup(&url, "s_m_stale").await;
    let g = Uuid::from_u128(G);
    let m = mid(300);
    seed_group(&su, g).await;
    seed_member_pii(
        &su,
        g,
        m.as_uuid(),
        &["rider"],
        phone_lookup("+15550000208").as_bytes().to_vec(),
        enc("Maria", 2),
        enc("+15550000208", 1),
        enc("1 Oak", 3),
        1000,
    )
    .await;

    let mut store = app_member_store(&url, "s_m_stale", g).await;
    let edit = || MemberEditWrite {
        name_encrypted: Some(enc("Maria Updated", 9)),
        phone_lookup: None,
        phone_encrypted: None,
        address_encrypted: None,
        roles: None,
    };

    // Wrong token (older `updated_at`) → Stale, no write.
    let stale = store
        .edit_member(m, edit(), UnixSeconds::new(999), UnixSeconds::new(NOW))
        .await
        .unwrap();
    assert_eq!(stale, EditApplied::Stale);
    assert_eq!(
        dec(&su
            .query_one(
                "SELECT name_encrypted FROM members WHERE id=$1",
                &[&m.as_uuid()]
            )
            .await
            .unwrap()
            .get::<_, Vec<u8>>("name_encrypted")),
        "Maria",
        "stale edit did not write"
    );

    // Correct token → Updated.
    let ok = store
        .edit_member(m, edit(), UnixSeconds::new(1000), UnixSeconds::new(NOW))
        .await
        .unwrap();
    assert_eq!(ok, EditApplied::Updated);
    assert_eq!(
        dec(&su
            .query_one(
                "SELECT name_encrypted FROM members WHERE id=$1",
                &[&m.as_uuid()]
            )
            .await
            .unwrap()
            .get::<_, Vec<u8>>("name_encrypted")),
        "Maria Updated"
    );

    // A second edit with the now-stale original token is rejected (the trigger moved `updated_at`).
    let stale2 = store
        .edit_member(m, edit(), UnixSeconds::new(1000), UnixSeconds::new(NOW))
        .await
        .unwrap();
    assert_eq!(
        stale2,
        EditApplied::Stale,
        "token moved after the successful edit"
    );
}

#[tokio::test]
async fn pg_member_store_edit_recomputes_phone_lookup() {
    // AC11: a phone change recomputes the lookup hash so the member's NEXT sign-in matches the new
    // number — and the old number no longer resolves.
    let url = url_or_skip!();
    let su = setup(&url, "s_m_editphone").await;
    let g = Uuid::from_u128(G);
    let m = mid(310);
    seed_group(&su, g).await;
    seed_member_pii(
        &su,
        g,
        m.as_uuid(),
        &["rider"],
        phone_lookup("+15550000209").as_bytes().to_vec(),
        enc("Maria", 2),
        enc("+15550000209", 1),
        enc("1 Oak", 3),
        1000,
    )
    .await;

    let mut store = app_member_store(&url, "s_m_editphone", g).await;
    let applied = store
        .edit_member(
            m,
            MemberEditWrite {
                name_encrypted: None,
                phone_lookup: Some(phone_lookup("+15550000299")),
                phone_encrypted: Some(enc("+15550000299", 7)),
                address_encrypted: None,
                roles: None,
            },
            UnixSeconds::new(1000),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap();
    assert_eq!(applied, EditApplied::Updated);

    let mut auth = app_store(&url, "s_m_editphone", g).await;
    assert_eq!(
        auth.find_member_by_phone(&phone_lookup("+15550000299"))
            .await
            .unwrap()
            .map(|r| r.member_id),
        Some(m),
        "the new number resolves"
    );
    assert!(
        auth.find_member_by_phone(&phone_lookup("+15550000209"))
            .await
            .unwrap()
            .is_none(),
        "the old number no longer resolves"
    );
}

#[tokio::test]
async fn pg_member_store_onboarding_status_derivation() {
    // The derived status: a freshly-issued member is IssuedNotOnboarded (live code, no device); a
    // member whose code is consumed AND who has a bound device is Onboarded; a member with no live
    // code and no device is CodeExpiredOrLost.
    let url = url_or_skip!();
    let su = setup(&url, "s_m_status").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_m_status", g).await;
    let InsertMemberOutcome::Created(issued) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Rider],
                "+15550000210",
                "Maria",
                "1 Oak",
                "C",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };

    // A second member: bind a device + consume the code → Onboarded.
    let InsertMemberOutcome::Created(onboarded) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Driver],
                "+15550000211",
                "Dan",
                "2 Elm",
                "C2",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };
    let ou = onboarded.as_uuid();
    su.execute(
        "INSERT INTO device_tokens (group_id, member_id, platform, app_version, token_encrypted) \
         VALUES ($1, $2, 'ios', '1.0.0', $3)",
        &[&g, &ou, &b"tok".to_vec()],
    )
    .await
    .unwrap();
    su.execute(
        "UPDATE onboarding_codes SET consumed_at = now() WHERE member_id=$1",
        &[&ou],
    )
    .await
    .unwrap();

    // A third member: supersede the code without a replacement, no device → CodeExpiredOrLost.
    let InsertMemberOutcome::Created(expired) = store
        .insert_member(
            write_for(
                mid(7),
                vec![Role::Rider],
                "+15550000212",
                "Margaret",
                "3 Birch",
                "C3",
            ),
            dup_disclosure("u"),
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };
    su.execute(
        "UPDATE onboarding_codes SET superseded_at = now() WHERE member_id=$1",
        &[&expired.as_uuid()],
    )
    .await
    .unwrap();

    let listed = store.list_members().await.unwrap();
    let status = |id: MemberId| {
        listed
            .iter()
            .find(|s| s.member_id == id)
            .unwrap()
            .onboarding_status
    };
    assert_eq!(status(issued), OnboardingStatus::IssuedNotOnboarded);
    assert_eq!(status(onboarded), OnboardingStatus::Onboarded);
    assert_eq!(status(expired), OnboardingStatus::CodeExpiredOrLost);
}

#[tokio::test]
async fn rls_isolates_member_reads_by_tenant() {
    // A store scoped to Group A lists only A's members and cannot detail-read a Group-B member
    // (RLS → not found, no audit). The host precursor for AC16 (the live deployed-edge proof is T11).
    let url = url_or_skip!();
    let su = setup(&url, "s_m_rls").await;
    let (ga, gb) = (Uuid::from_u128(10), Uuid::from_u128(20));
    let (ma, mb) = (mid(11), mid(21));
    seed_group(&su, ga).await;
    seed_group(&su, gb).await;
    seed_member_pii(
        &su,
        ga,
        ma.as_uuid(),
        &["rider"],
        phone_lookup("+15550000001").as_bytes().to_vec(),
        enc("Aaa", 2),
        enc("+15550000001", 1),
        enc("1 A", 3),
        1000,
    )
    .await;
    seed_member_pii(
        &su,
        gb,
        mb.as_uuid(),
        &["rider"],
        phone_lookup("+15550000002").as_bytes().to_vec(),
        enc("Bbb", 2),
        enc("+15550000002", 1),
        enc("2 B", 3),
        1000,
    )
    .await;

    let mut store_a = app_member_store(&url, "s_m_rls", ga).await;
    let listed = store_a.list_members().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].member_id, ma, "A lists only its own member");

    let audit = AuditEntry {
        timestamp: UnixSeconds::new(NOW),
        admin_id: mid(7),
        member_id: mb,
        fields: vec![AuditField::Name],
        request_id: "x".into(),
    };
    assert!(
        store_a
            .read_member_detail_audited(mb, audit)
            .await
            .unwrap()
            .is_none(),
        "A cannot detail-read a B member (RLS isolation)"
    );
    // …and no audit row was written for the cross-tenant attempt (RLS hid the member → not found).
    assert_eq!(
        audit_rows(&su, mb.as_uuid()).await,
        0,
        "a hidden member's read writes no audit"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// AC16 host precursor, randomized: across random two-group member configurations, a store scoped
    /// to Group A lists exactly A's members (never B's) and cannot detail-read a B member. One schema is
    /// reused; each case wipes it (DELETE FROM groups cascades all member data) and reseeds two groups
    /// with fixed ids — so cases are self-contained and collision-free.
    #[test]
    fn prop_rls_isolates_random_two_group_configs(seed in any::<u64>(), na in 1usize..=3, nb in 1usize..=3) {
        let Some(url) = db_url() else { return Ok(()); };
        // A dedicated runtime per process (built lazily once via OnceLock would be ideal, but a
        // per-case current-thread runtime is simple and the case count is small).
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let su = setup(&url, "s_m_prop").await; // DROP+CREATE+migrate is idempotent per case
            let (ga, gb) = (Uuid::from_u128(0xA), Uuid::from_u128(0xB));
            seed_group(&su, ga).await;
            seed_group(&su, gb).await;
            for i in 0..na {
                let raw = format!("+1555{:07}", seed.wrapping_add(i as u64) % 9_999_999);
                seed_member_pii(&su, ga, Uuid::from_u128(0xA00 + i as u128), &["rider"],
                    phone_lookup(&raw).as_bytes().to_vec(), enc("A", (i as u8)+1), enc("pa", (i as u8)+50), enc("ad", (i as u8)+100), 1000).await;
            }
            for i in 0..nb {
                let raw = format!("+1666{:07}", seed.wrapping_add(i as u64 + 500) % 9_999_999);
                seed_member_pii(&su, gb, Uuid::from_u128(0xB00 + i as u128), &["driver"],
                    phone_lookup(&raw).as_bytes().to_vec(), enc("B", (i as u8)+1), enc("pb", (i as u8)+50), enc("bd", (i as u8)+100), 1000).await;
            }

            let mut store_a = app_member_store(&url, "s_m_prop", ga).await;
            let listed = store_a.list_members().await.unwrap();
            assert_eq!(listed.len(), na, "A sees exactly its own {na} members, never B's");

            let b0 = MemberId::from_uuid(Uuid::from_u128(0xB00));
            let audit = AuditEntry { timestamp: UnixSeconds::new(NOW), admin_id: mid(7), member_id: b0, fields: vec![AuditField::Name], request_id: "p".into() };
            assert!(store_a.read_member_detail_audited(b0, audit).await.unwrap().is_none(), "A cannot read a B member");
        });
    }
}
