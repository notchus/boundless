//! Shared real-Postgres test harness for `boundless-server-store` (spec 001 T07-shell).
//!
//! Used by both the store-level suite (`integration.rs`, slice A) and the orchestration-level suite
//! (`service_pg.rs`, slice B — the async-port bridge). It connects to a live Postgres, provisions a
//! **non-superuser** `boundless_app` role (so RLS actually applies), and gives each test a
//! uniquely-named schema (dropped + recreated + migrated fresh) so the suite is parallel-safe.
//!
//! **Self-skipping:** [`db_url`] returns `None` (and `url_or_skip!` returns early with a notice)
//! unless `DATABASE_URL` (or `BOUNDLESS_TEST_PG`) points at a Postgres a superuser can reach. CI
//! provides a `postgres:16` service (`.github/workflows/ci.yml` → `server-store`). Locally:
//! `docker run -e POSTGRES_PASSWORD=postgres -p 55432:5432 postgres:16` then
//! `DATABASE_URL=postgres://postgres:postgres@localhost:55432/boundless_test cargo test -p
//! boundless-server-store`.
//!
//! `#![allow(dead_code)]`: this module is compiled into every test binary and not every binary uses
//! every helper — the standard `tests/common` pattern (it is *in* tests, so the forbidden-patterns
//! "no allow(dead_code) outside tests" does not apply).
#![allow(dead_code)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use boundless_crypto::{
    onboarding_code_hash, phone_lookup_hash, recovery_code_hash, refresh_token_hash, CodeHash,
    HmacKey, RefreshTokenHash,
};
use boundless_domain::{MemberId, OnboardingCode, RecoveryCode, RefreshToken};
use boundless_server_core::normalize_phone;
use boundless_server_store::PgAuthStore;
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;

/// The connection target for the integration suite, or `None` to skip.
pub fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("BOUNDLESS_TEST_PG").ok())
}

/// Resolve the DB URL or print a skip notice and return early from the calling test.
#[macro_export]
macro_rules! url_or_skip {
    () => {
        match $crate::common::db_url() {
            Some(u) => u,
            None => {
                eprintln!(
                    "ℹ skipping boundless-server-store integration tests \
                     (set DATABASE_URL=postgres://postgres:postgres@localhost:55432/boundless_test)"
                );
                return;
            }
        }
    };
}

/// The fixed per-instance HMAC secret (production reads this from Secrets Store).
pub fn key() -> HmacKey {
    HmacKey::from_bytes([0x42; 32])
}

pub fn phone_hash(raw: &str) -> Vec<u8> {
    phone_lookup_hash(&key(), &normalize_phone(raw).expect("valid E.164"))
        .as_bytes()
        .to_vec()
}
pub fn onb_hash(raw: &str) -> Vec<u8> {
    onboarding_code_hash(&key(), &OnboardingCode::new(raw))
        .as_bytes()
        .to_vec()
}
pub fn rec_hash_bytes(raw: &str) -> Vec<u8> {
    recovery_code_hash(&key(), &RecoveryCode::new(raw))
        .as_bytes()
        .to_vec()
}
pub fn rec_hash(raw: &str) -> CodeHash {
    recovery_code_hash(&key(), &RecoveryCode::new(raw))
}
pub fn refresh_hash(raw: &str) -> RefreshTokenHash {
    refresh_token_hash(&key(), &RefreshToken::new(raw))
}
pub fn pg_time(secs: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs as u64)
}

/// Connect and detach the connection driver task (lives until the client drops).
pub async fn connect(url: &str) -> Client {
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .expect("connect to test Postgres");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    client
}

/// Ensure the non-superuser app role exists (idempotent; safe under parallel tests).
///
/// Attempts `CREATE ROLE` and swallows the "already exists" outcomes — both `duplicate_object`
/// (the role existed at CREATE time) **and** `unique_violation` (two concurrent CREATEs racing on
/// `pg_authid`, which the many parallel `setup()` calls do hit). An `IF NOT EXISTS` pre-check would
/// be a TOCTOU and is deliberately omitted.
pub async fn ensure_role(c: &Client) {
    c.batch_execute(
        "DO $$ BEGIN \
           CREATE ROLE boundless_app NOLOGIN NOSUPERUSER NOBYPASSRLS; \
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; END $$;",
    )
    .await
    .expect("ensure boundless_app role");
}

/// Drop + recreate `schema`, apply migrations 0001-0008 into it, grant the app role.
/// Returns a superuser client whose `search_path` is the fresh schema (for seeding + assertions).
pub async fn setup(url: &str, schema: &str) -> Client {
    let su = connect(url).await;
    ensure_role(&su).await;
    su.batch_execute(&format!(
        "DROP SCHEMA IF EXISTS {schema} CASCADE; CREATE SCHEMA {schema}; SET search_path = {schema};"
    ))
    .await
    .expect("fresh schema");

    let mig_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../migrations");
    let mut files: Vec<_> = std::fs::read_dir(mig_dir)
        .expect("read migrations dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".up.sql"))
        })
        .collect();
    files.sort();
    assert_eq!(files.len(), 8, "expected 8 up migrations");
    for f in &files {
        let sql = std::fs::read_to_string(f).expect("read migration");
        su.batch_execute(&sql)
            .await
            .unwrap_or_else(|e| panic!("apply {}: {e}", f.display()));
    }
    su.batch_execute(&format!(
        "GRANT USAGE ON SCHEMA {schema} TO boundless_app; \
         GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA {schema} TO boundless_app;"
    ))
    .await
    .expect("grant app role");
    su
}

