# Boundless Architecture

> This is the *intent*. Implementation may evolve; if it diverges, update this file in the same PR.

---

## One-paragraph summary

Boundless is a privacy-first geofence carpooling platform for closed groups. Native UI on every platform (SwiftUI on Apple, Compose on Android, SvelteKit for admin web). A single Rust core encapsulates domain types, matching, cryptography, and sync — compiled to Swift/Kotlin frameworks for clients and to WebAssembly for Cloudflare Workers. The edge tier (Cloudflare Workers + Durable Objects) hosts the matching service, WebSocket fanout, and admin API. Persistent data lives in Neon Postgres (with PostGIS) accessed via Hyperdrive; PII is encrypted at the field level with per-Group keys, and plaintext addresses exist only during the matching call.

---

## High-level diagram

```
                          ┌──────────────────────────────────┐
                          │       Rust core (boundless)      │
                          │  domain · matching · crypto ·    │
                          │  sync · geofence · ETA           │
                          └──────────────────────────────────┘
                                  │            │           │
                  ┌───────────────┘            │           └───────────────┐
                  ▼                            ▼                           ▼
         UniFFI → Swift               UniFFI → Kotlin              cargo (native + wasm)
                  │                            │                           │
       ┌──────────┴──────────┐      ┌──────────┴──────────┐                │
       ▼          ▼          ▼      ▼          ▼          ▼                ▼
    iOS Rider  watchOS    macOS  Android   Wear OS    Glance       Cloudflare Workers
    iOS Driver Rider+Drv  Drv+   Rider     Rider+Drv  widgets        (Rust via Wasm)
    iPad       complic'ns admin  Driver    Tiles                          │
    SwiftUI    SwiftUI    SwiftUI Compose  Compose                        │
                                                                          ▼
                                                              ┌──────────────────────┐
                                                              │ Durable Objects      │
                                                              │  one per Group:      │
                                                              │  · matching engine   │
                                                              │  · WebSocket hub     │
                                                              │  · ephemeral addrs   │
                                                              └──────────────────────┘
                                                                          │
                                                                ┌─────────┼─────────┐
                                                                ▼         ▼         ▼
                                                              KV     Queues   Workflows
                                                          (i18n)  (push     (multi-step
                                                                   fanout)   reassign)
                                                                          │
                                                                          ▼
                                                                    Hyperdrive
                                                                          │
                                                                          ▼
                                                                 ┌──────────────────┐
                                                                 │ Neon Postgres    │
                                                                 │  + PostGIS       │
                                                                 │  + pgcrypto      │
                                                                 │  + RLS           │
                                                                 │  PII encrypted   │
                                                                 └──────────────────┘

                          SvelteKit Admin Web ──── Cloudflare Pages ─── WebAuthn (in-app) ─── same Workers API

External:
  · APNs (Apple) and FCM (Google) — push, via Queues
  · Self-hosted Nominatim + OSRM — batch geocoding/ETA on admin address updates (NOT in request path)
  · Weblate — translator workflow
  · GitHub Actions — CI
```

---

## Components

### 1. Rust core

Workspace under `core/`. Crates:

- `core/domain` — `Rider`, `Driver`, `Group`, `Chain`, `OptOut`, `Gathering`, `EffortCaps`, tainted-type wrappers (`Address`, `PhoneNumber`, `DeviceToken`). Pure logic, no I/O.
- `core/matching` — geofence algorithm, chain construction, effort-cap enforcement. Property-tested with `proptest`.
- `core/crypto` — sealed boxes, per-Group keys, phone hashing, address encryption.
- `core/auth` — authentication & onboarding logic: device binding (Onboarding/Recovery codes), session + silent refresh-token rotation, device-token binding `(member_id, platform, app_version)`, client-version comparison. Pure logic + injected `Clock`/RNG. See ADR-0016 and spec 001. (Admin WebAuthn *verification* is the exception — it runs in TypeScript on the edge, not here; see §4 and ADR-0017.)
- `core/sync` — WebSocket message types (compiled from `api/boundless.proto`), state machines.
- `core/server` — Worker entry points, Durable Object class, Hyperdrive queries.
- `core/ffi-swift` — UniFFI binding crate → produces XCFramework.
- `core/ffi-kotlin` — UniFFI binding crate → produces AAR.
- `core/ffi-wasm` — wasm-bindgen crate → produces JS package for admin web (limited use, mainly validation).

