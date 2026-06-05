//! Real-Postgres tests for the T08 [`AdminProvisioningStore`] impl on [`PgAuthStore`]: developer
//! Admin creation + invitation mint/re-issue (AC1/AC16, I11, ADR-0015). These prove the DB-level
//! contracts the in-memory stub can only model — atomic supersede-then-insert against the
//! `admin_invitations_one_live_per_admin` partial-unique index, server-time TTL persistence, the
//! role/tenant gates (RLS), and that concurrent re-issues never leave two live invitations.
//!
//! Self-skips without `DATABASE_URL` (see `common`); connects as the non-superuser `boundless_app`
//! role so RLS actually applies.

mod common;

use boundless_auth::UnixSeconds;
use boundless_crypto::{
    admin_invitation_token_hash, admin_invitation_token_matches, AdminInvitationTokenHash,
};
use boundless_domain::AdminInvitationToken;
use boundless_server_core::AdminProvisioningStore;
use uuid::Uuid;

use common::*;

/// A test invitation token from a label.
fn token(label: &str) -> AdminInvitationToken {
    AdminInvitationToken::new(label)
}

/// The at-rest hash of a labelled token under the harness key (what the store persists).
fn token_hash(label: &str) -> AdminInvitationTokenHash {
    admin_invitation_token_hash(&key(), &token(label))
}

/// Rebuild a stored `token_hash` (`bytea`) into the typed hash for a constant-time verify.
fn hash_from_db(bytes: Vec<u8>) -> AdminInvitationTokenHash {
    AdminInvitationTokenHash::from_bytes(bytes.try_into().expect("token_hash is 32 bytes"))
}

#[tokio::test]
async fn create_mints_pending_admin_and_one_live_invitation() {
    let url = url_or_skip!();
    let su = setup(&url, "admin_create").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let mut store = app_store(&url, "admin_create", g).await;
    let admin = store
        .create_pending_admin_with_invitation(token_hash("inv-001"), UnixSeconds::new(100_000))
        .await
        .unwrap();
    let aid = admin.as_uuid();

    // A pending Admin: admin role, NO phone (Admins authenticate via WebAuthn).
    assert!(pending_admin_exists(&su, aid).await);
    // Exactly one live invitation; server-time TTL persisted as the timestamptz we passed.
    assert_eq!(live_invitations(&su, aid).await, 1);
    assert_eq!(total_invitations(&su, aid).await, 1);
    assert_eq!(live_invitation_expiry_secs(&su, aid).await, Some(100_000));

    // The stored token_hash verifies against the token, constant-time (the deferred consume path,
    // T09, relies on exactly this); a different token does not.
    let stored = hash_from_db(live_invitation_hash(&su, aid).await.unwrap());
    assert!(admin_invitation_token_matches(
        &key(),
        &token("inv-001"),
        &stored
    ));
    assert!(!admin_invitation_token_matches(
        &key(),
        &token("inv-WRONG"),
        &stored
    ));
}

