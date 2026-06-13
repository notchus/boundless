//! Real-Postgres tests for the spec 009 **T02** [`AdminWebAuthnStore`] impl on [`PgAuthStore`]
//! (Option B1, ADR-0027): invite-resolve + atomic single-use consume + credential CRUD + the combined
//! `register_complete` transaction. These prove the DB-level contracts the in-memory double can only
//! *model*:
//!
//! - **AC4b** — `resolve_invitation_by_token` routes the match through the core HMAC
//!   (`admin_invitation_token_hash`), so the right token resolves and a wrong one does not.
//! - **AC4a / R15** — `consume_invitation` is single-use, and under concurrency exactly one of two
//!   racing consumes wins.
//! - **R11** — `register_complete` consumes the invite, revokes the admin's prior credentials (D4),
//!   and inserts the new one in **one transaction**; a second attempt on the consumed token writes
//!   nothing (`InviteNotConsumable`).
//! - **AC3 (store leg)** — a credential inserted via one store instance is read back by a **fresh**
//!   store instance (durable in Postgres, not in-process).
//! - **R10** — `bump_sign_count` advances only-if-strictly-greater.
//! - **AC14** — a Group-B-scoped store sees none of Group A's invites/credentials, and a cross-tenant
//!   consume/register is a no-op (RLS).
//!
//! Self-skips without `DATABASE_URL` (see `common`); connects as the non-superuser `boundless_app`
//! role so RLS actually applies.

mod common;

use boundless_auth::UnixSeconds;
use boundless_crypto::{admin_invitation_token_hash, AdminInvitationTokenHash};
use boundless_domain::{AdminInvitationToken, MemberId};
use boundless_server_core::{
    AdminProvisioningStore, AdminWebAuthnStore, NewAdminCredential, RegisterCompleteOutcome,
};
use uuid::Uuid;

use common::*;

/// A test invitation token from a label.
fn token(label: &str) -> AdminInvitationToken {
    AdminInvitationToken::new(label)
}

/// The at-rest hash of a labelled token under the harness key (what the store persists / matches).
fn token_hash(label: &str) -> AdminInvitationTokenHash {
    admin_invitation_token_hash(&key(), &token(label))
}

/// A `NewAdminCredential` from simple byte labels — opaque PII-free WebAuthn material stand-ins
/// (a real `credential_id`/`public_key` is just bytes to this layer).
fn new_cred(credential_id: &[u8], public_key: &[u8], sign_count: i64) -> NewAdminCredential {
    NewAdminCredential {
        credential_id: credential_id.to_vec(),
        public_key: public_key.to_vec(),
        sign_count,
        transports: Some(vec!["internal".to_string()]),
        aaguid: None,
    }
}

/// Seed a pending Admin (role `admin`, no PII) + its live invitation for `label`, returning the
/// admin id. Reuses the proven T08 `create_pending_admin_with_invitation` (the seam the operator
/// seed, T10, also drives).
async fn seed_pending_admin(url: &str, schema: &str, g: Uuid, label: &str) -> MemberId {
    let mut store = app_store(url, schema, g).await;
    store
        .create_pending_admin_with_invitation(token_hash(label), UnixSeconds::new(100_000))
        .await
        .unwrap()
}

