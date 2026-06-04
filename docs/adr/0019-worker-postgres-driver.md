# ADR-0019: Worker → Postgres via `tokio-postgres` over a Hyperdrive Socket (not `sqlx`)

- **Status:** Accepted
- **Date:** 2026-06-05
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0001, ADR-0014, ADR-0016; I3/I4; O-series; spec 001 tasks T06, T07 (T07-shell slice A)
- **Supersedes (in part):** the tentative "`sqlx` … Postgres via Hyperdrive" line in `docs/stack-matrix.md` and the T06 deferral "adopt `sqlx::migrate!` + pin `sqlx`" in `DEFERRED.md`.

## Context

The Boundless server tier is a **Cloudflare Worker written in Rust** (workers-rs), compiled to
`wasm32-unknown-unknown`, reaching Neon Postgres **through Hyperdrive** (`docs/architecture.md`,
ADR-0014). T06 shipped the schema; T07-core shipped the pure auth orchestration engine behind
sync in-memory `AuthStore` ports, *modelling* the atomic/RLS/TOCTOU contracts. The deployable
shell (T07-shell) needs a **real** Postgres-backed store.

`docs/stack-matrix.md` and the T06 register named **`sqlx`** as the Postgres path, with a TODO to
adopt `sqlx::migrate!`. Before building the adapter we verified the runtime constraints
(2026-06-05, via `docs-researcher` against Cloudflare's docs + crates.io/docs.rs):

- **`sqlx` cannot run in the Cloudflare Workers wasm runtime.** It depends on a system async I/O
  stack (`tokio` net, OS sockets) that does not exist on `wasm32-unknown-unknown` in workerd.
  Cloudflare's "supported Rust crates" guidance does not list `sqlx`.
- **The supported path is `tokio-postgres` driven over a `worker::Socket`** obtained from
  `env.hyperdrive(...).connect()`. Cloudflare documents exactly this ("tokio-postgres can be
  compiled to Wasm; it must be configured to use a Socket from workers-rs").
- A real sharp edge exists: Hyperdrive's connection pooler dislikes `tokio-postgres`'s *unnamed*
  prepared statements; the pooler-safe path is `query_raw` / simple-protocol handling. (Native
  tests against a direct Postgres are unaffected; this only bites through the pooler.)

A second forcing constraint: the `AuthStore` port trait (`core/server/src/ports.rs`) is **sync**
(`&mut self`, infallible) — correct for the in-memory stub, but a database backend is inherently
**async and fallible**, and you cannot block-on-async in wasm. So the real store cannot literally
`impl AuthStore`; the trait must eventually become async.

## Decision

**The Boundless Worker reaches Postgres with `tokio-postgres` over a Hyperdrive `worker::Socket`.
`sqlx` is dropped from the production data path.** The store adapter (`boundless-server-store`,
`server/store/`) is a **native crate** implementing the SQL + transaction logic; it is exercised
by real-Postgres integration tests and will later be driven by the Worker over the Socket.

Sequenced to respect "code without a passing test is incomplete":

1. **Slice A (this ADR / T07-shell slice A) — native, fully tested.** `boundless-server-store`
   provides `PgAuthStore` with `async fn`s mirroring the `AuthStore` contract 1:1, using
   `tokio-postgres` (pinned `0.7`, lock = `0.7.17`; feature `with-uuid-1`). It is tested against a
   real `postgres:16` (Docker locally / CI service), proving the contracts the in-memory stub only
   modelled: single-consume under concurrency, **atomic supersede-then-insert**, **rotate-vs-replay
   TOCTOU → revoked family** (a concurrency bug *was* found and fixed — see below), family-kill
   persistence, and **RLS tenant isolation** (`SET`-via-`set_config(...,true)` per request txn,
   non-superuser role, fail-closed when unset).
2. **Slice B (deferred → `DEFERRED.md`, T07-shell-B) — wasm wiring.** The workers-rs runtime
   (`#[event]`/Router/`GroupHub` DO), `hyperdrive.connect()` → `Socket` transport, the
   `tokio-postgres` **wasm feature flags + pooler-safe `query_raw`**, access-token signing, APNs/FCM,
   `wrangler.toml`, and a miniflare/workerd integration harness — plus the **async-port bridge**
   (making `core/server`'s ports `async` and wiring `PgAuthStore` into `AuthService`).

### Rotate-vs-replay serialization (a real bug this slice caught)

The naive `rotate_session` (supersede the current row, insert the new current) and `revoke_family`
(stamp `revoked_at` on the family's live rows) **race** under READ COMMITTED: a rotate that commits
a *new* current row while a concurrent revoke's `UPDATE` is in flight leaves that row outside the
revoke's snapshot — a live credential **surviving** a family-kill (ADR-0016 D2 / carry-forward (b)).
The `concurrent_rotate_and_replay_resolves_to_revoked` test reproduced it. Fix: both `rotate_session`
and `revoke_family` take a **`pg_advisory_xact_lock` on the family** at transaction start, so one
fully precedes the other (rotate-first ⇒ revoke also revokes the new row; revoke-first ⇒ rotate finds
no live current and aborts). Both acquire the family lock before any row lock ⇒ no deadlock.

### Migrations

The Worker cannot run `sqlx::migrate!` (sqlx is out). Migrations stay plain reversible
`NNNN_*.{up,down}.sql` (T06), applied **out of band** (CI `psql` / `scripts/test-migrations.sh`; the
store tests apply them via `batch_execute`). No `sqlx` dependency anywhere.

## Considered alternatives

### Option B — `sqlx` (the prior assumption)

**Rejected:** does not compile/run on the Workers wasm runtime. Viable only for a *separate native*
service, which we do not run (the matching/auth tier is the Worker, ADR-0014). Keeping it would mean
two data layers (native sqlx + wasm something-else) — needless divergence (violates the spirit of P4).

### Option C — a separate always-on native Rust service (axum + sqlx) beside the Worker

**Rejected for now:** reintroduces an always-on box to deploy/scale/monitor, against the
edge-only operational model (ADR-0014, P11 free-tier path). `tokio-postgres`-over-Socket keeps
everything in the Worker. (Revisit only if Workers limits ever force it — would need its own ADR.)

### Option D — Neon's serverless HTTP driver

**Rejected:** ties the data path to a Neon-specific HTTP endpoint rather than the documented
Hyperdrive Postgres-wire path, and bypasses Hyperdrive pooling/caching (ADR-0014). Less portable
across the "any Postgres" goal.

## Consequences

### Positive

- The documented, Cloudflare-blessed Worker→Postgres path; everything stays at the edge.
- The risk-bearing SQL/transaction logic is **proven against real Postgres now** (slice A), not
  deferred wholesale to a hard-to-test wasm integration — and it already caught and fixed a TOCTOU.
- Same `tokio-postgres` crate native (tests) and wasm (Worker), so the query/transaction code is
  shared; only the connection transport differs (TcpStream vs Hyperdrive Socket).

### Negative / costs

- **The `AuthStore` port must become async** before the Worker can use `PgAuthStore` (the
  async-port bridge). Deferred to T07-shell-B so this slice does not async-refactor the committed,
  48-test T07-core for a consumer that does not exist yet.
- The Hyperdrive-pooler **unnamed-prepared-statement** edge must be handled in slice B (use
  `query_raw` / a pooler-safe pattern); native tests don't exercise it, so it is an explicit B risk.
- `tokio-postgres` + `tokio` are added to the *server* dependency tree (native). MIT OR Apache-2.0,
  no trackers (allow-list clean).

### Neutral / follow-ups

- `docs/stack-matrix.md`: the Edge/Server `sqlx` row is replaced by `tokio-postgres`; `tokio` (test)
  added; versions pinned from the lock.
- `DEFERRED.md`: new "Server / store (T07-shell slice A)" register; the T06 "adopt sqlx" note is
  superseded; the device-token store methods (`register_device` needs at-rest token encryption,
  spec 007/008) remain deferred.

## References

- `docs/architecture.md` (KV/Hyperdrive); ADR-0014 (server-driven config); ADR-0016 D2 (sessions/refresh)
- `core/server/src/ports.rs` (`AuthStore` contract); `server/store/` (`PgAuthStore` + integration tests)
- [Cloudflare — Supported Rust crates](https://developers.cloudflare.com/workers/languages/rust/crates/) (sqlx absent; tokio-postgres over a Socket)
- [Cloudflare Hyperdrive — Connect to Postgres](https://developers.cloudflare.com/hyperdrive/examples/connect-to-postgres/) · [worker `Socket`](https://docs.rs/worker/latest/worker/struct.Socket.html)
- [`tokio-postgres` 0.7.17](https://docs.rs/tokio-postgres/0.7.17/tokio_postgres/) — verified 2026-06-05