#[tokio::test]
async fn reissue_supersedes_prior_and_keeps_exactly_one_live() {
    let url = url_or_skip!();
    let su = setup(&url, "admin_reissue").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let mut store = app_store(&url, "admin_reissue", g).await;

    let admin = store
        .create_pending_admin_with_invitation(token_hash("inv-A"), UnixSeconds::new(100_000))
        .await
        .unwrap();
    let aid = admin.as_uuid();

    let existed = store
        .reissue_admin_invitation(
            admin,
            token_hash("inv-B"),
            UnixSeconds::new(200_000),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap();
    assert!(existed);

    // Atomic supersede-then-insert: still one live, but a second (consumed) row now exists.
    assert_eq!(live_invitations(&su, aid).await, 1);
    assert_eq!(total_invitations(&su, aid).await, 2);
    assert_eq!(live_invitation_expiry_secs(&su, aid).await, Some(200_000));

    let live = hash_from_db(live_invitation_hash(&su, aid).await.unwrap());
    assert!(
        admin_invitation_token_matches(&key(), &token("inv-B"), &live),
        "the re-issued token is the live one"
    );
    assert!(
        !admin_invitation_token_matches(&key(), &token("inv-A"), &live),
        "the superseded token is no longer live (single-use preserved)"
    );
}

#[tokio::test]
async fn reissue_unknown_admin_is_a_noop() {
    let url = url_or_skip!();
    let su = setup(&url, "admin_unknown").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;
    let mut store = app_store(&url, "admin_unknown", g).await;

    let phantom = mid(0xABCD);
    let existed = store
        .reissue_admin_invitation(
            phantom,
            token_hash("inv-X"),
            UnixSeconds::new(200_000),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap();
    assert!(!existed, "re-inviting an unknown admin is a no-op");
    assert_eq!(total_invitations(&su, phantom.as_uuid()).await, 0);
}

#[tokio::test]
async fn reissue_non_admin_member_is_a_noop() {
    let url = url_or_skip!();
    let su = setup(&url, "admin_rolegate").await;
    let g = Uuid::from_u128(G);
    let m = mid(1);
    seed_group(&su, g).await;
    // A plain Rider member (with a phone) is NOT a pending Admin.
    seed_member(
        &su,
        g,
        m.as_uuid(),
        &["rider"],
        Some(phone_hash("+15550000001")),
    )
    .await;
    let mut store = app_store(&url, "admin_rolegate", g).await;

    let existed = store
        .reissue_admin_invitation(
            m,
            token_hash("inv-X"),
            UnixSeconds::new(200_000),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap();
    assert!(
        !existed,
        "a Rider is not a pending Admin → no invitation minted"
    );
    assert_eq!(total_invitations(&su, m.as_uuid()).await, 0);
}

#[tokio::test]
async fn admin_provisioning_is_tenant_isolated() {
    let url = url_or_skip!();
    let su = setup(&url, "admin_tenant").await;
    let g1 = Uuid::from_u128(1);
    let g2 = Uuid::from_u128(2);
    seed_group(&su, g1).await;
    seed_group(&su, g2).await;

    let mut store1 = app_store(&url, "admin_tenant", g1).await;
    let admin = store1
        .create_pending_admin_with_invitation(token_hash("inv-1"), UnixSeconds::new(100_000))
        .await
        .unwrap();

    // A store scoped to g2 cannot see g1's admin (RLS) → re-issue is a no-op and g1's invite stands.
    let mut store2 = app_store(&url, "admin_tenant", g2).await;
    let existed = store2
        .reissue_admin_invitation(
            admin,
            token_hash("inv-2"),
            UnixSeconds::new(200_000),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap();
    assert!(
        !existed,
        "another tenant must not be able to touch this admin (RLS)"
    );

    assert_eq!(live_invitations(&su, admin.as_uuid()).await, 1);
    let live = hash_from_db(live_invitation_hash(&su, admin.as_uuid()).await.unwrap());
    assert!(
        admin_invitation_token_matches(&key(), &token("inv-1"), &live),
        "g1's original invitation is untouched"
    );
}

#[tokio::test]
async fn concurrent_reissue_keeps_exactly_one_live() {
    let url = url_or_skip!();
    let su = setup(&url, "admin_concurrent").await;
    let g = Uuid::from_u128(G);
    seed_group(&su, g).await;

    let admin = {
        let mut store = app_store(&url, "admin_concurrent", g).await;
        store
            .create_pending_admin_with_invitation(token_hash("inv-0"), UnixSeconds::new(100_000))
            .await
            .unwrap()
    };

    // Two concurrent re-issues, distinct tokens. The admin-scoped advisory xact lock serializes
    // them, so the second sees the first's committed live row and supersedes it: both succeed, and
    // the one-live invariant is never violated (no unique-violation error).
    let mut a = app_store(&url, "admin_concurrent", g).await;
    let mut b = app_store(&url, "admin_concurrent", g).await;
    let ha = tokio::spawn(async move {
        a.reissue_admin_invitation(
            admin,
            token_hash("inv-A"),
            UnixSeconds::new(200_000),
            UnixSeconds::new(1_000),
        )
        .await
        .unwrap()
    });
    let hb = tokio::spawn(async move {
        b.reissue_admin_invitation(
            admin,
            token_hash("inv-B"),
            UnixSeconds::new(200_000),
            UnixSeconds::new(1_001),
        )
        .await
        .unwrap()
    });
    let (ra, rb) = tokio::join!(ha, hb);
    assert!(
        ra.unwrap() && rb.unwrap(),
        "both re-issues succeed (serialized)"
    );

    assert_eq!(
        live_invitations(&su, admin.as_uuid()).await,
        1,
        "exactly one live invitation survives concurrent re-issue"
    );
    assert_eq!(
        total_invitations(&su, admin.as_uuid()).await,
        3,
        "create + two re-issues = 3 rows (2 consumed, 1 live)"
    );
}