**Forbidden in `core::domain`:** any I/O, time-now, randomness without injection. See `docs/forbidden-patterns.md`.

### 2. Apple workspace

`apple/` — Xcode workspace.

- `BoundlessKit` — Swift Package wrapping the XCFramework.
- `BoundlessRider` — iOS + iPadOS + watchOS targets sharing SwiftUI views via a `RiderShared` package.
- `BoundlessDriver` — iOS + iPadOS + macOS + watchOS targets, similar share pattern.
- `Widgets` — WidgetKit extensions for Lock Screen and Smart Stack widgets.
- Both apps use Live Activities for the doorbell moment.

### 3. Android workspace

`android/` — Gradle multi-module.

- `core-bridge` — Kotlin wrapper around the UniFFI AAR; idiomatic Kotlin coroutines on top of Rust suspend bindings.
- `rider/app` — phone app (Compose).
- `rider/wear` — Wear OS app (Compose for Wear).
- `rider/glance` — Glance widgets.
- `driver/app`, `driver/wear`, `driver/glance` — same for Driver.

### 4. Web admin

`web/` — SvelteKit 2.

- Server routes hit the same Workers API as the apps (no separate admin API).
- Admin auth is **in-app WebAuthn** (passkeys or hardware keys) against our own credential store — verified server-side in TypeScript (`@simplewebauthn/server`) on the SvelteKit Cloudflare edge, with WebAuthn challenges held in KV (5-min TTL). **Not** Cloudflare Access. See ADR-0016 D4 and ADR-0017.
- Heavy table UIs with TanStack Table + TanStack Query.

### 5. Edge / server

`server/` — workers-rs project.

- Routes:
  - `/api/auth/*` — passkey + 6-digit code flows
  - `/api/rider/*` — rider client API
  - `/api/driver/*` — driver client API
  - `/api/admin/*` — admin API (audit-logged)
  - `/api/dev/*` — developer-only (admin issuance)
  - `/ws/...` — WebSocket upgrade to DO
- Durable Object class: `GroupHub` — one per Group, holds:
  - Connected WebSocket sessions
  - Matching state (ephemeral plaintext addresses during compute)
  - Per-Group key (encrypted at rest in DO storage, decrypted via Secrets Store on init)
- Workflows: `ReassignmentFlow`, `NightlyMatchFlow`.

### 6. Database (Neon Postgres)

`server/migrations/` — numbered SQL migrations.

Key tables:
- `groups` — Group metadata
- `members` — (Member, Group) pairs with role, encrypted PII
- `gatherings` — recurring + special events
- `seat_toggles` — per (Driver, Gathering instance) availability
- `chains` — historical matches (opaque IDs only)
- `audit_log` — admin reads of PII
- `device_tokens` — push tokens, scoped per (Member, Platform, App Version)
- `delegated_keys` — per-Group encryption keys, themselves encrypted with KEK

Row-level security enforced on every PII-bearing table.

---

## Key data flows

### A — Rider opt-out

```
Rider taps "Can't make it tonight"
  → Rider client posts /api/rider/me/opt-out  (no body, idempotent)
  → Worker authenticates session
  → Worker calls GroupHub DO's RPC `opt_out(member_id, gathering_id)`
  → DO updates ephemeral state, persists OptOut row
  → DO broadcasts WS event `RiderOptedOut { rider_id }` to assigned Driver session
  → DO triggers ReassignmentFlow if chain affected
  → Worker returns 204
```

### B — Driver seat toggle on

