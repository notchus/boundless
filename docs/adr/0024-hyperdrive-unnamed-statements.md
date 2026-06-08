# ADR-0024: Worker→Postgres uses `tokio-postgres`'s unnamed-statement `query_typed*` family (no driver fork)

- **Status:** Accepted
- **Date:** 2026-06-08
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0014 (server-driven config / Hyperdrive), ADR-0019 (Worker→Postgres driver), ADR-0021 (access-token verify lookup); I3/I4; spec 001 T07-shell-B (PgAuthStore-over-Hyperdrive)
- **Refines:** ADR-0019 — resolves its open "unnamed-prepared-statement edge … use `query_raw` / a pooler-safe pattern" slice-B risk into a concrete contract, and **corrects** the named/unnamed polarity in ADR-0019's Context (see below).

## Context

The Boundless server tier is a **Cloudflare Worker in Rust** (workers-rs, `wasm32-unknown-unknown`)
reaching Neon Postgres **through Hyperdrive** (ADR-0014). ADR-0019 chose `tokio-postgres` over a
`worker::Socket` and dropped `sqlx`, but left **how to issue queries through the pooler** unresolved,
flagging it as an explicit slice-B risk. `DEFERRED.md` (T07-shell-B) escalated it to a build blocker:

> *"…the Hyperdrive pooler rejects tokio-postgres's default prepared statements — so this needs a
> **forked `tokio-postgres` (`unnamed-statement` branch)** or `query_with_param_types()`, plus the
> wasm32 feature flags. … Record the driver choice in an ADR before building."*

`PgAuthStore` (`server/store/`) is built, native-tested against real `postgres:16`, and implements the
async `AuthStore` contract — but it currently issues every query via `query_one` / `query_opt` /
`execute` with a `&str` (grep `server/store/src/lib.rs`; **no explicit `prepare()`**), which is the
**named-statement** path. This ADR decides what the Worker (slice B) does so the build is mechanical.

### Findings (verified 2026-06-08 — `docs-researcher` + vendored-source ground truth)

1. **The "forked `tokio-postgres` `unnamed-statement` branch" candidate is dead.** The blog-cited
   `devsnek/rust-postgres` fork returns **404** / is not discoverable, with no upstream merge. A
   git-branch fork of a database driver on the privacy-critical **auth** path is a standing
   supply-chain + maintenance liability (no crates.io release, no security-advisory coverage, manual
   rebases). It is not a viable production dependency.

2. **The pinned, published `tokio-postgres` 0.7.17 already has the full unnamed-statement family.**
   Verified directly in the vendored source
   (`~/.cargo/registry/src/index.crates.io-*/tokio-postgres-0.7.17/src/client.rs`): the **typed** family
   `query_typed` (L442), **`query_typed_one`** (L467), **`query_typed_opt`** (L489), `query_typed_raw`
   (L553), and **`execute_typed`** (L592, returns rows-affected `u64`); plus the **simple-protocol**
   methods `simple_query` / `simple_query_raw` / `batch_execute`. (`query_raw` (L422) also exists but is
   the *named*-statement raw variant — given a `&str` it auto-`prepare`s a named statement, so it is
   **not** pooler-safe; listed here only to disclaim it.) The upstream doc comment (L436–438, verbatim)
   endorses exactly our use case:

   > *"…without preparing them first … by requiring the caller to specify parameter values along with
   > their Postgres type. Thus, this is suitable in environments where prepared statements aren't
   > supported (**such as Cloudflare Workers with Hyperdrive**)."*

   The **typed** family issues a one-shot **unnamed** statement (`query::execute_typed`/`query_typed`
   both call `frontend::parse("", …)` — empty statement name — `query.rs` L137/L76; Parse/Bind/Execute
   with the caller-supplied `$n` types, nothing cached on the connection), so nothing persists across a
   transaction-mode pooler's connection reassignment; the simple-protocol methods issue no prepared
   statement at all (no `$n` params). `query_with_param_types()` (the `DEFERRED.md`
   name) does not exist — the real method is `query_typed`.

3. **`query_typed_one` / `query_typed_opt` are 1:1 drop-ins** for the store's current `query_one` /
   `query_opt`, so the slice-B migration is mechanical (supply each `$n`'s
   `tokio_postgres::types::Type`), not a rewrite.

