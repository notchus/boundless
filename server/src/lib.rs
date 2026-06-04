//! `boundless-worker` — the Cloudflare Workers (workers-rs) deployment crate for the
//! Boundless edge tier (docs/architecture.md §5).
//!
//! Hosts the `/api/auth/*`, `/api/dev/*`, and `/api/admin/auth/*` routes and the
//! `GroupHub` Durable Object, delegating logic to `boundless-server-core`. Numbered SQL
//! migrations live in `server/migrations/` (T06); the N-2 backward-compat replay harness
//! (O1) lives in `server/tests/compat/` (T08).
//!
//! Scaffolded by spec 001 task **T01**; the `#[event]` entry point + routes land in
//! **T07/T08**.