```
Driver taps "I have a seat tonight" with Effort Caps
  → Driver client posts /api/driver/me/seat-toggle { caps, gathering_id }
  → Worker calls GroupHub DO's `set_seat(driver_id, caps, gathering_id)`
  → DO persists, broadcasts to Admin web (live counts)
  → If new Riders are unmatched, DO immediately runs matching
```

### C — Match computation (the privacy-sensitive one)

```
DO's nightly trigger (or new-seat trigger) calls `compute_chain(gathering_id)`
  → DO fetches encrypted addresses for currently-in Riders + available Drivers
  → DO loads Per-Group key from local cache (else fetches via Secrets Store)
  → DO decrypts addresses INTO A MatchingContext (Rust struct, no Clone)
  → core::matching::compute(&ctx) returns Chain(member_id ordering only)
  → ctx is dropped → memory zeroed
  → DO persists Chain rows
  → DO computes Approximate Pickup Time per Rider using cached ETA matrix
  → DO broadcasts `ChainAssigned` events via WS
  → For offline sessions: Queues → APNs/FCM
```

### D — Approximate Pickup Time

```
For Recurring Gatherings:
  drive_off (from Driver clock) + duration[Driver→Rider] (from cached ETA matrix)
  = approx_pickup
  
The ETA matrix is computed by a scheduled Worker on admin address updates:
  Admin updates address in /api/admin/members/{id}
  → Triggers a Workflow that calls Nominatim (geocode) and OSRM (durations)
  → Stores duration[D_i → R_j] in `eta_matrix` table for the Group
  → Plaintext addresses never enter this path (already encrypted; Nominatim
    receives the new address only at the moment of admin update)
```

---

## Trust boundaries

| From → To | Trust |
|---|---|
| Rider client → Worker | Authenticated; rate-limited; Turnstile on auth |
| Driver client → Worker | Authenticated; rate-limited |
| Admin web → Worker | Authenticated via in-app WebAuthn (passkey/hardware key; verified on the edge via `@simplewebauthn/server`); audit-logged. See ADR-0017 |
| Developer → Worker `/api/dev/*` | Hardware-key (WebAuthn) only |
| Worker → DO | RPC, in-Cloudflare network only |
| DO → Postgres | Hyperdrive, mTLS |
| DO → Secrets Store | First-party binding |
| DO → KV | First-party binding |

---

## Deployment topology

- **Workers + DOs + KV + Queues + Workflows:** Cloudflare global edge.
- **Postgres:** Neon (or self-hosted), single region (EU by default), read replicas via Hyperdrive.
- **Nominatim + OSRM:** containerized, deployed on Hetzner / Fly / Cloudflare Containers (when GA) — *not* in the request path, only on admin address updates.
- **Static admin web:** Cloudflare Pages.
- **Translation pipeline:** Weblate (self-hosted or weblate.org).
- **Observability:** OpenTelemetry → self-hosted Grafana stack (Tempo/Mimir/Loki) on the same Hetzner box as Nominatim, or Cloudflare Analytics Engine for non-PII metrics.

---

## What this architecture explicitly does not do

- **No native desktop apps.** Admin is a web app; the responsive design covers desktop browsers.
- **No third-party tracking, analytics, or ad SDKs.**
- **No live tracking of Drivers by default.** Optional, opt-in, E2E-encrypted only.
- **No marketplace features** (rating, tipping, surge pricing, etc.). This is not Uber.
- **No multi-tenancy across organizations.** One Boundless install = one Group.
- **No self-signup.** Admin issues all member accounts.

---

## Future considerations (not commitments)

- **visionOS app** — free port of the iPad SwiftUI codebase. Probably never relevant for elderly users but trivial to ship.
- **Cloudflare Containers** — when GA, may replace Hetzner-hosted Nominatim/OSRM.
- **Federation between Groups** (e.g. a regional federation of congregations sharing drivers across nearby groups) — explicitly out of scope for v1.
- **End-user-installable Boundless** ("I want to run this for my congregation") — eventually, with a one-click deploy template, but only once the multi-instance story is polished.
