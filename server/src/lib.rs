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
//! Scaffolded by spec 001 task **T01**. The deployable Worker **skeleton** — the `#[event(fetch)]`
//! entry, the `worker::Router`, the [`GroupHub`](runtime::GroupHub) Durable Object, and the KV +
//! Queues bindings — lands in **T07-shell-B slice 1**. It is `#[cfg(target_arch = "wasm32")]`: a
//! native `cargo build`/`test` compiles only the (cfg-empty) lib + the `[[test]] compat` harness,
//! so the store member + compat suite stay green; the wasm build (`worker-build`) compiles the full
//! runtime. The runtime composes the core's [`AuthService`] over a **scaffold in-memory store**
//! ([`runtime::ScaffoldStore`]) — a clearly-labelled `wrangler dev`/test stand-in to be replaced by
//! the Postgres-over-Hyperdrive `PgAuthStore` in a later slice (see `DEFERRED.md` → T07-shell-B).
//!
//! [`AuthService`]: boundless_server_core::AuthService

// The deployable Worker runtime is wasm-only (the `worker` crate does not build natively). Gating it
// keeps the native compat harness (`tests/compat/`) + the `store` member buildable with plain cargo.
#[cfg(target_arch = "wasm32")]
mod runtime;
