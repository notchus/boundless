//! Real-Postgres integration tests for [`PgMemberStore`]'s [`DelegatedKeyStore`] (spec 008 **T07**):
//! the per-Group wrapped key (ADR-0025) is returned **wrapped** (never plaintext), and a Group with
//! no `delegated_keys` row returns `None` (issuance then fails closed, AC12).
//!
//! Harness lives in `common`.

mod common;

use boundless_crypto::{wrap_group_key, Kek, Nonce, KEY_LEN, MAC_LEN, NONCE_LEN};
use boundless_server_core::DelegatedKeyStore;
use uuid::Uuid;

use common::*;

#[tokio::test]
async fn pg_delegated_key_store_persists_only_wrapped() {
    let url = url_or_skip!();
    let su = setup(&url, "s_dk").await;
    let (ga, gb) = (Uuid::from_u128(10), Uuid::from_u128(20));
    seed_group(&su, ga).await;
    seed_group(&su, gb).await;

    // Wrap the Group key under a KEK (the shape ADR-0025 stores: nonce ‖ ciphertext ‖ MAC over the key).
    let kek = Kek::from_bytes([0x55; KEY_LEN]);
    let wrapped = wrap_group_key(&group_key(), &kek, &Nonce::from_bytes([7u8; NONCE_LEN]));
    assert_eq!(
        wrapped.len(),
        NONCE_LEN + MAC_LEN + KEY_LEN,
        "a wrapped-key blob, not a bare key"
    );
    seed_delegated_key(&su, ga, &wrapped).await;

    // Group A (bootstrapped) returns exactly the wrapped bytes — never a plaintext key.
    let mut store_a = app_member_store(&url, "s_dk", ga).await;
    let got = store_a
        .current_wrapped_key()
        .await
        .unwrap()
        .expect("a wrapped key");
    assert_eq!(
        got, wrapped,
        "the store returns the stored wrapped blob verbatim"
    );

    // Group B (never bootstrapped) returns None → issuance fails closed (AC12).
    let mut store_b = app_member_store(&url, "s_dk", gb).await;
    assert!(
        store_b.current_wrapped_key().await.unwrap().is_none(),
        "no key row → None (fail closed)"
    );

    // RLS: A's store never sees B's (absent) row and vice-versa — the wrapped key is group-scoped.
    // (Seed B now and confirm A still returns ITS key, not B's.)
    let wrapped_b = wrap_group_key(&group_key(), &kek, &Nonce::from_bytes([9u8; NONCE_LEN]));
    seed_delegated_key(&su, gb, &wrapped_b).await;
    assert_eq!(
        store_a.current_wrapped_key().await.unwrap().unwrap(),
        wrapped,
        "A still gets A's key under RLS"
    );
    assert_eq!(
        store_b.current_wrapped_key().await.unwrap().unwrap(),
        wrapped_b,
        "B gets B's key"
    );
    assert_ne!(wrapped, wrapped_b, "distinct per-Group wrapped keys");
}
