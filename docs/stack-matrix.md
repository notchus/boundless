# Boundless Stack Matrix

> **Single source of truth for what library, what version.** Never invent a version. If you need to upgrade, file an ADR and update this file.
>
> The lock files (`Cargo.lock`, `Package.resolved`, `pnpm-lock.yaml`, `gradle.lockfile`) are the *enforced* truth; this file is the *intended* truth and should match them. If they diverge, lock files win and this file is updated to match.
>
> **Update process:** PR that bumps a version must include (a) the new version, (b) the changelog entry justifying it, (c) any required code migrations.

---

## Languages & toolchains

| Tool | Version | Pinned via |
|---|---|---|
| Rust | 1.95.0 (latest stable at project init, 2026-06-04; ≥ dryoc MSRV 1.89) | `rust-toolchain.toml` |
| Swift | TODO (latest stable shipping with Xcode) | Xcode version pin |
| Kotlin | TODO (2.x latest stable) | `gradle/libs.versions.toml` |
| TypeScript | TODO (5.x strict) | `package.json` + `tsconfig.json` |
| Node.js | 22 LTS (current LTS) | `.nvmrc` |
| Xcode | TODO (latest GM) | `.xcode-version` |
| Android Studio | TODO (latest stable) | (developer machine) |
| pnpm | latest 9.x | `packageManager` in `package.json` |
| uv | latest | (developer machine) |

> **Why some entries are TODO:** at project init, run `cargo --version`, `swift --version`, etc. and fill these in with the exact strings. Then pin via the listed mechanism.

---

## Rust core