4. **Cloudflare documents `tokio-postgres`-over-`Socket` as *the* Rust path** ("tokio-postgres can be
   compiled to Wasm; it must be configured to use a `Socket` from workers-rs"). No other Rust Postgres
   driver is documented for Hyperdrive.

5. **Correction to ADR-0019 (surfaced, not silently reconciled).** ADR-0019's Context says the pooler
   *"dislikes `tokio-postgres`'s **unnamed** prepared statements; the pooler-safe path is `query_raw` /
   simple-protocol."* The polarity is **inverted**: the pooler problem is with **named/persistent**
   cached statements (the default `query`/`execute(&str,…)` path auto-`prepare`s a *named* statement
   that may not exist on a reassigned pooled backend). **Unnamed** statements (`query_typed*`) are the
   **fix**. ADR-0024 records the correct framing; ADR-0019's accepted *decision* (tokio-postgres over a
   Socket, sqlx dropped) is unchanged.

6. **The original premise has softened — recorded honestly.** Hyperdrive *added* named-prepared-statement
   support in **June 2024**, so "the pooler rejects default prepared statements" is no longer strictly
   true. The unnamed path is still the right choice: Cloudflare explicitly **hedges for non-node
   drivers** ("may have worse performance or may not be supported"), `tokio-postgres` is not listed as
   tested, and the unnamed path is correct **independent of whether the Worker reuses a `Client` or
   builds one per request** — Hyperdrive's transaction-mode pooling can reassign the backend connection
   under a persistent `Client` regardless, so not relying on a per-connection named-statement cache is
   the safe posture either way. Unnamed `query_typed*` is the documented, supply-chain-clean,
   account-free-decidable path that does not depend on a Cloudflare-hedged behaviour.

## Decision

**The Boundless Worker stays on the published `tokio-postgres` 0.7.17 (no fork, no driver swap) and
issues every query through the unnamed-statement typed family (`query_typed*` / `execute_typed`), with
simple-protocol methods for no-param/DDL, on the wasm/Hyperdrive path.**

- Parameterized reads: `query_typed_one` / `query_typed_opt` / `query_typed` / `query_typed_raw`,
  passing each `$n`'s `tokio_postgres::types::Type`.
- Parameterized writes (today's `execute(&str, params)`): **`execute_typed`** (`client.rs:592`,
  signature `(&str, &[(&(dyn ToSql + Sync), Type)]) -> u64`) — the 1:1 unnamed drop-in for `execute`,
  returning rows-affected; verified unnamed (`query::execute_typed` → `frontend::parse("", …)`,
  `query.rs:137`). (`query_typed`/`query_typed_raw` also work if a row stream is wanted.)
- No-parameter / DDL: `simple_query` / `batch_execute` (already used by the migration test harness).
- **Forbidden on the Worker path:** the default **named-cached** methods — `query` / `query_one` /
  `query_opt` / `execute` with a `&str`, and any explicit `prepare()` + reuse — because they create a
  named statement that may not survive Hyperdrive connection reassignment.

The crate and version are already pinned (`tokio-postgres = 0.7`, lock `0.7.17`, feature `with-uuid-1`,
ADR-0019); **this ADR adds no dependency** — it only constrains *which methods* the data path calls.

## Considered alternatives

### Option B — a git-branch fork of `tokio-postgres` (`unnamed-statement`)

**Rejected.** The specific fork (`devsnek/rust-postgres`) is **inaccessible (404)** and unmaintained,
with no upstream merge. Even a *live* fork would be an unacceptable supply-chain dependency on the auth
path (no published release, no advisory coverage, manual maintenance) — and it is moot, because the
published crate already exposes the needed unnamed-statement API (Finding 2).

### Option C — swap database drivers (`sqlx`, SeaORM, a Neon serverless HTTP driver)

**Rejected.** `sqlx` was already dropped because it cannot run in the Workers wasm runtime (ADR-0019);
SeaORM is built on `sqlx`; a Neon-specific HTTP driver ties the data path off the documented Hyperdrive
Postgres-wire path and bypasses pooling (rejected as Option D in ADR-0019). Switching adds churn and a
new dependency on the privacy-critical auth path for no benefit `query_typed*` doesn't already provide.

