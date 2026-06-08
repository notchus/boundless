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
//
// It is ALSO gated on the non-default `scaffold` feature (security-auditor F1, T07-shell-B): the only
// store wired today is the in-memory `runtime::ScaffoldStore` — a hardcoded dev HMAC key + one seeded
// demo member that must NEVER reach production. The local/test path opts in (`worker-build --release
// --features scaffold`, server/package.json); the deploy path (`wrangler deploy` → wrangler.toml
// [build] = `worker-build --release`) stays featureless and hits the `compile_error!` below — so the
// scaffold cannot be silently deployed. The PgAuthStore-over-Hyperdrive slice deletes the scaffold and
// retires this feature (DEFERRED.md → T07-shell-B).
#[cfg(all(target_arch = "wasm32", feature = "scaffold"))]
mod runtime;

// Fail-closed deploy guard (security-auditor F1): a featureless wasm build — what `wrangler deploy`
// runs — has no store wired, so fail the build LOUDLY rather than ship an empty Worker (or, worse, the
// scaffold). Both this and `mod runtime` are wasm32-gated, so a native build (pre-push / the `server`
// CI job) never trips it. The sentinel line is asserted by scripts/check-worker-deploy-guard.sh.
#[cfg(all(target_arch = "wasm32", not(feature = "scaffold")))]
compile_error!(
    "boundless-worker has no production store yet: build with `--features scaffold` for local \
     miniflare/dev (the in-memory ScaffoldStore — a hardcoded dev HMAC key + one seeded demo member, \
     which must never be deployed). A production `wrangler deploy` must first wire PgAuthStore over \
     Hyperdrive — see DEFERRED.md → T07-shell-B (PgAuthStore slice)."
);