| Crate | Version | Used for |
|---|---|---|
| `serde` + `serde_json` | 1.0.228 / 1.0.150 | Domain type (de)serialization. `serde` with `derive` (pulls transitive `serde_core` 1.0.228); `serde_json` is **test-only** (golden-fixture round-trip). MIT OR Apache-2.0, wasm32-safe. Pinned 2026-06-04 (spec 001 T02; lock = ground truth). |
| `uniffi` | TODO (latest stable) | Swift/Kotlin binding generation |
| `wasm-bindgen` | TODO | wasm target for admin web (limited) |
| `tokio` | TODO | Async runtime (server side only) |
| `proptest` | 1.11.0 | Property-based tests. **Dev-only** (host test runner — never compiled into the wasm32 client/server target, so no `getrandom`/wasm caveat). First used in `core/auth` (spec 001 T04) for the code/version invariants (`prop_onboarding_code_single_use_ttl_ratelimit`, `prop_n_minus_2_version_window`); also the future matching property tests. Failing seeds persist to each crate's committed `proptest-regressions/` (P9 reproducible seeds). MIT OR Apache-2.0; MSRV 1.85 (≤ the workspace 1.89 floor). Latest published release, verified 2026-06-04 via docs-researcher (lock = ground truth). Pinned spec 001 T04. |
| `pollster` | 0.4.0 | **Dev-only.** Minimal `block_on` to drive `core/server`'s now-`async` `AuthStore`/`DeviceStore` ports in host unit tests (the in-memory stub's futures are always ready, so a full async runtime is unnecessary). Added with the async-port bridge (ADR-0020, spec 001 T07-shell-B). **Zero production dependencies** (allow-list clean, I8). Apache-2.0 OR MIT; MSRV 1.69 (≤ the workspace 1.89 floor). Latest published release, verified 2026-06-05 via docs-researcher (lock = ground truth). |
| `rand_core` | 0.9.5 | **Traits only** (`default-features = false` → **no `getrandom`**): the `RngCore`/`CryptoRng`/`SeedableRng` contracts that `core/server`'s production `RngSecretSource<R>` is generic over (ADR-0021). The RNG instance is **injected** (the Worker supplies a getrandom-backed CSPRNG; tests a seeded one), so no ambient randomness enters the core and it stays wasm32-safe — same discipline as the injected `Clock`. The **0.9** `RngCore` API (not the 0.10 `Rng` rename) is deliberate: it is what is already resolved in `Cargo.lock` (pulled transitively) and pairs with `rand_chacha` 0.9, so this adds **no new crate versions**. MIT OR Apache-2.0. Pinned spec 001 T07-shell-B; verified against the vendored source 2026-06-05 (lock = ground truth). |
| `rand_chacha` | 0.9.0 | **Dev-only.** Seeded `ChaCha20Rng` (`CryptoRng + SeedableRng`) for reproducible `RngSecretSource` host tests (the Worker injects a real CSPRNG instead — same trait, no code change). Already resolved in `Cargo.lock` (pairs with `rand_core` 0.9). Pure-Rust, MIT OR Apache-2.0. Pinned spec 001 T07-shell-B. |
| `insta` | 1.47.2 | Snapshot tests for serialization (feature `json` for `assert_json_snapshot!`). **Dev-only.** Apache-2.0. Pinned 2026-06-04 (spec 001 T02). |
| `static_assertions` | 1.1.0 | Compile-time proof that the tainted PII types (T02) **and** the `core/crypto` key/hash types — `HmacKey`/`PhoneLookupHash`/`CodeHash` (T03) — expose **no** `Debug`/`Display`/`Serialize` (P2/I3) via `assert_not_impl_any!`. **Dev-only.** MIT OR Apache-2.0. Pinned 2026-06-04 (spec 001 T02). |
| `base64` | 0.22.1 | **Dev-only.** Decodes the `fixtures/manifest/**` detached-signature + public-key fields in `core/crypto`'s `invariants.rs` (T03). MIT OR Apache-2.0. Pinned 2026-06-04 (spec 001 T03). |
| `chrono` or `time` | TODO (pick one — file ADR if both used) | Date/time |
| `uuid` | 1.23.2 | Stable IDs (backs `MemberId`). Feature `serde` **only** — deliberately **no** `v4`/`rng` (those pull `getrandom`, which breaks wasm32 and violates the no-ambient-randomness rule in `core::domain`). MIT OR Apache-2.0. Pinned 2026-06-04 (spec 001 T02). |
| `dryoc` | 0.8.0 | libsodium crypto, single-source across client+server (ADR-0014, P4): **Ed25519 detached-signature manifest verification** (`core::crypto`, T03 / AC10) plus the future per-Group **sealed-box/secretbox PII encryption** (I1 — lands with issuance, **spec 008**; *not* in T03). **Does NOT provide HMAC-SHA256** — its `crypto_auth` is HMAC-SHA512-256 and `crypto_hash` is SHA-512 only — so the I3 keyed phone/code hash uses `hmac`+`sha2` (ADR-0018) while dryoc stays the sole *signature* impl. **Pure-Rust, wasm32-compatible** (Workers + browser). Transitively (not feature-gated) pulls `rand` → `getrandom 0.4`; on wasm32 the `getrandom` `wasm_js` backend is enabled in `core/crypto/Cargo.toml` purely to compile — T03 uses **zero** randomness (deterministic verify only). Chosen over `sodiumoxide` (deprecated, C-FFI, no wasm). MIT. **MSRV 1.89.** 0.8.0 is the latest *published* release on crates.io (2026-05-15); the 0.9.0 → 0.8.0 pin correction (T01) was re-checked at T03 against the registry — still latest, no bump. |
| `hmac` | 0.13.0 | **HMAC-SHA256** keyed hash backing the phone-lookup hash and the Onboarding/Recovery code-at-rest hash, with constant-time `verify_slice` (I3 / AC3 / R2). dryoc has no SHA-256, so RustCrypto provides this (ADR-0018); dryoc remains the sole Ed25519 signature impl. Pure-Rust, wasm32-safe, no `getrandom` (deterministic). MIT OR Apache-2.0. Pinned 2026-06-04 (spec 001 T03). |
| `sha2` | 0.11.0 | SHA-256 digest plugged into `hmac` for the HMAC-SHA256 above (ADR-0018). Pure-Rust, wasm32-safe, no `getrandom`. MIT OR Apache-2.0. Pinned 2026-06-04 (spec 001 T03). |
| `geo` / `geo-types` | TODO | Geometry primitives |
| `petgraph` | TODO | Graph algorithms for chain optimization |

**Forbidden in core:**
- `unwrap()` on non-test code (use `expect` with a message or proper error handling)
- `println!` / `dbg!` (use `tracing`)
- Direct `std::time::SystemTime` (use injected `Clock` trait for testability)
- Network access in `core::domain` (must be in `core::sync` or `core::server`)

