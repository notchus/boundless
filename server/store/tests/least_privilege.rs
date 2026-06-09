//! W2 — the least-privilege boot guard (`ensure_least_privilege`, sec-audit's highest-impact item).
//!
//! Proves every leg against real Postgres: a superuser connection (the Neon-default role footgun) is
//! **rejected** (fail-closed), a `REPLICATION` role (which can stream the WAL past RLS) is
//! **rejected**, and the non-superuser / non-`BYPASSRLS` / non-`REPLICATION` `boundless_app` role —
//! the role the Worker must connect as — is **accepted**. Self-skips without `DATABASE_URL`.
//!
//! This proves the guard's *decision*; for *why* such a role is dangerous (it bypasses RLS →
//! cross-tenant reads) see the tenant-isolation tests in `integration.rs`, which the guard exists to
//! make unreachable in production.

mod common;

use boundless_server_store::{ensure_least_privilege, StoreError};
use common::*;

#[tokio::test]
async fn ensure_least_privilege_rejects_superuser_accepts_app_role() {
    let url = url_or_skip!();
    let schema = "w2_least_privilege";
    // `setup` provisions the non-superuser `boundless_app` role + a fresh schema and returns a
    // SUPERUSER client (the exact role the guard must reject).
    let su = setup(&url, schema).await;

    // A superuser bypasses FORCE ROW LEVEL SECURITY → tenant isolation would silently not apply.
    // The guard must refuse (fails closed).
    match ensure_least_privilege(&su).await {
        Err(StoreError::PrivilegeTooHigh(_)) => {}
        other => panic!("a superuser connection must be rejected by the guard, got {other:?}"),
    }

    // The Worker's actual runtime role: NOSUPERUSER NOBYPASSRLS → the guard must accept it.
    let app = app_client(&url, schema).await;
    ensure_least_privilege(&app)
        .await
        .expect("boundless_app (NOSUPERUSER NOBYPASSRLS) must pass the least-privilege guard");
}

/// The third role-attribute RLS bypass: a role that is **neither** superuser **nor** `BYPASSRLS`
/// but **has `REPLICATION`** can open a replication connection and stream the WAL — every tenant's
/// rows, with RLS never consulted. So it must be rejected just like a superuser (the leg added when
/// the runtime guard's check was widened to mirror `scripts/provision-neon.sh`).
#[tokio::test]
async fn ensure_least_privilege_rejects_replication_role() {
    let url = url_or_skip!();
    // The harness `DATABASE_URL` is a superuser — mint a NOLOGIN role with REPLICATION (and
    // explicitly NOSUPERUSER NOBYPASSRLS, so the *only* attribute the guard can be tripping on is
    // REPLICATION, not a co-incidental superuser/BYPASSRLS). Idempotent like `ensure_role`.
    let su = connect(&url).await;
    su.batch_execute(
        "DO $$ BEGIN \
           CREATE ROLE boundless_repl NOLOGIN NOSUPERUSER NOBYPASSRLS REPLICATION; \
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; END $$;",
    )
    .await
    .expect("ensure boundless_repl role");

    // Assume it via `SET ROLE` (the same path `app_client` uses for `boundless_app`, so the guard
    // sees it as the effective `current_user`) and prove the widened guard refuses it.
    let repl = connect(&url).await;
    repl.batch_execute("SET ROLE boundless_repl;")
        .await
        .expect("assume the replication role");
    match ensure_least_privilege(&repl).await {
        Err(StoreError::PrivilegeTooHigh(_)) => {}
        other => panic!("a REPLICATION role must be rejected by the guard, got {other:?}"),
    }
}