#[tokio::test]
async fn pg_admin_auth_store_resolve_routes_through_core_match() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_resolve").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin = seed_pending_admin(&url, "aw_resolve", g, "inv-resolve").await;

    let mut store = app_store(&url, "aw_resolve", g).await;
    // The right token resolves to the row (core HMAC compare) — live = consumed_at None.
    let rec = store
        .resolve_invitation_by_token(&key(), &token("inv-resolve"))
        .await
        .unwrap()
        .expect("the live token resolves");
    assert_eq!(rec.admin_id, admin);
    assert_eq!(rec.group_id, g);
    assert_eq!(rec.expires_at, UnixSeconds::new(100_000));
    assert_eq!(rec.consumed_at, None);

    // A wrong token resolves to None — the core HMAC gates it, no existence oracle.
    assert!(store
        .resolve_invitation_by_token(&key(), &token("inv-WRONG"))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn pg_admin_auth_store_consume_is_single_use() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_consume").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin = seed_pending_admin(&url, "aw_consume", g, "inv-c").await;

    let mut store = app_store(&url, "aw_consume", g).await;
    assert!(
        store
            .consume_invitation(&key(), &token("inv-c"), UnixSeconds::new(1_000))
            .await
            .unwrap(),
        "first consume wins"
    );
    assert!(
        !store
            .consume_invitation(&key(), &token("inv-c"), UnixSeconds::new(2_000))
            .await
            .unwrap(),
        "second consume is a no-op (single-use)"
    );
    assert_eq!(
        live_invitations(&su, admin.as_uuid()).await,
        0,
        "no live invitation remains after consume"
    );

    // A wrong/unknown token never consumes anything.
    assert!(!store
        .consume_invitation(&key(), &token("nope"), UnixSeconds::new(3_000))
        .await
        .unwrap());
}

#[tokio::test]
async fn pg_admin_auth_store_concurrent_consume_one_wins() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_concurrent").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin = seed_pending_admin(&url, "aw_concurrent", g, "inv-cc").await;

    // Two concurrent consumes of the SAME token. The conditional `UPDATE … WHERE consumed_at IS NULL`
    // lets exactly one affect a row; the other sees it already consumed (R15 — single-use stands
    // under concurrency, the same proof shape as `concurrent_reissue_keeps_exactly_one_live`).
    let mut a = app_store(&url, "aw_concurrent", g).await;
    let mut b = app_store(&url, "aw_concurrent", g).await;
    let ha = tokio::spawn(async move {
        a.consume_invitation(&key(), &token("inv-cc"), UnixSeconds::new(1_000))
            .await
            .unwrap()
    });
    let hb = tokio::spawn(async move {
        b.consume_invitation(&key(), &token("inv-cc"), UnixSeconds::new(1_001))
            .await
            .unwrap()
    });
    let (ra, rb) = tokio::join!(ha, hb);
    assert!(
        ra.unwrap() ^ rb.unwrap(),
        "exactly one concurrent consume wins (single-use under concurrency, R15)"
    );
    assert_eq!(live_invitations(&su, admin.as_uuid()).await, 0);
}

#[tokio::test]
async fn pg_admin_auth_store_credential_persists_across_fresh_store_instance() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_persist").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin = seed_pending_admin(&url, "aw_persist", g, "inv-p").await;

    // Insert via one store instance (one connection), then drop it.
    {
        let mut store = app_store(&url, "aw_persist", g).await;
        store
            .insert_credential(admin, new_cred(b"cred-1", b"pub-1", 0))
            .await
            .unwrap();
    }

    // A FRESH store instance (new connection) over the same schema/group reads it back — the
    // credential is durable in Postgres, not in-process (AC3 store leg).
    let mut fresh = app_store(&url, "aw_persist", g).await;
    let found = fresh
        .find_active_credential(b"cred-1")
        .await
        .unwrap()
        .expect("the persisted credential is found by a fresh store instance");
    assert_eq!(found.credential_id, b"cred-1".to_vec());
    assert_eq!(found.admin_id, admin);
    assert_eq!(found.public_key, b"pub-1".to_vec());
    assert_eq!(found.sign_count, 0);
    assert_eq!(found.transports, Some(vec!["internal".to_string()]));
    assert_eq!(found.aaguid, None);
    assert_eq!(found.revoked_at, None);

    let active = fresh.list_active_credentials(admin).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].credential_id, b"cred-1".to_vec());
}

