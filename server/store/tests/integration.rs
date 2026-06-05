//! Real-Postgres integration tests for [`PgAuthStore`] at the **store level** (spec 001 T07-shell
//! slice A).
//!
//! These prove against a live database the contracts the in-memory `AuthStore` stub could only
//! *model*: single-consume under concurrency, atomic supersede-then-insert, rotate-vs-replay
//! TOCTOU, family-kill persistence, and RLS tenant isolation (the T05/T06 carry-forwards). The
//! orchestration-level proof (these methods driven through `AuthService`) is `service_pg.rs`.
//!
//! Harness (connect / role / schema / seeding / assertions + `url_or_skip!`) lives in `common`.

mod common;

use boundless_auth::{RefreshPresentation, SessionFamilyStatus, UnixSeconds};
use boundless_crypto::PhoneLookupHash;
use boundless_domain::{MemberId, RefreshToken};
use boundless_server_core::AuthStore;
use boundless_server_store::StoreError;
use uuid::Uuid;

use common::*;

#[tokio::test]
async fn rls_isolates_reads_by_tenant() {
    let url = url_or_skip!();
    let su = setup(&url, "s_isolate").await;
    let (ga, gb) = (Uuid::from_u128(10), Uuid::from_u128(20));
    let (ma, mb) = (Uuid::from_u128(11), Uuid::from_u128(21));
    seed_group(&su, ga).await;
    seed_group(&su, gb).await;
    seed_member(&su, ga, ma, &["rider"], Some(phone_hash("+15550000001"))).await;
    seed_member(&su, gb, mb, &["rider"], Some(phone_hash("+15550000002"))).await;

    // Scoped to tenant A: A's member is visible by phone; B's member is NOT (RLS isolates reads).
    let mut store_a = app_store(&url, "s_isolate", ga).await;
    let phash_a = PhoneLookupHash::from_bytes(phone_hash("+15550000001").try_into().unwrap());
    let phash_b = PhoneLookupHash::from_bytes(phone_hash("+15550000002").try_into().unwrap());
    let found_a = store_a.find_member_by_phone(&phash_a).await.unwrap();
    assert_eq!(found_a.map(|r| r.member_id), Some(MemberId::from_uuid(ma)));
    assert!(
        store_a
            .find_member_by_phone(&phash_b)
            .await
            .unwrap()
            .is_none(),
        "tenant A must not see tenant B's member (RLS isolation)"
    );
}

#[tokio::test]
async fn rls_unset_tenant_denies_fail_closed() {
    let url = url_or_skip!();
    let su = setup(&url, "s_failclosed").await;
    let g = Uuid::from_u128(30);
    seed_group(&su, g).await;
    seed_member(&su, g, Uuid::from_u128(31), &["rider"], None).await;

    // A connection AS the app role that NEVER sets app.current_group_id sees zero rows — the
    // fail-closed resolver (T06 R1). We bypass the adapter (which always sets the GUC) to prove the
    // schema denies by default.
    let c = app_client(&url, "s_failclosed").await;
    let n: i64 = c
        .query_one("SELECT count(*) FROM members", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 0, "unset tenant must see zero rows (fail-closed RLS)");
}

#[tokio::test]
async fn find_member_returns_roles_and_none() {
    let url = url_or_skip!();
    let su = setup(&url, "s_roles").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    seed_member(
        &su,
        g,
        mid(40).as_uuid(),
        &["driver", "rider"],
        Some(phone_hash("+15550000040")),
    )
    .await;

    let mut store = app_store(&url, "s_roles", g).await;
    let found = store
        .find_member_by_phone(&PhoneLookupHash::from_bytes(
            phone_hash("+15550000040").try_into().unwrap(),
        ))
        .await
        .unwrap()
        .expect("member found");
    assert_eq!(found.member_id, mid(40));
    assert!(found.is_driver(), "roles round-trip through text[]");
    assert_eq!(found.roles.len(), 2);

    // An unknown phone yields None (not an error, no existence leak at this layer).
    let none = store
        .find_member_by_phone(&PhoneLookupHash::from_bytes(
            phone_hash("+15559999999").try_into().unwrap(),
        ))
        .await
        .unwrap();
    assert!(none.is_none());
}

