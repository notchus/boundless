# ADR-0020: Async, fallible auth-store ports + a separate `DeviceStore` port (the async-port bridge)

- **Status:** Accepted
- **Date:** 2026-06-05
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0016 (sessions/refresh), ADR-0019 (Worker→Postgres driver); I4; spec 001 tasks T07 (T07-core, T07-shell slice A, **T07-shell-B**)
- **Amends:** the `core::server` `AuthStore` port contract introduced in T07-core (`core/server/src/ports.rs`).

## Context

T07-core shipped the member-auth orchestration engine (`core/server`, `boundless-server-core`)
behind a **sync, infallible** `AuthStore` port — correct for the in-memory test double. T07-shell
slice A shipped `PgAuthStore` (`server/store`), whose methods deliberately **mirror** `AuthStore`
1:1 but are **`async` and fallible** (`-> Result<_, StoreError>`) — a database backend is inherently
asynchronous and can fail on transport, and you cannot block-on-async in the Cloudflare Workers wasm
runtime (ADR-0019). The two halves had never run together.

ADR-0019 named this gap explicitly ("**the `AuthStore` port must become async** before the Worker can
use `PgAuthStore` — the async-port bridge") and deferred it so slice A would not async-refactor the
committed, 48-test T07-core for a consumer that did not exist yet. This ADR records building that
bridge (T07-shell-B, slice 1), which is the prerequisite for the deployable Worker.

A second constraint surfaced while wiring `PgAuthStore` into `AuthService`: a literal
`impl AuthStore for PgAuthStore` is **impossible today** because the trait includes the three
device-token methods, and `register_device` must persist a **reversibly-encrypted** device token
(push needs the plaintext back, so a one-way hash will not do) — and that at-rest encryption
primitive is deferred to spec 008. Only that one concern is blocked; the other nine session / code /
member methods are fully built.

## Decision

**1. The store ports become `async` + fallible, sharing one error via a supertrait.**

```rust
pub trait StoreBackend { type Error; }              // one error for the whole backend
pub trait AuthStore: StoreBackend { /* 9 async fns -> Result<_, Self::Error> */ }
pub trait DeviceStore: StoreBackend { /* 3 async fns -> Result<_, Self::Error> */ }
```

`AuthService`'s endpoint methods (`sign_in`/`bind_device`/`refresh`/`recovery_rebind`) become
`async fn … -> Result<Resp, St::Error>` and `.await?` the store calls. The orchestration logic is
**unchanged** — only sync→async + error propagation. The shared `StoreBackend::Error` lets the `?`
operator unify across both ports in the methods that touch both.

**2. The device-token methods split into their own `DeviceStore` port.** This lets `PgAuthStore`
implement the complete `AuthStore` (session/code/member) **now**, while its `DeviceStore`
implementation lands with spec 008 (token encryption). `AuthService` requires `St: AuthStore` for
`sign_in`/`refresh`, and `St: AuthStore + DeviceStore` for `bind_device`/`recovery_rebind` (which
re-bind a device). The in-memory test stub implements both; production will compose
`PgAuthStore` (AuthStore) with a future `PgDeviceStore` (DeviceStore).

**3. Reads take `&mut self`** (the Postgres twin scopes every statement inside a per-request
transaction for RLS, which needs `&mut`; the in-memory twin ignores it). No call site holds two
store borrows at once.

**4. The trait futures are intentionally not `Send`-bound** (`#[allow(async_fn_in_trait)]`, with a
documented justification): the wasm `?Send` Worker drives them on its single-threaded event loop,
and the host unit tests use a single-threaded executor. Nothing spawns these futures across threads.
Native AFIT (stable since Rust 1.75) is used directly — **no `async-trait` proc-macro dependency**.

**5. Tests.** The 48 committed T07-core tests are adapted to drive the now-async endpoints via
`pollster::block_on` (the `MemStore` futures are always ready — no async runtime needed; a thin
test-only blocking extension trait keeps the `#[test]` bodies readable). A new real-`postgres:16`
suite (`server/store/tests/service_pg.rs`) drives `AuthService` over the **real** `PgAuthStore`
(composed with an in-memory `DeviceStore`), proving sign-in / atomic onboarding-consume / refresh
rotate-then-replay-kill / recovery consume-and-rotate / below-min degradation **end-to-end against
Postgres** — the genuinely-new proof the bridge unlocks.

## Considered alternatives

### Keep the ports sync; `block_on` inside the adapter

**Rejected:** you cannot block-on-async in the Workers wasm runtime (the whole reason the adapter is
async — ADR-0019). The orchestration must `.await` the Postgres calls, so it must be async.

### Keep one combined `AuthStore` trait (device methods included)

**Rejected:** `PgAuthStore` cannot implement `register_device` honestly today — it would have to
store a device token without the (deferred) reversible encryption, violating P2 / the `_encrypted`
column contract. A `panic!`/`unimplemented!` default is forbidden (shipped-incomplete code). Splitting
the device port is the honest decomposition and isolates exactly the blocked concern.

### Box the error (`Box<dyn Error + Send + Sync>`) instead of an associated type

**Rejected:** loses the concrete error type (the in-memory stub is genuinely `Infallible`; the
Postgres backend is `StoreError`) and would force allocation on the error path. The shared
`StoreBackend { type Error }` supertrait gives one statically-known error per backend with no boxing.

### Add the `async-trait` crate

**Rejected:** unnecessary — native `async fn` in traits is stable and sufficient here (we do not need
`dyn`-dispatch or `Send` bounds on these ports). Avoids a proc-macro dependency.

## Consequences

### Positive

- `AuthService` now runs against **real Postgres**, proven end-to-end (`service_pg.rs`) — the bridge
  ADR-0019 deferred is built, and the deployable Worker (the rest of T07-shell-B) just supplies real
  port implementations.
- The device-token concern is cleanly isolated behind `DeviceStore`, so its deferral (spec 008) no
  longer blocks the session/code/member half.
- No `async-trait` dependency; the core stays runtime-free and `wasm32`-safe (verified:
  `cargo build --target wasm32-unknown-unknown -p boundless-server-core`).

### Negative / costs

- **Committed T07-core's public API changed:** the ports are `async` + fallible (associated
  `StoreBackend::Error`); the endpoint methods now return `Result<_, St::Error>`; `bind_device` /
  `recovery_rebind` now require `St: AuthStore + DeviceStore`. All 48 tests were updated.