---

## Apple (SwiftUI)

| Dependency | Source | Used for |
|---|---|---|
| BoundlessCore | UniFFI-generated XCFramework | Domain types and operations |
| swift-snapshot-testing | swiftpackageindex | Snapshot tests for views |
| swift-collections | Apple | Specialized collections |
| swift-async-algorithms | Apple | Async streams |
| (no Combine for app state) | — | Use `Observation` framework instead |
| (no third-party DI container) | — | Pass dependencies through view init |

**Required entitlements:**
- Push Notifications
- Critical Alerts (requires Apple review — apply early)
- App Groups (for Widget extensions sharing state with app)
- WidgetKit
- WatchKit
- HealthKit (no — explicitly NOT used)

**Forbidden:**
- `print(_:)` of any tainted type
- Force-unwrapping (`!`) of optionals in production code
- `UserDefaults` for PII (use Keychain)
- Background fetch for live tracking (use APNs Live Activities)

---

## Android (Jetpack Compose)

| Dependency | Group:Artifact | Used for |
|---|---|---|
| Jetpack Compose BOM | androidx.compose | UI |
| Compose Material 3 | androidx.compose.material3 | Components |
| Compose for Wear OS | androidx.wear.compose | Wear UI |
| Glance | androidx.glance:glance-appwidget | Home screen widgets |
| Hilt | com.google.dagger:hilt-android | DI |
| Kotlinx Coroutines | org.jetbrains.kotlinx:kotlinx-coroutines-android | Async |
| Kotlinx DateTime | org.jetbrains.kotlinx:kotlinx-datetime | Dates |
| BoundlessCore | UniFFI-generated AAR | Domain |
| Paparazzi | app.cash.paparazzi | Snapshot tests |
| Turbine | app.cash.turbine | Flow testing |

**Forbidden:**
- `Log.d` / `Log.i` / etc. of tainted types
- `LiveData` (use `StateFlow`)
- RxJava
- Singleton state outside Hilt graph

---

## Web (Admin — SvelteKit)