#[tokio::test]
async fn onboarding_single_use() {
    let url = url_or_skip!();
    let su = setup(&url, "s_onb").await;
    let g = Uuid::from_u128(G);
    let m = mid(50);
    seed_group(&su, g).await;
    seed_member(&su, g, m.as_uuid(), &["rider"], None).await;
    seed_onboarding(&su, g, m.as_uuid(), onb_hash("CODE-A"), 100_000, 5).await;

    let mut store = app_store(&url, "s_onb", g).await;
    assert!(store.load_live_onboarding(m).await.unwrap().is_some());
    assert!(
        store
            .consume_onboarding_if_live(m, UnixSeconds::new(1_000))
            .await
            .unwrap(),
        "first consume succeeds"
    );
    assert!(
        store.load_live_onboarding(m).await.unwrap().is_none(),
        "consumed code is no longer live"
    );
    assert!(
        !store
            .consume_onboarding_if_live(m, UnixSeconds::new(1_001))
            .await
            .unwrap(),
        "second consume loses the race (single-use)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bind_single_consume_under_concurrency() {
    let url = url_or_skip!();
    let su = setup(&url, "s_onbrace").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    // Repeat to shake out interleavings: each iteration, one live code raced by two connections.
    for i in 0..5u128 {
        let m = mid(60 + i);
        seed_member(&su, g, m.as_uuid(), &["rider"], None).await;
        seed_onboarding(
            &su,
            g,
            m.as_uuid(),
            onb_hash(&format!("RACE-{i}")),
            100_000,
            5,
        )
        .await;

        let mut a = app_store(&url, "s_onbrace", g).await;
        let mut b = app_store(&url, "s_onbrace", g).await;
        let ha = tokio::spawn(async move {
            a.consume_onboarding_if_live(m, UnixSeconds::new(1_000))
                .await
                .unwrap()
        });
        let hb = tokio::spawn(async move {
            b.consume_onboarding_if_live(m, UnixSeconds::new(1_000))
                .await
                .unwrap()
        });
        let (ra, rb) = tokio::join!(ha, hb);
        let trues = [ra.unwrap(), rb.unwrap()].iter().filter(|x| **x).count();
        assert_eq!(
            trues, 1,
            "exactly one concurrent consume may win (iter {i})"
        );
    }
}

#[tokio::test]
async fn classify_reports_superseded_after_rotations() {
    let url = url_or_skip!();
    let su = setup(&url, "s_classify").await;
    let g = Uuid::from_u128(G);
    let m = mid(70);
    seed_group(&su, g).await;
    seed_member(&su, g, m.as_uuid(), &["rider"], None).await;

    let mut store = app_store(&url, "s_classify", g).await;
    let fam = store
        .create_session_family(
            m,
            refresh_hash("R0"),
            UnixSeconds::new(1_900),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap()
        .family_id;
    // Rotate the live credential three times: R0 -> R1 -> R2 -> R3.
    for (cur, next, t) in [
        ("R0", "R1", 1_100),
        ("R1", "R2", 1_200),
        ("R2", "R3", 1_300),
    ] {
        let _ = cur; // (the live one is implied by lineage)
        store
            .rotate_session(
                fam,
                refresh_hash(next),
                UnixSeconds::new(t + 900),
                UnixSeconds::new(t),
            )
            .await
            .expect("rotate live credential");
    }

    // The original credential, rotated 3 times ago, classifies Superseded (NOT Unknown) — so a
    // replay of it would kill the family (carry-forward (c)). The latest is Current; a never-seen
    // credential is Unknown.
    let c0 = store
        .classify_refresh(&RefreshToken::new("R0"), &key())
        .await
        .unwrap();
    assert_eq!(c0.presentation, RefreshPresentation::Superseded);
    assert_eq!(c0.family.unwrap().status, SessionFamilyStatus::Active);

    let c3 = store
        .classify_refresh(&RefreshToken::new("R3"), &key())
        .await
        .unwrap();
    assert_eq!(c3.presentation, RefreshPresentation::Current);

    let cx = store
        .classify_refresh(&RefreshToken::new("NOPE"), &key())
        .await
        .unwrap();
    assert_eq!(cx.presentation, RefreshPresentation::Unknown);
    assert!(cx.family.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_rotate_and_replay_resolves_to_revoked() {
    let url = url_or_skip!();
    let su = setup(&url, "s_replay").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    // Many iterations + a barrier that releases rotate and revoke together maximize the window
    // where an INSERT (new current) and the revoke's UPDATE are simultaneously in flight — the
    // exact schedule that, WITHOUT the family advisory lock, lets a freshly-inserted current row
    // escape the revoke's READ COMMITTED snapshot (this test fails on the un-locked impl, so it is
    // a real regression guard, not just a witness).
    for i in 0..20u128 {
        let m = mid(80 + i);
        seed_member(&su, g, m.as_uuid(), &["rider"], None).await;

        // Build {current=R1, superseded=R0}.
        let mut setup_store = app_store(&url, "s_replay", g).await;
        let fam = setup_store
            .create_session_family(
                m,
                refresh_hash(&format!("R0-{i}")),
                UnixSeconds::new(1_900),
                UnixSeconds::new(1_000),
            )
            .await
            .unwrap()
            .family_id;
        setup_store
            .rotate_session(
                fam,
                refresh_hash(&format!("R1-{i}")),
                UnixSeconds::new(2_000),
                UnixSeconds::new(1_100),
            )
            .await
            .unwrap();
        drop(setup_store);

        // Concurrently: A legitimately rotates the current credential; B replays the rotated-away
        // one (→ classify Superseded → revoke the family). For EVERY interleaving the family must
        // end revoked with zero live credentials (the crown-jewel TOCTOU proof, carry-forward (b)).
        let mut a = app_store(&url, "s_replay", g).await;
        let mut b = app_store(&url, "s_replay", g).await;
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let (ba, bb) = (barrier.clone(), barrier.clone());
        let r2 = format!("R2-{i}");
        let r0 = format!("R0-{i}");
        let ha = tokio::spawn(async move {
            ba.wait().await; // enter the contended window together with the revoke
                             // Always ATTEMPT the rotate (no pre-classify gate) — the rotate's own "supersede must
                             // hit exactly 1 row or abort" handles the lost race. Ok or NoLiveCurrentToRotate only.
            match a
                .rotate_session(
                    fam,
                    refresh_hash(&r2),
                    UnixSeconds::new(2_900),
                    UnixSeconds::new(2_000),
                )
                .await
            {
                Ok(_) | Err(StoreError::NoLiveCurrentToRotate) => {}
                Err(e) => panic!("unexpected rotate error: {e}"),
            }
        });
        let hb = tokio::spawn(async move {
            // The replay is detected as Superseded BEFORE the contended window, then revoked inside.
            let c = b
                .classify_refresh(&RefreshToken::new(&r0), &key())
                .await
                .unwrap();
            assert_eq!(c.presentation, RefreshPresentation::Superseded);
            bb.wait().await;
            b.revoke_family(fam, UnixSeconds::new(2_001)).await.unwrap();
        });
        let _ = tokio::join!(ha, hb);

        let famu = fam.as_uuid();
        assert_eq!(
            live_sessions(&su, famu).await,
            0,
            "no live credential may survive the replay-kill (iter {i})"
        );
        assert!(
            revoked_rows(&su, famu).await >= 1,
            "the family must be persisted revoked (iter {i})"
        );
    }
}

#[tokio::test]
async fn family_kill_persists_and_blocks_legit_current() {
    let url = url_or_skip!();
    let su = setup(&url, "s_revoke").await;
    let g = Uuid::from_u128(G);
    let m = mid(90);
    seed_group(&su, g).await;
    seed_member(&su, g, m.as_uuid(), &["rider"], None).await;

    let mut store = app_store(&url, "s_revoke", g).await;
    let fam = store
        .create_session_family(
            m,
            refresh_hash("LIVE"),
            UnixSeconds::new(1_900),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap()
        .family_id;
    store
        .revoke_family(fam, UnixSeconds::new(1_500))
        .await
        .unwrap();

    // revoked_at is persisted; the (still-current-lineage) credential now reports Revoked…
    assert!(revoked_rows(&su, fam.as_uuid()).await >= 1);
    let c = store
        .classify_refresh(&RefreshToken::new("LIVE"), &key())
        .await
        .unwrap();
    assert_eq!(c.presentation, RefreshPresentation::Current);
    assert_eq!(c.family.unwrap().status, SessionFamilyStatus::Revoked);

    // …and a rotate of a revoked family is refused (no live current to supersede).
    match store
        .rotate_session(
            fam,
            refresh_hash("R2"),
            UnixSeconds::new(2_900),
            UnixSeconds::new(2_000),
        )
        .await
    {
        Err(StoreError::NoLiveCurrentToRotate) => {}
        other => panic!("revoked family must refuse rotation, got {other:?}"),
    }
}

#[tokio::test]
async fn recovery_consume_and_rotate_single_use() {
    let url = url_or_skip!();
    let su = setup(&url, "s_rec").await;
    let g = Uuid::from_u128(G);
    let (driver, norec) = (mid(100), mid(101));
    seed_group(&su, g).await;
    seed_member(&su, g, driver.as_uuid(), &["driver"], None).await;
    seed_member(&su, g, norec.as_uuid(), &["driver"], None).await;
    seed_recovery(&su, g, driver.as_uuid(), rec_hash_bytes("C0")).await;

    let mut store = app_store(&url, "s_rec", g).await;
    assert!(store.load_live_recovery(driver).await.unwrap().is_some());

    // Consume + rotate to a fresh code; exactly one live code remains throughout.
    assert!(store
        .consume_and_rotate_recovery(driver, rec_hash("C1"), UnixSeconds::new(1_000))
        .await
        .unwrap());
    assert_eq!(live_recovery(&su, driver.as_uuid()).await, 1);
    // The fresh one is now live → another rotate also succeeds.
    assert!(store
        .consume_and_rotate_recovery(driver, rec_hash("C2"), UnixSeconds::new(1_001))
        .await
        .unwrap());
    assert_eq!(live_recovery(&su, driver.as_uuid()).await, 1);

    // A member with no live recovery code cannot consume one.
    assert!(!store
        .consume_and_rotate_recovery(norec, rec_hash("X"), UnixSeconds::new(1_002))
        .await
        .unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn recovery_consume_concurrency() {
    let url = url_or_skip!();
    let su = setup(&url, "s_recrace").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    for i in 0..5u128 {
        let m = mid(110 + i);
        seed_member(&su, g, m.as_uuid(), &["driver"], None).await;
        seed_recovery(&su, g, m.as_uuid(), rec_hash_bytes(&format!("C0-{i}"))).await;

        let mut a = app_store(&url, "s_recrace", g).await;
        let mut b = app_store(&url, "s_recrace", g).await;
        let (ca, cb) = (rec_hash(&format!("CA-{i}")), rec_hash(&format!("CB-{i}")));
        let ha = tokio::spawn(async move {
            a.consume_and_rotate_recovery(m, ca, UnixSeconds::new(1_000))
                .await
                .unwrap()
        });
        let hb = tokio::spawn(async move {
            b.consume_and_rotate_recovery(m, cb, UnixSeconds::new(1_000))
                .await
                .unwrap()
        });
        let (ra, rb) = tokio::join!(ha, hb);
        let trues = [ra.unwrap(), rb.unwrap()].iter().filter(|x| **x).count();
        assert_eq!(
            trues, 1,
            "exactly one concurrent recovery consume may win (iter {i})"
        );
        assert_eq!(
            live_recovery(&su, m.as_uuid()).await,
            1,
            "one live code remains (iter {i})"
        );
    }
}

#[tokio::test]
async fn revoked_family_marks_superseded_credentials_revoked() {
    // revoke_family stamps revoked_at on EVERY row of the family (current + rotated-away), so a
    // *superseded* credential in a revoked family must also classify Revoked. This pins the
    // emergent equivalence the row-derived family status depends on (reviewer M1) — without it, a
    // future selective-revoke could silently diverge from MemStore and the in-memory tests wouldn't
    // catch it.
    let url = url_or_skip!();
    let su = setup(&url, "s_revsup").await;
    let g = Uuid::from_u128(G);
    let m = mid(120);
    seed_group(&su, g).await;
    seed_member(&su, g, m.as_uuid(), &["rider"], None).await;

    let mut store = app_store(&url, "s_revsup", g).await;
    let fam = store
        .create_session_family(
            m,
            refresh_hash("R0"),
            UnixSeconds::new(1_900),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap()
        .family_id;
    store
        .rotate_session(
            fam,
            refresh_hash("R1"),
            UnixSeconds::new(2_000),
            UnixSeconds::new(1_100),
        )
        .await
        .unwrap();
    store
        .revoke_family(fam, UnixSeconds::new(1_500))
        .await
        .unwrap();

    // The rotated-away credential is still Superseded by lineage, but its family is now Revoked.
    let c0 = store
        .classify_refresh(&RefreshToken::new("R0"), &key())
        .await
        .unwrap();
    assert_eq!(c0.presentation, RefreshPresentation::Superseded);
    assert_eq!(c0.family.unwrap().status, SessionFamilyStatus::Revoked);
    // …and the current credential is Current + Revoked.
    let c1 = store
        .classify_refresh(&RefreshToken::new("R1"), &key())
        .await
        .unwrap();
    assert_eq!(c1.presentation, RefreshPresentation::Current);
    assert_eq!(c1.family.unwrap().status, SessionFamilyStatus::Revoked);
}

#[tokio::test]
async fn onboarding_consume_ignores_ttl_which_is_the_cores_job() {
    // The store's `*_if_live` keys off consumed/superseded ONLY — never `expires_at`. TTL is gated
    // by core::auth against server time (the OnboardingCodeRow carries expires_at for exactly that).
    // So an expired-but-unconsumed code must still load as live AND consume — the store must not
    // silently treat expiry as "not live" (which would diverge from the contract). (reviewer M3)
    let url = url_or_skip!();
    let su = setup(&url, "s_onbttl").await;
    let g = Uuid::from_u128(G);
    let m = mid(130);
    seed_group(&su, g).await;
    seed_member(&su, g, m.as_uuid(), &["rider"], None).await;
    seed_onboarding(&su, g, m.as_uuid(), onb_hash("OLD"), 1, 5).await; // expires_at = epoch + 1s (long past)

    let mut store = app_store(&url, "s_onbttl", g).await;
    let row = store
        .load_live_onboarding(m)
        .await
        .unwrap()
        .expect("an expired-but-unconsumed code is still 'live' to the store");
    assert_eq!(
        row.expires_at.as_secs(),
        1,
        "the TTL is surfaced for the core to gate, not applied here"
    );
    assert!(
        store
            .consume_onboarding_if_live(m, UnixSeconds::new(999_999))
            .await
            .unwrap(),
        "the store consumes regardless of TTL"
    );
}

#[tokio::test]
async fn onboarding_superseded_is_not_live() {
    // A superseded code (the regenerate-invalidates-prior lineage, AC17) is excluded from the live
    // set: load_live → None, consume_if_live → false. (reviewer M3)
    let url = url_or_skip!();
    let su = setup(&url, "s_onbsup").await;
    let g = Uuid::from_u128(G);
    let m = mid(140);
    seed_group(&su, g).await;
    seed_member(&su, g, m.as_uuid(), &["rider"], None).await;
    seed_onboarding(&su, g, m.as_uuid(), onb_hash("SUP"), 100_000, 5).await;
    // Mark it superseded directly (the regenerate path itself is issuance's job, not this adapter's).
    let mu = m.as_uuid();
    su.execute(
        "UPDATE onboarding_codes SET superseded_at = now() WHERE member_id = $1",
        &[&mu],
    )
    .await
    .expect("supersede the seeded code");

    let mut store = app_store(&url, "s_onbsup", g).await;
    assert!(store.load_live_onboarding(m).await.unwrap().is_none());
    assert!(!store
        .consume_onboarding_if_live(m, UnixSeconds::new(1_000))
        .await
        .unwrap());
}