### Option A2 — rely on Hyperdrive's June-2024 named-statement support, keep the default `query*` path

**Rejected (for now).** It would let `PgAuthStore`'s current calls stand unchanged, but it depends on a
behaviour Cloudflare **explicitly hedges for non-node drivers** and does not list `tokio-postgres` as
tested — a risk only verifiable with a real Cloudflare account, and one that yields ~no benefit given
fresh-`Client`-per-request. Unnamed `query_typed*` removes that dependency entirely. (If a future
benchmark ever shows the named-cache path is both safe *and* materially faster for `tokio-postgres`
through Hyperdrive, revisit in a new ADR.)

## Consequences

### Positive

- **Driver question resolved with zero new dependency and no account** — the PgAuthStore-over-Hyperdrive
  build slice is now mechanical (a per-method call migration), not blocked on a fork decision.
- **No supply-chain risk on the auth path** — published, upstream-maintained crate; the dead fork is off
  the table.
- **Correctness independent of a Cloudflare-hedged feature** — unnamed statements survive pooler
  connection reassignment by construction; nothing relies on Hyperdrive's per-connection named cache.
- Keeps ADR-0019's shared-code property: same crate native (tests) and wasm (Worker); only the
  connection transport (`TcpStream` vs Hyperdrive `Socket`) and the query method differ.

### Negative / costs

- **The 9 `PgAuthStore` methods must migrate their query calls** from `query_one`/`query_opt`/`execute`
  to `query_typed_one`/`query_typed_opt`/`execute_typed` (writes), supplying each `$n`'s `Type`.
  Mechanical (1:1 method swaps), but it touches every method and adds per-query type boilerplate.
  (Build slice, T07-shell-B.)
- **Native tests should also exercise `query_typed*`** so the native suite actually covers what ships
  (the contract is enforced where it bites — the Worker — but native-only `query*` would leave the
  shipped method path untested). Recorded as a build-slice follow-up.

### Neutral / follow-ups

- `DEFERRED.md` (T07-shell-B): the "forked driver / `query_with_param_types` / record an ADR" text is
  replaced by "decided by ADR-0024"; the remaining build work (call migration + wasm32 feature flags +
  `hyperdrive.connect()`→`Socket` transport + the real-pooler/deployed-edge E2E) stays — now
  driver-unblocked, still needing a Cloudflare account + a local Postgres.
- `docs/stack-matrix.md`: the `tokio-postgres` row's "pooler-safe `query_raw`" note is updated to name
  the `query_typed*` family (ADR-0024).
- ADR-0019: a "Refined by ADR-0024" pointer is added; its accepted decision is untouched.

## References

- ADR-0019 (Worker→Postgres driver), ADR-0014 (Hyperdrive/server-driven config), ADR-0021 (access-token verify lookup)
- `server/store/src/lib.rs` (`PgAuthStore` — the methods to migrate); `core/server/src/ports.rs` (`AuthStore` contract)
- `tokio-postgres` 0.7.17 vendored source: `src/client.rs` L436–442 (`query_typed` doc + signature; the "Cloudflare Workers with Hyperdrive" endorsement), `src/client.rs` L592 (`execute_typed`), `src/query.rs` L76/L137 (`frontend::parse("", …)` — both typed paths are unnamed) — verified 2026-06-08; lock = ground truth
- [`tokio-postgres` 0.7.17 `Client`](https://docs.rs/tokio-postgres/0.7.17/tokio_postgres/struct.Client.html)
- [Cloudflare — Supported Rust crates](https://developers.cloudflare.com/workers/languages/rust/crates/) (tokio-postgres over a Socket; no other Rust driver)
- [Cloudflare Hyperdrive — Connect to Postgres](https://developers.cloudflare.com/hyperdrive/examples/connect-to-postgres/) · [worker `Socket`](https://docs.rs/worker/latest/worker/struct.Socket.html)
- [Cloudflare blog — Supporting Postgres Named Prepared Statements in Hyperdrive](https://blog.cloudflare.com/postgres-named-prepared-statements-supported-hyperdrive/) (June 2024; the hedge for non-node drivers)