#[tokio::test]
async fn pg_admin_auth_store_bump_sign_count_only_if_greater() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_bump").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin = seed_pending_admin(&url, "aw_bump", g, "inv-b").await;

    let mut store = app_store(&url, "aw_bump", g).await;
    store
        .insert_credential(admin, new_cred(b"cred-b", b"pub-b", 5))
        .await
        .unwrap();

    store.bump_sign_count(b"cred-b", 10).await.unwrap();
    assert_eq!(
        db_sign_count(&su, b"cred-b").await,
        Some(10),
        "advances to a strictly-greater count"
    );

    store.bump_sign_count(b"cred-b", 8).await.unwrap();
    assert_eq!(
        db_sign_count(&su, b"cred-b").await,
        Some(10),
        "a lower count is ignored (clone-detection backstop, R10)"
    );

    store.bump_sign_count(b"cred-b", 10).await.unwrap();
    assert_eq!(
        db_sign_count(&su, b"cred-b").await,
        Some(10),
        "an equal count is ignored (strictly greater)"
    );
}

#[tokio::test]
async fn pg_admin_auth_store_revoke_all_for_admin() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_revoke").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin = seed_pending_admin(&url, "aw_revoke", g, "inv-rv").await;

    let mut store = app_store(&url, "aw_revoke", g).await;
    store
        .insert_credential(admin, new_cred(b"cred-1", b"pub-1", 0))
        .await
        .unwrap();
    store
        .insert_credential(admin, new_cred(b"cred-2", b"pub-2", 0))
        .await
        .unwrap();
    assert_eq!(active_credentials(&su, admin.as_uuid()).await, 2);

    store
        .revoke_all_for_admin(admin, UnixSeconds::new(9_000))
        .await
        .unwrap();
    assert_eq!(
        active_credentials(&su, admin.as_uuid()).await,
        0,
        "all active credentials revoked"
    );
    assert_eq!(
        total_credentials(&su, admin.as_uuid()).await,
        2,
        "revoke does not delete the rows"
    );
    assert!(store.find_active_credential(b"cred-1").await.unwrap().is_none());
    assert!(store.list_active_credentials(admin).await.unwrap().is_empty());

    // A revoked credential's counter never advances (bump's `revoked_at IS NULL` guard, R10 defence).
    store.bump_sign_count(b"cred-1", 99).await.unwrap();
    assert_eq!(
        db_sign_count(&su, b"cred-1").await,
        Some(0),
        "bump is a no-op on a revoked credential"
    );
}

#[tokio::test]
async fn pg_admin_auth_store_duplicate_credential_id_rejected() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_dup").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin1 = seed_pending_admin(&url, "aw_dup", g, "inv-d1").await;
    let admin2 = seed_pending_admin(&url, "aw_dup", g, "inv-d2").await;

    let mut store = app_store(&url, "aw_dup", g).await;
    store
        .insert_credential(admin1, new_cred(b"cred-dup", b"pub-a", 0))
        .await
        .unwrap();

    // The same credential_id under the SAME admin is rejected (the unique index — never a silent
    // replace; an admin holds multiple DISTINCT credentials, AC20).
    assert!(
        store
            .insert_credential(admin1, new_cred(b"cred-dup", b"pub-b", 0))
            .await
            .is_err(),
        "a duplicate credential_id is rejected, not silently replaced"
    );
    // The same credential_id under a DIFFERENT admin is also rejected (the index is global) — no
    // cross-admin credential-id hijack.
    assert!(
        store
            .insert_credential(admin2, new_cred(b"cred-dup", b"pub-c", 0))
            .await
            .is_err(),
        "a credential_id cannot be hijacked onto another admin"
    );

    // The original is untouched: admin1 still has exactly its one credential, admin2 has none.
    assert_eq!(active_credentials(&su, admin1.as_uuid()).await, 1);
    assert_eq!(active_credentials(&su, admin2.as_uuid()).await, 0);
    let found = store.find_active_credential(b"cred-dup").await.unwrap().unwrap();
    assert_eq!(found.admin_id, admin1, "the credential still belongs to admin1");
    assert_eq!(found.public_key, b"pub-a".to_vec(), "the original public key is unchanged");
}

