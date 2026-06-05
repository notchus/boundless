//! W2 — the least-privilege boot guard (`ensure_least_privilege`, sec-audit's highest-impact item).
//!
//! Proves both legs against real Postgres: a superuser connection (the Neon-default `postgres` role
//! footgun) is **rejected** (fail-closed), and the non-superuser / non-`BYPASSRLS` `boundless_app`
//! role — the role the Worker must connect as — is **accepted**. Self-skips without `DATABASE_URL`.
//!
//! This proves the guard's *decision*; for *why* a superuser is dangerous (it bypasses RLS →
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