- New **dev-only** dependency `pollster` (zero production deps; Apache-2.0 OR MIT; MSRV 1.69) for the
  host tests' `block_on`.
- The **`DeviceStore` Postgres implementation is still deferred** (needs spec-008 token encryption);
  until then the Worker/tests compose `PgAuthStore` with an in-memory `DeviceStore`.

### Neutral / follow-ups

- `docs/stack-matrix.md`: `pollster` added (Rust core, dev-only).
- `DEFERRED.md`: the async-port-bridge item is ticked DONE; the remaining T07-shell-B is the
  workers-rs Worker runtime + the `DeviceStore` Postgres impl.

## References

- `core/server/src/ports.rs` (`StoreBackend`/`AuthStore`/`DeviceStore`); `core/server/src/{service,signin,bind,refresh,recovery}.rs`
- `server/store/src/lib.rs` (`impl AuthStore for PgAuthStore`); `server/store/tests/service_pg.rs` (orchestration-over-Postgres)
- ADR-0019 (Worker→Postgres driver; named this bridge as deferred); ADR-0016 D2 (sessions/refresh); I4
- [Rust 1.75 — `async fn` in traits](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html) · [`pollster` 0.4.0](https://docs.rs/pollster/0.4.0/pollster/fn.block_on.html) — verified 2026-06-05
