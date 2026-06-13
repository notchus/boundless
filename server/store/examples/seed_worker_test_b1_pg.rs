//! Seed the worker miniflare test DB with the **Option B1 admin-WebAuthn fixtures** (spec 009 T04,
//! ADR-0027) so `server/test/admin-webauthn{,-cross-tenant}.spec.ts` can drive the real
//! `/api/admin/webauthn/*` endpoints. Run by `scripts/setup-worker-test-db.sh` (AFTER the migrations,
//! the Group-A bootstrap, and the Group-B cross-tenant member), as the superuser (→ bypasses RLS for
//! the seed). Runs against a fresh schema (the setup DROPs + re-migrates each invocation).
//!
//! An **example**, never compiled into the lib or the deployed Worker. It computes the invitation
//! token hashes **in the core** (`admin_invitation_token_hash`, P4 — not a hand-rolled SQL/openssl
//! blob) under the Worker's test `HMAC_KEY`, and drives the proven `create_pending_admin_with_invitation`
//! / `insert_credential` store methods (the same seam the operator seed, T10, will use).
//!
//! Seeds (all token labels + the credential id are shared VERBATIM with the test specs):
//! - **Group A** (the Worker's single-install tenant): one pending admin with a live invitation for the
//!   read-only resolve round-trip (`boundless-test-invite-resolve`), plus a POOL of pending admins each
//!   with a live invitation (`boundless-test-invite-register-{0..N}`) the register-complete round-trip
//!   consumes one of. The pool exists because `register-complete` is single-use (consumes its invite),
//!   so the test claims the first still-live one — keeping the suite green across N re-runs WITHOUT a
//!   re-`setup-worker-test-db.sh` (the harness's "robust to accumulated state" rule). Distinct admins
//!   because `admin_invitations_one_live_per_admin` permits at most one live invite per admin.
//! - **Group B** (cross-tenant): one pending admin + a live invitation
//!   (`boundless-test-invite-xtenant`) + one active credential (`xtenant-b1-credential`), seeded with
//!   the CORRECT token hash / credential id so that — absent RLS — the Group-A Worker WOULD resolve
//!   them. A 404 from the Group-A-scoped Worker therefore proves RLS isolation, not a mere mismatch
//!   (the non-vacuous form, AC14 — the same philosophy as the cross-tenant member's real `updated_at`).
//!
//! Env (set by the setup script; `WORKER_TEST_HMAC_KEY_HEX` MUST match `server/vitest.config.ts`'s
//! `HMAC_KEY` binding — the Worker resolves tokens under that exact key):
//! - `WORKER_TEST_SUPERUSER_URL` — the DB to seed.
//! - `WORKER_TEST_GROUP_ID` — Group A (the Worker's tenant).
//! - `WORKER_TEST_XTENANT_GROUP_ID` — Group B (the cross-tenant group; its `groups` row already seeded).
//! - `WORKER_TEST_HMAC_KEY_HEX` — the 64-char-hex per-instance HMAC key.

use boundless_auth::UnixSeconds;
use boundless_crypto::{admin_invitation_token_hash, HmacKey};
use boundless_domain::AdminInvitationToken;
use boundless_server_core::{AdminProvisioningStore, AdminWebAuthnStore, NewAdminCredential};
use boundless_server_store::PgAuthStore;
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

// Token labels + the Group-B credential id — SHARED VERBATIM with the test specs (the test presents
// these to the Worker). Keep in lock-step with `server/test/admin-webauthn*.spec.ts`.
const TOKEN_RESOLVE: &str = "boundless-test-invite-resolve";
/// The register-complete pool: `boundless-test-invite-register-{0..REGISTER_POOL}`. The test claims
/// the first still-live one, so the suite survives this many re-runs without a re-seed.
const TOKEN_REGISTER_PREFIX: &str = "boundless-test-invite-register-";
const REGISTER_POOL: usize = 24;
const TOKEN_XTENANT: &str = "boundless-test-invite-xtenant";
const XTENANT_CREDENTIAL_ID: &[u8] = b"xtenant-b1-credential";

