//! Seed the worker miniflare test DB with a **bootstrapped Group** — a `groups` row + a KEK-wrapped
//! `delegated_keys` row — so the spec-008 T09 admin-issuance miniflare tests can encrypt/decrypt PII
//! (issuance fails closed without a per-Group key, AC12). Run by `scripts/setup-worker-test-db.sh`
//! after the migrations, as the superuser (→ bypasses RLS for the seed). Idempotent.
//!
//! This is an **example**, never compiled into the lib or the deployed Worker — it keeps the
//! Group-key wrap in Rust (P4) instead of baking an opaque ciphertext blob into SQL. The plaintext
//! Group key never leaves this process (only the KEK-wrapped blob is written).
//!
//! Env (set by the setup script; the `KEK` must match `server/vitest.config.ts`'s `KEK` binding):
//! - `WORKER_TEST_SUPERUSER_URL` — the DB to seed.
//! - `WORKER_TEST_GROUP_ID` — the single-install tenant (the Worker's `GROUP_ID` binding).
//! - `WORKER_TEST_KEK_HEX` — the 64-char-hex KEK the Worker will unwrap the Group key with.

use boundless_crypto::{wrap_group_key, GroupKey, Kek, Nonce};
use tokio_postgres::NoTls;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let url = env("WORKER_TEST_SUPERUSER_URL");
    let group_id: uuid::Uuid = env("WORKER_TEST_GROUP_ID")
        .parse()
        .expect("WORKER_TEST_GROUP_ID must be a uuid");
    let kek = Kek::from_bytes(hex32(&env("WORKER_TEST_KEK_HEX")));

    // A fixed Group key, wrapped under the test KEK with a fixed nonce → the `delegated_keys.wrapped_key`
    // blob. ANY valid key works (issuance just needs to encrypt then decrypt consistently); fixed bytes
    // keep the seed deterministic. The plaintext key is dropped (zeroized) at the end of this scope.
    let wrapped = wrap_group_key(
        &GroupKey::from_bytes([0x55; 32]),
        &kek,
        &Nonce::from_bytes([0x11; 24]),
    );

    let (client, conn) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("connect to the worker test DB");
    tokio::spawn(async move {
        let _ = conn.await;
    });

    client
        .execute(
            "INSERT INTO groups (id, name) VALUES ($1, 'Worker Test Group') \
             ON CONFLICT (id) DO NOTHING",
            &[&group_id],
        )
        .await
        .expect("seed groups row");
    client
        .execute(
            "INSERT INTO delegated_keys (group_id, wrapped_key) VALUES ($1, $2) \
             ON CONFLICT (group_id) DO UPDATE SET wrapped_key = EXCLUDED.wrapped_key",
            &[&group_id, &wrapped],
        )
        .await
        .expect("seed delegated_keys row");

    // Success is signalled by exit 0: the caller `scripts/setup-worker-test-db.sh` echoes the
    // confirmation, and any failure above panics via `.expect`, which the script's `set -e` catches.
    // Deliberately no stdout/stderr here (the pre-commit lint forbids console-print macros in non-test
    // Rust, and an example is not `cfg(test)`).
}

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| panic!("{k} must be set"))
}

/// Decode a 64-char hex string into 32 bytes (the KEK).
fn hex32(s: &str) -> [u8; 32] {
    let s = s.trim();
    assert_eq!(s.len(), 64, "KEK hex must be exactly 64 chars (32 bytes)");
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("KEK must be valid hex");
    }
    out
}
