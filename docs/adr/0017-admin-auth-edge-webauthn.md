# ADR-0017: Admin Auth — In-App WebAuthn Verified on the Cloudflare Edge

- **Status:** Accepted
- **Date:** 2026-06-04
- **Author:** Boundless founder
- **Deciders:** Boundless founder

## Context

Two always-loaded documents disagreed on how an Admin (Sarah) authenticates:

- `docs/architecture.md` §4 (older) said admin auth is **"via Cloudflare Access (SSO + passkeys)."**
- Spec `001-onboarding` + **ADR-0016 D4** (newer) specify an **in-app WebAuthn ceremony** against our **own** credential store (`admin_webauthn_credentials`), with Developer-re-invite recovery (ADR-0015).

The `/speckit.plan` gate surfaced this as a conflict (plan §10-A) and the founder chose **in-app WebAuthn** (keep admin identity in our own store — best fit for the closed-group, Developer-issued model (I11), and for keeping auth logic ours rather than Cloudflare's).

Choosing in-app WebAuthn raised a second question: **where does the server-side WebAuthn verification run?** A `docs-researcher` pass (2026-06-04) established two facts:

1. The Rust crate **`webauthn-rs` 0.5.5** (MPL-2.0) is solid but depends on `openssl-sys` (C-FFI) and therefore **cannot compile to/run in the Cloudflare Workers wasm runtime**. Using it would force a separate **native Rust sidecar service**.
2. **`@simplewebauthn/server` 13.3.1** (MIT) is WebCrypto-based, **runs in the Workers/`workerd` runtime** (since v8.0.0, hardened in v8.3.4–8.3.5), and supports everything ADR-0016 D4 requires: discoverable/resident credentials (usernameless), `userVerification: "required"`, `attestation: "none"`, and multiple credentials per Admin.

## Decision

**D1 — In-app WebAuthn, not Cloudflare Access.** Admin authentication is an in-app WebAuthn ceremony against our own `admin_webauthn_credentials` store (ADR-0016 D4 stands). Cloudflare Access does **not** own admin identity. `architecture.md` §4, the trust-boundary table, and the diagram are amended to match.

**D2 — Verification runs in TypeScript on the SvelteKit Cloudflare edge.** Server-side WebAuthn registration + assertion verification uses **`@simplewebauthn/server` (13.x)** in the SvelteKit server routes, deployed on Cloudflare. **No native sidecar.** `webauthn-rs` is explicitly **not** adopted (it can't run in Workers wasm).

**D3 — WebAuthn challenges are persisted in KV.** Because the edge is stateless, the per-ceremony challenge is written to Cloudflare KV with a short TTL (~5 minutes), one-time-use (deleted on successful verify). (This is inherent to any stateless-edge WebAuthn RP, not specific to the library.)

**D4 — Documented P4 carve-out.** Constitution P4 puts business logic in the Rust core. Admin WebAuthn is the **one** exception in the onboarding slice: the *ceremony* is browser-native (the platform WebAuthn API) and the *verification* is `@simplewebauthn/server` in TypeScript on the edge — **neither lives in `core::auth`**. This is justified because (a) WebAuthn is a standardized web-platform protocol, not Boundless business logic; (b) no wasm-compatible Rust RP verifier exists today; (c) the admin surface is web-only, so there is no cross-platform duplication for the core to prevent. **Everything else** in auth — Onboarding/Recovery codes, sessions, refresh rotation, device-token binding, version comparison — **stays in `core::auth`** (P4 intact). ADR-0016 D4 already located WebAuthn verification "server-side, not in core"; this ADR makes that concrete and records the P4 exception explicitly.

## Considered alternatives

### Option A — Cloudflare Access owns admin identity
Admin identity managed by Cloudflare Zero Trust Access (passkey SSO at the edge). **Pros:** least code; Access free tier covers <50 users (P11-ok). **Cons:** admin identity lives in Cloudflare's list, not ours; reshapes the I11 Developer-issued + ADR-0015 re-invite-recovery model we deliberately designed; weaker control over the issue/revoke lifecycle. **Rejected** by the founder (plan §10-A).

### Option B — In-app WebAuthn verified by a native Rust sidecar (`webauthn-rs`)
Keeps verification in Rust (P4-pure). **Pros:** single-language auth; SUSE-audited crate. **Cons:** adds an always-on native service to deploy/host/monitor — against the all-Cloudflare-edge architecture and the free/donation-funded ethos; more operational surface for a low-RPS management plane. **Rejected** as the default (kept as a documented fallback — see Consequences).

### Option C (chosen) — In-app WebAuthn verified by `@simplewebauthn/server` on the edge
**Pros:** no new infra (one deployment unit — the SvelteKit Worker); MIT (P11-ok); WebCrypto-based, no FFI; feature-complete for ADR-0016 D4; stays all-Cloudflare. **Cons:** introduces a non-Rust auth implementation (the P4 carve-out, D4); Workers support is "unofficially supported" (periodically tested, no SLA) — mitigated below; requires KV challenge persistence (D3).

## Consequences

### Positive
- Admin auth stays on the existing Cloudflare edge — **zero new infrastructure**, no sidecar to run or pay for (P11).
- Admin identity remains in our own store, preserving the I11 / ADR-0015 issue-and-re-invite-recovery model.
- The P4 exception is narrow and explicit; all non-WebAuthn auth logic stays in `core::auth`.

### Negative / costs / mitigations
- **Non-Rust auth code** on the admin path (the P4 carve-out). Bounded to WebAuthn verification only; documented here.
- **`@simplewebauthn` Workers support is "unofficially supported."** If an edge-runtime break ever appears, the **fallback is Option B (a native `webauthn-rs` sidecar)** — recorded in `DEFERRED.md` (Auth / Onboarding). We do not build the sidecar now.
- **KV challenge persistence (D3)** is required correctness, not optional — a missing challenge store breaks verification. Captured as an implementation requirement in the plan/tasks.
- **Resident-credential slot limits** on hardware keys (e.g. ~25 per YubiKey) — mitigated by ADR-0016 D4's "register a backup credential" guidance and accepting cross-platform keys.

### Neutral / follow-ups
- `docs/stack-matrix.md` pins `@simplewebauthn/server` 13.x (Web) and notes `webauthn-rs` was considered and rejected for the edge constraint.
- Concrete WebAuthn challenge TTL and the KV namespace are implementation details for `/speckit.tasks`.

## Compliance
- **Constitution:** P4 — documented carve-out (D4); P11 — MIT lib, no paid dependency, no new infra; P7 — admin is web-only, no shared-UI concern.
- **ADRs:** ADR-0016 D4 (in-app WebAuthn parameters) — unchanged and now concretely hosted; ADR-0015 (Developer re-invite recovery) — unchanged; ADR-0001/P4 — carve-out recorded.
- **Privacy invariants:** I11 (Admins Developer-issued; no self-signup) — preserved; admin reads remain audit-logged (I5).
- **Docs amended:** `architecture.md` §1 (adds `core/auth`), §4, trust-boundary table, and the diagram; `stack-matrix.md` (Web table); spec 001 plan §10.

## References
- `specs/001-onboarding/spec.md` (AC2, AC11b, AC16, AC20) and `specs/001-onboarding/plan.md` §10-A/B
- ADR-0016 (auth model, D4 admin WebAuthn) · ADR-0015 (admin invite channel) · ADR-0001 (Rust core / P4)
- `docs/architecture.md` §4 + trust-boundary table
- [@simplewebauthn/server](https://simplewebauthn.dev/docs/packages/server/) (v13.3.1, MIT; Cloudflare Workers support since v8.0.0)
- [Cloudflare Workers Web Crypto](https://developers.cloudflare.com/workers/runtime-apis/web-crypto/)
- [webauthn-rs](https://github.com/kanidm/webauthn-rs) (MPL-2.0; native-only — `openssl-sys` blocks wasm; the documented fallback)
