//! `boundless-server-core` — server-side core logic: Worker entry points and the
//! `GroupHub` Durable Object's auth methods (docs/architecture.md §1, §5).
//!
//! Holds the `/api/auth/*` and `/api/dev/*` handler logic, the per-Group rate-limit
//! counter + device-token invalidation (I4), and the below-min-version / notification-
//! declined non-PII admin-alert paths. All logging goes through the PII-free
//! `emit()` path (P2/I10) — never raw `tracing::*`.
//!
//! This is the reusable core library; the deployable workers-rs crate lives at the
//! top-level `server/` (`boundless-worker`) and depends on this.
//!
//! Scaffolded by spec 001 task **T01**; endpoints land in **T07/T08**.