/// A fresh adapter connection scoped to `schema` + `group`, connected as the non-superuser role.
pub async fn app_store(url: &str, schema: &str, group: Uuid) -> PgAuthStore {
    let c = app_client(url, schema).await;
    PgAuthStore::new(c, group)
}

/// A raw non-superuser client scoped to `schema` (for the service wrapper / fail-closed probes).
pub async fn app_client(url: &str, schema: &str) -> Client {
    let c = connect(url).await;
    c.batch_execute(&format!(
        "SET search_path = {schema}; SET ROLE boundless_app;"
    ))
    .await
    .expect("app session");
    c
}

// --- seeding (run as superuser → bypasses RLS) ---

pub async fn seed_group(c: &Client, g: Uuid) {
    c.execute(
        "INSERT INTO groups (id, name) VALUES ($1, 'Test Group')",
        &[&g],
    )
    .await
    .expect("seed group");
}
pub async fn seed_member(c: &Client, g: Uuid, m: Uuid, roles: &[&str], phone: Option<Vec<u8>>) {
    let roles: Vec<String> = roles.iter().map(|s| s.to_string()).collect();
    c.execute(
        "INSERT INTO members (id, group_id, roles, phone_lookup_hash) \
         VALUES ($1, $2, $3::text[]::member_role[], $4)",
        &[&m, &g, &roles, &phone],
    )
    .await
    .expect("seed member");
}
pub async fn seed_onboarding(
    c: &Client,
    g: Uuid,
    m: Uuid,
    code_hash: Vec<u8>,
    expires: i64,
    max: i32,
) {
    c.execute(
        "INSERT INTO onboarding_codes (id, group_id, member_id, code_hash, expires_at, max_attempts) \
         VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)",
        &[&g, &m, &code_hash, &pg_time(expires), &max],
    )
    .await
    .expect("seed onboarding code");
}
pub async fn seed_recovery(c: &Client, g: Uuid, m: Uuid, code_hash: Vec<u8>) {
    c.execute(
        "INSERT INTO recovery_codes (id, group_id, member_id, code_hash) \
         VALUES (gen_random_uuid(), $1, $2, $3)",
        &[&g, &m, &code_hash],
    )
    .await
    .expect("seed recovery code");
}

// --- assertion helpers (superuser, bypasses RLS → sees the whole schema) ---

pub async fn count(c: &Client, sql: &str, p: Uuid) -> i64 {
    c.query_one(sql, &[&p]).await.expect("count").get(0)
}
pub async fn live_sessions(c: &Client, family: Uuid) -> i64 {
    count(
        c,
        "SELECT count(*) FROM sessions WHERE family_id=$1 AND rotated_at IS NULL AND revoked_at IS NULL",
        family,
    )
    .await
}
pub async fn revoked_rows(c: &Client, family: Uuid) -> i64 {
    count(
        c,
        "SELECT count(*) FROM sessions WHERE family_id=$1 AND revoked_at IS NOT NULL",
        family,
    )
    .await
}
pub async fn live_recovery(c: &Client, member: Uuid) -> i64 {
    count(
        c,
        "SELECT count(*) FROM recovery_codes WHERE member_id=$1 AND consumed_at IS NULL AND superseded_at IS NULL",
        member,
    )
    .await
}

// --- T08: admin-invitation helpers (superuser, bypasses RLS) ---

/// Count an admin's **live** (un-consumed) invitations — the one-live invariant (AC16).
pub async fn live_invitations(c: &Client, admin: Uuid) -> i64 {
    count(
        c,
        "SELECT count(*) FROM admin_invitations WHERE admin_id=$1 AND consumed_at IS NULL",
        admin,
    )
    .await
}

/// Count all of an admin's invitations (live + superseded) — to prove a re-issue adds a row.
pub async fn total_invitations(c: &Client, admin: Uuid) -> i64 {
    count(
        c,
        "SELECT count(*) FROM admin_invitations WHERE admin_id=$1",
        admin,
    )
    .await
}

/// Whether a row is a **pending Admin**: holds the `admin` role and has **no phone** (Admins
/// authenticate via WebAuthn, not phone — spec §B / migration 0002).
pub async fn pending_admin_exists(c: &Client, admin: Uuid) -> bool {
    c.query_opt(
        "SELECT 1 FROM members \
         WHERE id=$1 AND roles @> ARRAY['admin']::member_role[] AND phone_lookup_hash IS NULL",
        &[&admin],
    )
    .await
    .expect("query pending admin")
    .is_some()
}

/// The `token_hash` bytes of an admin's live invitation, if any (for a constant-time verify).
pub async fn live_invitation_hash(c: &Client, admin: Uuid) -> Option<Vec<u8>> {
    c.query_opt(
        "SELECT token_hash FROM admin_invitations WHERE admin_id=$1 AND consumed_at IS NULL",
        &[&admin],
    )
    .await
    .expect("query live invitation hash")
    .map(|r| r.get::<_, Vec<u8>>("token_hash"))
}

/// The `expires_at` of an admin's live invitation as whole epoch seconds, if any (AC16 server TTL).
pub async fn live_invitation_expiry_secs(c: &Client, admin: Uuid) -> Option<i64> {
    c.query_opt(
        "SELECT EXTRACT(EPOCH FROM expires_at)::bigint AS s \
         FROM admin_invitations WHERE admin_id=$1 AND consumed_at IS NULL",
        &[&admin],
    )
    .await
    .expect("query live invitation expiry")
    .map(|r| r.get::<_, i64>("s"))
}

pub const G: u128 = 1;
pub fn mid(n: u128) -> MemberId {
    MemberId::from_uuid(Uuid::from_u128(n))
}