#[tokio::test]
async fn pg_admin_auth_store_register_complete_is_atomic_and_revokes_priors() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_register").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let admin = seed_pending_admin(&url, "aw_register", g, "inv-r").await;

    let mut store = app_store(&url, "aw_register", g).await;
    // Seed a PRIOR active credential — a re-invite registration must revoke it (ADR-0016 D4).
    store
        .insert_credential(admin, new_cred(b"old-cred", b"old-pub", 3))
        .await
        .unwrap();

    // One transaction: consume the live invite, revoke priors, insert the new credential (R11).
    let outcome = store
        .register_complete(
            &key(),
            &token("inv-r"),
            new_cred(b"new-cred", b"new-pub", 0),
            UnixSeconds::new(5_000),
        )
        .await
        .unwrap();
    assert_eq!(outcome, RegisterCompleteOutcome::Completed { admin_id: admin });

    assert_eq!(
        live_invitations(&su, admin.as_uuid()).await,
        0,
        "the invitation is consumed"
    );
    assert!(
        credential_revoked(&su, b"old-cred").await,
        "the prior credential is revoked (D4)"
    );
    let active = store.list_active_credentials(admin).await.unwrap();
    assert_eq!(active.len(), 1, "exactly the new credential is active");
    assert_eq!(active[0].credential_id, b"new-cred".to_vec());

    // A second register_complete on the now-consumed token writes nothing (TOCTOU backstop).
    let again = store
        .register_complete(
            &key(),
            &token("inv-r"),
            new_cred(b"dup-cred", b"dup-pub", 0),
            UnixSeconds::new(6_000),
        )
        .await
        .unwrap();
    assert_eq!(again, RegisterCompleteOutcome::InviteNotConsumable);
    assert!(
        store.find_active_credential(b"dup-cred").await.unwrap().is_none(),
        "no credential inserted on a non-consumable invite (atomic rollback)"
    );
    assert_eq!(
        store.list_active_credentials(admin).await.unwrap().len(),
        1,
        "the registered credential is untouched"
    );
}

#[tokio::test]
async fn pg_admin_auth_store_isolates_invite_and_credentials_by_tenant() {
    let url = url_or_skip!();
    let su = setup(&url, "aw_tenant").await;
    let g1 = Uuid::from_u128(1);
    let g2 = Uuid::from_u128(2);
    seed_group(&su, g1).await;
    seed_group(&su, g2).await;

    // Group 1: a pending admin + invite + a registered credential.
    let admin1 = seed_pending_admin(&url, "aw_tenant", g1, "inv-g1").await;
    {
        let mut s1 = app_store(&url, "aw_tenant", g1).await;
        s1.insert_credential(admin1, new_cred(b"cred-g1", b"pub-g1", 0))
            .await
            .unwrap();
    }

    // A store scoped to Group 2 sees NONE of Group 1's rows (RLS, AC14).
    let mut s2 = app_store(&url, "aw_tenant", g2).await;
    assert!(
        s2.resolve_invitation_by_token(&key(), &token("inv-g1"))
            .await
            .unwrap()
            .is_none(),
        "g1's invite token is invisible to a g2-scoped store"
    );
    assert!(
        s2.find_active_credential(b"cred-g1").await.unwrap().is_none(),
        "g1's credential is invisible to a g2-scoped store"
    );
    assert!(
        s2.list_active_credentials(admin1).await.unwrap().is_empty(),
        "g1's admin's credentials are invisible to g2"
    );

    // A g2-scoped consume / register_complete on g1's token is a no-op (touches nothing in g1).
    assert!(!s2
        .consume_invitation(&key(), &token("inv-g1"), UnixSeconds::new(1_000))
        .await
        .unwrap());
    assert_eq!(
        s2.register_complete(
            &key(),
            &token("inv-g1"),
            new_cred(b"evil", b"evil", 0),
            UnixSeconds::new(2_000),
        )
        .await
        .unwrap(),
        RegisterCompleteOutcome::InviteNotConsumable
    );

    // Group 1's invite is still live and its credential still active (untouched by g2).
    assert_eq!(live_invitations(&su, admin1.as_uuid()).await, 1);
    assert_eq!(active_credentials(&su, admin1.as_uuid()).await, 1);
    assert!(!credential_revoked(&su, b"cred-g1").await);
}
