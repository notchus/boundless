//! `boundless-worker` — the Cloudflare Workers (workers-rs) deployment crate for the
//! Boundless edge tier (docs/architecture.md §5).
//!
//! Hosts the `/api/auth/*`, `/api/dev/*`, and `/api/admin/auth/*` routes and the
//! `GroupHub` Durable Object, delegating logic to `boundless-server-core` (P4 — the server
//! cannot drift from what the clients render). Numbered SQL migrations live in
//! `server/migrations/` (T06); the N-2 backward-compat replay harness (O1) lives in
//! `server/tests/compat/` (T08).
//!
//! ## Functional core, imperative shell
//!
//! Scaffolded by spec 001 task **T01**; the deployable Worker skeleton landed in **T07-shell-B slice
//! 1** and the real **PgAuthStore-over-Hyperdrive** data path in the **T07-shell-B PgAuthStore
//! slice**. The runtime is `#[cfg(target_arch = "wasm32")]`: a native `cargo build`/`test` compiles
//! only the (cfg-empty) lib + the `[[test]] compat` harness, so the `store` member + compat suite
//! stay green with plain cargo; the wasm build (`worker-build`) compiles the full runtime, which
//! composes the core's [`AuthService`] over the real [`PgAuthStore`](runtime) (P4).
//!
//! [`AuthService`]: boundless_server_core::AuthService

// The deployable Worker runtime is wasm-only (the `worker` crate does not build natively). Gating it
// keeps the native compat harness (`tests/compat/`) + the `store` member buildable with plain cargo.
//
// The old `scaffold`-feature `compile_error!` deploy guard (security-auditor F1) is RETIRED: the only
// store is now the real `PgAuthStore` over Hyperdrive, so a featureless `wrangler deploy` build is
// correct (no hardcoded key / seeded member exists to protect against). Fail-closed moved to RUNTIME
// — the W2 `ensure_least_privilege` guard refuses a superuser/BYPASSRLS DB role, and a missing
// `HMAC_KEY`/`GROUP_ID`/`HYPERDRIVE` binding errors the request (see `runtime`). DEFERRED.md → T07-shell-B.
#[cfg(target_arch = "wasm32")]
mod runtime;