/// A far-future TTL (epoch seconds, ~year 2100) so the seeded invitations are always "live" for the
/// edge-TS `evaluateInvite` verdict regardless of when the test runs.
const FAR_FUTURE: i64 = 4_102_444_800;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let url = env("WORKER_TEST_SUPERUSER_URL");
    let group_a: Uuid = env("WORKER_TEST_GROUP_ID")
        .parse()
        .expect("WORKER_TEST_GROUP_ID must be a uuid");
    let group_b: Uuid = env("WORKER_TEST_XTENANT_GROUP_ID")
        .parse()
        .expect("WORKER_TEST_XTENANT_GROUP_ID must be a uuid");
    let key = HmacKey::from_bytes(hex32(&env("WORKER_TEST_HMAC_KEY_HEX")));

    // A connection per group (PgAuthStore owns its Client + carries a fixed group_id).
    let mut store_a = PgAuthStore::new(connect(&url).await, group_a);
    let mut store_b = PgAuthStore::new(connect(&url).await, group_b);

    // Group A — two live invites (distinct pending admins). The token hashes are computed in the core
    // under the Worker's HMAC key, so the Worker's `resolve_invitation_by_token` matches them.
    store_a
        .create_pending_admin_with_invitation(hash(&key, TOKEN_RESOLVE), UnixSeconds::new(FAR_FUTURE))
        .await
        .expect("seed Group-A resolve invitation");
    for i in 0..REGISTER_POOL {
        store_a
            .create_pending_admin_with_invitation(
                hash(&key, &format!("{TOKEN_REGISTER_PREFIX}{i}")),
                UnixSeconds::new(FAR_FUTURE),
            )
            .await
            .expect("seed Group-A register invitation");
    }

    // Group B — a live invite + an active credential, with the CORRECT hash / id (so the Group-A
    // Worker's 404 is a genuine RLS signal, AC14). The pending admin owns the credential.
    let admin_b = store_b
        .create_pending_admin_with_invitation(hash(&key, TOKEN_XTENANT), UnixSeconds::new(FAR_FUTURE))
        .await
        .expect("seed Group-B invitation");
    store_b
        .insert_credential(
            admin_b,
            NewAdminCredential {
                credential_id: XTENANT_CREDENTIAL_ID.to_vec(),
                public_key: b"xtenant-b1-pubkey".to_vec(),
                sign_count: 0,
                transports: None,
                aaguid: None,
            },
        )
        .await
        .expect("seed Group-B credential");

    // Success is signalled by exit 0 (any failure above panics via `.expect`, which the caller's
    // `set -e` catches). No stdout/stderr — the pre-commit lint forbids console-print macros in
    // non-test Rust, and an example is not `cfg(test)`.
}

/// The at-rest hash of a labelled token under the Worker's HMAC key (what `admin_invitations` stores
/// and the Worker matches — computed in the core, P4).
fn hash(key: &HmacKey, label: &str) -> boundless_crypto::AdminInvitationTokenHash {
    admin_invitation_token_hash(key, &AdminInvitationToken::new(label))
}

/// Connect to the local test DB (NoTls — no Hyperdrive here; the seed talks to Postgres directly), and
/// detach the connection driver.
async fn connect(url: &str) -> Client {
    let (client, conn) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("connect to the worker test DB");
    tokio::spawn(async move {
        let _ = conn.await;
    });
    client
}

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| panic!("{k} must be set"))
}

/// Decode a 64-char hex string into 32 bytes (the HMAC key).
fn hex32(s: &str) -> [u8; 32] {
    let s = s.trim();
    assert_eq!(s.len(), 64, "HMAC key hex must be exactly 64 chars (32 bytes)");
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("HMAC key must be valid hex");
    }
    out
}
