//! Bootstrap a REAL Boundless Group on a deployed/owner Postgres — the PRODUCTION analog of
//! `seed_worker_test_pg.rs`. Run ONCE per Group by the operator, after the migrations are applied and
//! before any member is issued or the AC16 cross-tenant check is run
//! (docs/runbooks/deploy-worker.md → "bootstrap the Group"). Driven by `scripts/bootstrap-group.sh`.
//!
//! It mints a fresh **random** per-Group secretbox key + a fresh **random** nonce, wraps the key under
//! the operator's **real** KEK (`boundless_crypto::wrap_group_key`, P4 — the wrap is in Rust, never an
//! opaque SQL blob), and writes the KEK-wrapped blob to `delegated_keys.wrapped_key`. The plaintext key
//! never leaves this process (only the wrapped blob is persisted — P2/I1). This is the spec-008 operator
//! provisioning that `server/src/runtime/pg.rs` deliberately keeps OFF the Worker request path
//! (`unreachable!("Group bootstrap is operator-run provisioning, never a Worker path")`).
//!
//! UNLIKE the test seed (which `DO UPDATE`s a fixed key), this inserts **`ON CONFLICT DO NOTHING`** — it
//! **NEVER overwrites** an existing key, because overwriting would orphan every row of PII already
//! encrypted under it (the old key would be gone, the data undecryptable). A re-run is therefore safe.
//!
//! Exit code (no stdout — the lint forbids print macros in non-test Rust; the wrapper maps the code):
//! - `0` — a new key was minted and stored (Group newly bootstrapped).
//! - `3` — a key already existed; left untouched (idempotent no-op).
//! - non-zero/panic — a bad KEK / URL / connection (the `.expect`s below).
//!
//! Env (set by `scripts/bootstrap-group.sh`; the `KEK` MUST match the Worker's `KEK` secret):
//! - `BOOTSTRAP_OWNER_URL` — the DB OWNER DIRECT URL (neondb_owner; BYPASSRLS lands the insert under FORCE RLS). NOT `boundless_app`.
//! - `BOOTSTRAP_GROUP_ID` — the Group uuid (this install's `GROUP_ID` Worker binding).
//! - `BOOTSTRAP_KEK_HEX` — the 64-char-hex KEK the Worker unwraps the Group key with.
//! - `BOOTSTRAP_GROUP_NAME` — the Group's human-readable name (glossary: groups have a `name`).

use boundless_crypto::{unwrap_group_key, wrap_group_key, GroupKey, Kek, Nonce};
use std::process::ExitCode;
use tokio_postgres::NoTls;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let url = env("BOOTSTRAP_OWNER_URL");
    let group_id: uuid::Uuid = env("BOOTSTRAP_GROUP_ID")
        .parse()
        .expect("BOOTSTRAP_GROUP_ID must be a uuid");
    let group_name = env("BOOTSTRAP_GROUP_NAME");
    let kek = Kek::from_bytes(hex32(&env("BOOTSTRAP_KEK_HEX")));

    // A fresh RANDOM Group key + RANDOM nonce from the OS CSPRNG (getrandom on the host). A getrandom
    // failure must PANIC (fail-closed) rather than yield zero bytes — zero key/nonce = catastrophic
    // reuse (R1). The plaintext key is dropped (zeroized — `GroupKey` impls Drop) at the end of `main`;
    // only the KEK-wrapped blob is ever written.
    let mut key_bytes = [0u8; 32];
    getrandom::fill(&mut key_bytes).expect("OS CSPRNG (getrandom) must not fail");
    let mut nonce_bytes = [0u8; 24];
    getrandom::fill(&mut nonce_bytes).expect("OS CSPRNG (getrandom) must not fail");
    let wrapped = wrap_group_key(
        &GroupKey::from_bytes(key_bytes),
        &kek,
        &Nonce::from_bytes(nonce_bytes),
    );

    // Self-check: the blob we are about to persist MUST unwrap with this KEK. Catches a wrong/garbled KEK
    // here (a loud panic) instead of letting issuance later fail closed with an opaque key error.
    unwrap_group_key(&wrapped, &kek).expect("wrapped key must round-trip with the provided KEK");

    let (client, conn) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("connect to the owner DB (use the DIRECT/unpooled endpoint, as neondb_owner)");
    tokio::spawn(async move {
        let _ = conn.await;
    });

    // The groups row must exist first (delegated_keys.group_id REFERENCES groups). Idempotent.
    client
        .execute(
            "INSERT INTO groups (id, name) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING",
            &[&group_id, &group_name],
        )
        .await
        .expect("insert groups row");

    // Insert the wrapped key ONLY if absent — NEVER overwrite (DO NOTHING, not DO UPDATE). The number of
    // rows affected tells us which happened: 1 ⇒ newly bootstrapped; 0 ⇒ a key already existed (untouched).
    let inserted = client
        .execute(
            "INSERT INTO delegated_keys (group_id, wrapped_key) VALUES ($1, $2) \
             ON CONFLICT (group_id) DO NOTHING",
            &[&group_id, &wrapped],
        )
        .await
        .expect("insert delegated_keys row");

    if inserted == 1 {
        ExitCode::SUCCESS // 0 — newly bootstrapped
    } else {
        ExitCode::from(3) // a key already existed; left untouched
    }
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
