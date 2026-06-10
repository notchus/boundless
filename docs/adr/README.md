# Architecture Decision Records (ADRs)

This directory is the **append-only long-term memory** of the Boundless project. Every non-trivial architectural decision lands here.

## Why ADRs

The biggest long-term LLM failure mode is *context drift*: a model in month 18 doesn't know why the model in month 1 decided what it did. ADRs are the fix. The model reads them when needed instead of trying to remember them.

## Conventions

- **Numbering is permanent.** Once `0001-rust-core.md` exists, that number is taken forever — even if the ADR is superseded.
- **Filenames:** `NNNN-short-kebab-title.md` where NNNN is 4 digits, zero-padded.
- **Status field:** `Proposed`, `Accepted`, `Deprecated`, `Superseded by ADR-NNNN`.
- **Never delete an ADR.** Mark it Deprecated or Superseded and explain.
- **Specs reference ADRs**, not the other way around.
- **Every ADR considers at least 2 alternatives** explicitly.
- **Every ADR documents trade-offs**, not just upsides.

## Process

To create one, use the slash command:

```
/adr <short-kebab-title>
```

It scaffolds the file from the template in `.claude/commands/adr.md`.

## Read this before reading any individual ADR

The constitution (`.specify/memory/constitution.md`) holds higher authority than any ADR. If an ADR and the constitution conflict, the constitution wins, and the ADR needs to be amended (or the constitution updated via the amendment process documented at the bottom of the constitution).

## Currently-active ADRs

- [ADR-0001](./0001-rust-core.md) — Single shared Rust core for domain types and business logic
- [ADR-0006](./0006-role-swaps.md) — Role swaps allowed: a Member's role is a set (`roles member_role[]`), not a scalar; issuance (spec 008) sets the initial set, the per-Gathering swap *workflow* is a sibling spec
- [ADR-0013](./0013-license.md) — AGPL-3.0 for the entire repository (+ App Store §7 exception)
- [ADR-0014](./0014-server-driven-config.md) — Server-Driven Configuration via Cloudflare KV (supports P13 / O1–O8)
- [ADR-0015](./0015-admin-invitation-channel.md) — Admin invitation channel; narrows I11 to permit a developer-minted single-use Email Workers registration link
- [ADR-0016](./0016-auth-model.md) — Authentication & device-binding model (Onboarding Code, indefinite silent-refresh sessions, rider/driver recovery, admin WebAuthn)
- [ADR-0017](./0017-admin-auth-edge-webauthn.md) — Admin auth = in-app WebAuthn verified by @simplewebauthn/server on the Cloudflare edge (no sidecar); resolves the architecture.md-vs-spec conflict; documents the P4 carve-out
- [ADR-0018](./0018-keyed-hash-hmac-sha256.md) — Keyed-hash algorithm: HMAC-SHA256 via RustCrypto `hmac`+`sha2` (dryoc has no SHA-256); I3 kept verbatim; dryoc stays the sole Ed25519 signature impl; getrandom `wasm_js` backend on wasm32
- [ADR-0019](./0019-worker-postgres-driver.md) — Worker → Postgres via `tokio-postgres` over a Hyperdrive Socket (not `sqlx`, which can't run in Workers wasm); the native `boundless-server-store` adapter + real-Postgres tests land first, the wasm wiring + async-port bridge are T07-shell-B
- [ADR-0020](./0020-async-auth-ports-device-split.md) — The async-port bridge: `core::server`'s store ports become `async` + fallible (shared `StoreBackend::Error`), device-token methods split into a separate `DeviceStore` port (its Postgres impl is blocked on spec-008 token encryption), `PgAuthStore` now implements `AuthStore`, and `AuthService` is proven end-to-end against real Postgres
- [ADR-0021](./0021-access-token-wire-format.md) — Access-token wire format = opaque-random 32-byte bearer verified by a constant-time keyed-HMAC store lookup (not EdDSA-JWT); resolves the plan §10-D open item; honors the time-independent, family-status-gated revocation model with zero new key-management infra
- [ADR-0022](./0022-uniffi-binding-mirror-types.md) — UniFFI binding crates (`core/ffi-swift`, later `core/ffi-kotlin`) mirror the core's enums with `#[derive(uniffi::Enum)]` + exhaustive `From` conversions instead of annotating the core, because the core must stay `uniffi`-free to keep compiling to `wasm32`; the exhaustive `match` is a compile-checked parity guard (not a hand-rolled duplicate, P4)
- [ADR-0023](./0023-auth-request-phone-on-wire.md) — Auth requests carry the plaintext `phone` (E.164, over TLS), not a client-computed `phone_lookup_hash`: that hash is HMAC-SHA256 keyed by a per-instance server secret (I3/ADR-0018) a client cannot hold, so the server hashes the received phone and drops the plaintext (P2 tainted type); reconciles the frozen OpenAPI/spec/fixtures with the as-built engine. Rejects a client keyless pre-hash (unsalted low-entropy hash is brute-forceable + violates P4 single-source normalization)
- [ADR-0024](./0024-hyperdrive-unnamed-statements.md) — Worker→Postgres uses `tokio-postgres`'s unnamed-statement `query_typed*` family (no driver fork); refines ADR-0019 and corrects its named/unnamed polarity
- [ADR-0025](./0025-per-group-key-lifecycle.md) — Per-Group field-encryption key lifecycle: a symmetric secretbox key, KEK-wrapped in Cloudflare Secrets Store, generated at Group bootstrap (spec 008), rotation runbook-driven (I1)

## Suggested early ADRs to author (stubs)

These were decided during the planning chats and should be formalized:

- ADR-0002 — Native UI on every platform (SwiftUI / Compose / SvelteKit), no cross-platform UI frameworks
- ADR-0003 — Cloudflare edge as the server tier (Workers + DOs + Hyperdrive)
- ADR-0004 — Neon Postgres with PostGIS via Hyperdrive for the primary store
- ADR-0005 — OpenAPI 3.1 + Protocol Buffers as API contract source of truth
- ADR-0006 — Role swaps allowed (a person may be Rider in one context, Driver in another) — **authored 2026-06-10 (spec 008); see the active list above**
- ADR-0007 — Silent reassignment (no "your driver changed" notification)
- ADR-0008 — ETA matrix computed batch on admin updates, not in request path
- ADR-0009 — Closed-group privacy model (no self-signup, admins issued by developer only)
- ADR-0010 — Optional Live Tracker is E2E encrypted (server cannot decrypt)
- ADR-0011 — Spec-Driven Development with GitHub Spec Kit as the development methodology
- ADR-0012 — Weblate as the translator workflow (self-host vs hosted: open)

Write these as you encounter the decisions during implementation, not all at once.

> ADR-0014 was authored ahead of order because the update-strategy is foundational — P13 needed it materialized before any UI work begins, since the manifest schema constrains how every client renders content. Subsequent ADRs (0015+) get the next available numbers.