| Dependency | Used for |
|---|---|
| SvelteKit | Framework |
| TypeScript (strict) | Type safety |
| Tailwind 4 | Styling |
| Radix Svelte (or melt-ui) | A11y primitives |
| TanStack Table | Tables |
| TanStack Query | Server state |
| Zod | Schema validation |
| Playwright | E2E tests |
| Vitest | Unit tests |
| axe-core | A11y CI lint |
| `@simplewebauthn/server` 13.x | Admin WebAuthn (passkey) Relying-Party verification on the Cloudflare edge — WebCrypto-based, runs in the Workers runtime (MIT). Challenges held in KV (5-min TTL). Chosen over a native `webauthn-rs` sidecar (which can't run in Workers wasm). See ADR-0017. Verified 2026-06-04 via docs-researcher. |

**Forbidden:**
- `localStorage` for PII (use server-side session)
- Inline event handlers without keyboard equivalents
- `dangerouslySetInnerHTML` / `{@html}` without sanitization audit

---

## Edge / Server (Cloudflare Workers + Rust)

| Dependency | Used for |
|---|---|
| `workers-rs` | Cloudflare Workers Rust SDK |
| `worker` crate | Bindings to DOs, KV, R2, Queues, Hyperdrive |
| `axum` (server option) | If running supplementary services outside Workers |
| `tokio-postgres` | 0.7 (lock = **0.7.17**, feature `with-uuid-1`). Postgres via Hyperdrive — the Worker drives it over a `worker::Socket` from `hyperdrive.connect()`. **Replaces `sqlx`**, which cannot run in the Workers wasm runtime (ADR-0019). MIT OR Apache-2.0. Used by `boundless-server-store` (`server/store/`, spec 001 T07-shell slice A); the wasm/Socket wiring + pooler-safe `query_raw` are T07-shell-B. `SystemTime`↔`timestamptz`, `bytea`, `text[]` are built-in; `with-uuid-1` maps `uuid::Uuid`↔`uuid`. Verified 2026-06-05 via docs-researcher; lock = ground truth. |
| ~~`sqlx`~~ | **Dropped (ADR-0019)** — does not compile/run on `wasm32-unknown-unknown` in the Workers runtime, so it is not on the Worker→Postgres path. Migrations stay plain reversible `NNNN_*.{up,down}.sql` (T06), applied out of band (CI `psql`); **not** `sqlx::migrate!`. |
| `tokio` | 1.52 (lock = **1.52.3**). **Test-only** in `boundless-server-store` (`rt-multi-thread`/`macros`/`net`/`time`/`sync`) — drives the `tokio-postgres` connection + spawns concurrent tasks in the real-Postgres integration tests. Pinned 2026-06-05 (spec 001 T07-shell slice A). |
| `serde_json` | JSON. **Dev-only** in the `boundless-worker` root crate (lock = **1.0.150**, same as core): parses the `fixtures/compat/**` request fixtures in the T08 N-2 compat replay harness (`server/tests/compat/`, `ac9_auth_endpoints_nminus2`). The Worker runtime's own JSON (request/response bodies) lands with the deployable shell (T07-shell-B / T08-shell). Pinned 2026-06-05 (spec 001 T08). |
| `boundless-auth` / `boundless-domain` | **Dev-only** path deps of `boundless-worker` (server root): the T08 compat harness replays fixture client versions through the core version policy (`evaluate_version` / `minimum_supported`) and `AppVersion` (P4 — the support-window decision lives in the core, not the harness). Added spec 001 T08. |
| `tower` / `tower-http` | If using Axum |
| `tracing` + `tracing-subscriber` | Structured logging |
| `opentelemetry` | Tracing |

**Cloudflare bindings used:**
- Durable Objects (one per Group for matching state + WebSocket hub)
- Hyperdrive (Postgres connection pooling)
- Queues (push fanout)
- Workflows (multi-step reassignment)
- KV (translation catalogs)
- Secrets Store (APNs key, FCM, DB creds)
- R2 (Logpush destination)
- Analytics Engine (non-PII metrics)
- Access (admin SSO)
- Turnstile (bot protection)
- Email Workers (admin invites)

**Forbidden in Workers:**
- Long-running CPU work (>30s limit; use Workflows for long flows)
- Plaintext PII in KV or R2 (encrypt before writing)
- WebSocket without Hibernation API
- Logging request bodies that contain PII

---

## Database (Neon Postgres)

| Extension | Used for |
|---|---|
| PostGIS | Nearest-neighbor queries |
| pgcrypto | Server-side crypto helpers |
| pg_stat_statements | Observability |

**Schema conventions:**
- Every PII column ends in `_encrypted bytea`.
- Every row has `created_at`, `updated_at`, `created_by` (admin ID for audit).
- Row-level security on every PII-bearing table.
- Migrations in `server/migrations/`, numbered.

---

## API contracts

| Format | Used for | Source of truth |
|---|---|---|
| OpenAPI 3.1 | HTTP API | `api/openapi.yaml` |
| Protocol Buffers (proto3) | WebSocket messages | `api/boundless.proto` |
| JSON Schema | Config files | `api/schemas/` |

**Client generation:**
- Swift: `swift-openapi-generator` + protoc-gen-swift
- Kotlin: openapi-generator (kotlin) + protoc-gen-kotlin
- TypeScript: openapi-typescript + ts-proto
- Rust: progenitor (or hand-rolled in core)

**Generated files live in `api/generated/<lang>/` and are NEVER edited by hand.**

---

## Translation pipeline

| Tool | Used for |
|---|---|
| ICU MessageFormat | Format syntax |
| Apple String Catalogs (`.xcstrings`) | Apple platforms |
| Android `strings.xml` (with ICU) | Android |
| FormatJS / `@formatjs/cli` | Web |
| Weblate (self-hosted or weblate.org) | Translator workflow |

---

## CI / Build

| Tool | Used for |
|---|---|
| GitHub Actions | CI orchestration |
| macOS runner | Apple builds, snapshot tests |
| Ubuntu runner | Rust, Android, web, Cloudflare deploy |
| Fastlane | iOS deploy automation |
| Gradle Managed Devices | Android UI tests |
| Wrangler | Cloudflare Workers deploy |
