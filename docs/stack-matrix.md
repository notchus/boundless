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
| Kotlin | 2.0.21 (Android; pinned at the bring-up — the Paparazzi 1.3.5-tested Kotlin, ground truth) | `android/gradle/libs.versions.toml` |
| TypeScript | 6.0.3 (strict; `web/` pinned exact 2026-06-05, spec 001 T09 — lock = ground truth; the SvelteKit app at T15 may widen the tree) | `package.json` + `tsconfig.json` |
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
| `uniffi` | 0.31.1 | Swift/Kotlin binding generation. **Apple/Android tooling ONLY — never on the wasm path:** activated solely by `core/ffi-swift` (spec 001 T10-shell, Swift leg → BoundlessKit; `core/ffi-kotlin` later). The wasm-safe core crates stay uniffi-free; `core/ffi-swift` mirrors their enums with `#[derive(uniffi::Enum)]` + exhaustive `From` conversions (a compile-checked parity guard, **not** a hand-rolled duplicate — ADR-0022). Lib `crate-type = ["lib","staticlib","cdylib"]`; the `uniffi-bindgen` CLI is a `[[bin]]` behind a `bindgen` feature (`uniffi/cli`, host-only — never compiled into the iOS `.a`). MPL-2.0. Latest stable, verified 2026-06-05 via docs-researcher against crates.io (lock = ground truth). Pinned spec 001 T10-shell. |
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
| BoundlessKit | UniFFI-generated XCFramework (uniffi 0.31.1) | The Rust core's onboarding state machine across the FFI. Built from `core/ffi-swift` by `scripts/build-boundlesskit.sh` → `apple/BoundlessKit/`; the xcframework + generated Swift are **build artifacts** (git-ignored, reproducible — not the committed `api/generated/**` wire bindings). Spec 001 T10-shell (Swift leg). |
| swift-snapshot-testing | 1.19.2 | Snapshot tests for SwiftUI views — the four required a11y variants (default / largest Dynamic Type / dark / RTL) via `.image(perceptualPrecision:layout:.device(config:.iPhone13),traits:)` (a11y bar / AC11). **First used spec 001 T11** (`apple/BoundlessRider`, Rider onboarding UI); a **test-only** dependency of that SwiftPM package (not BoundlessKit). Pulls swift-custom-dump 1.6.0 / swift-syntax 603.0.1 / xctest-dynamic-overlay 1.9.0 transitively (all pointfreeco/swiftlang — allow-list-clean, I8). Exact-pinned in `apple/BoundlessRider/Package.resolved` (lock = ground truth). MIT. Verified via docs-researcher 2026-06-05 + an on-simulator record/verify probe. **Also reused by `apple/BoundlessDriver` (`DriverShared`) at spec 001 T12** (Driver onboarding UI), which depends on `RiderShared` for the shared onboarding kit (same exact pin in `apple/BoundlessDriver/Package.resolved`); no new dependency. |
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

> **Version pins set at the Android bring-up (spec 001, 2026-06-06).** The constraint is
> **Paparazzi**: its latest *stable* is **1.3.5** (2.0.0 is alpha-only), and Paparazzi 1.3.5's own
> catalog pairs **AGP 8.4.2 / Kotlin 2.0.21 / Compose 1.7.5 / Material3 1.3.1** — so this is the
> *proven-Paparazzi-green* set, pinned one AGP major behind latest (AGP 9 removed `BaseExtension`,
> which Paparazzi 1.3.x needs). P1 + the a11y bar MANDATE the ×4 Paparazzi snapshots (T13/T14), so
> Paparazzi must work. AGP 8.4.2 caps `compileSdk` at 34 — exactly the API Paparazzi 1.3.5's
> layoutlib renders. The Gradle lockfiles under `android/` are the enforced truth (`gradle/libs.versions.toml`).

| Dependency | Group:Artifact | Version | Used for |
|---|---|---|---|
| Android Gradle Plugin | com.android.tools.build:gradle | 8.4.2 | Android build (Paparazzi-compatible; ground truth = Paparazzi 1.3.5 catalog) |
| Gradle (wrapper) | — | 8.7 | Build tool (AGP 8.4 min = 8.6; JDK 21-compatible) |
| Kotlin | org.jetbrains.kotlin | 2.0.21 | Language + Compose Compiler plugin (`org.jetbrains.kotlin.plugin.compose`) |
| Compose (ui / foundation) | androidx.compose.ui / .foundation | 1.7.5 | UI (explicit pins = Paparazzi tested set; BOM adoption is a T13/T14 option) |
| Compose Material 3 | androidx.compose.material3 | 1.3.1 | Components |
| BoundlessCore (`:core-bridge`) | UniFFI-generated AAR | from core/ffi-kotlin (uniffi 0.31.1) | Domain/auth state machine across the FFI (P4). Built by `scripts/build-corebridge.sh`: cargo cdylib → uniffi-bindgen Kotlin → cargo-ndk 4-ABI `.so`. Git-ignored build artifacts (reproducible; tracked via `core/**` in the drift gate), like the Swift BoundlessKit. ADR-0022. |
| JNA | net.java.dev.jna:jna | 5.17.0 | UniFFI 0.31.1 Kotlin runtime (`@aar` on-device; plain jar for host-JVM smoke test) |
| Paparazzi | app.cash.paparazzi | 1.3.5 | Snapshot tests (JVM/layoutlib API 34; ×4 a11y variants land with T13/T14) |
| JUnit4 | junit:junit | 4.13.2 | Test runner for the FFI smoke + Paparazzi tests |
| Compose for Wear OS | androidx.wear.compose | TODO | Wear UI (later spec) |
| Glance | androidx.glance:glance-appwidget | TODO | Home screen widgets (later spec) |
| Hilt | com.google.dagger:hilt-android | TODO | DI (added with T13/T14) |
| Kotlinx Coroutines | org.jetbrains.kotlinx:kotlinx-coroutines-android | TODO | Async (added with T13/T14) |
| Kotlinx DateTime | org.jetbrains.kotlinx:kotlinx-datetime | TODO | Dates |
| Turbine | app.cash.turbine | TODO | Flow testing (added with T13/T14) |

**Forbidden:**
- `Log.d` / `Log.i` / etc. of tainted types
- `LiveData` (use `StateFlow`)
- RxJava
- Singleton state outside Hilt graph

---

## Web (Admin — SvelteKit)

| Dependency | Used for |
|---|---|
| `@sveltejs/kit` 2.63.0 | Framework. **Scaffolded + first used spec 001 T15** (admin onboarding UI). devDep, exact-pinned (lock = ground truth). MIT. |
| `svelte` 5.56.1 | Svelte 5 (runes: `$state`/`$props`/`$derived`). Pinned T15. MIT. |
| `vite` 8.0.16 | Build/dev server (`vite dev` powers the Playwright `webServer`). Pinned T15. MIT. |
| `@sveltejs/vite-plugin-svelte` 7.1.2 | Svelte↔Vite integration. Pinned T15. MIT. |
| `@sveltejs/adapter-node` 5.5.4 | Build adapter for the **buildable + locally-testable** slice (always builds/previews, no `wrangler`). The Cloudflare adapter (`@sveltejs/adapter-cloudflare`) swap + `wrangler` deploy is the T15-shell (DEFERRED). Pinned T15. MIT. |
| `svelte-check` 4.6.0 | `.svelte`-aware typecheck (`pnpm typecheck` = `svelte-kit sync && svelte-check`). Pinned T15. MIT. |
| TypeScript (strict) | Type safety. `typescript` 6.0.3 (pinned T09). |
| `tailwindcss` 4.3.0 + `@tailwindcss/vite` 4.3.0 | Styling. v4 integrates via the `@tailwindcss/vite` plugin (no PostCSS); single `@import "tailwindcss"` in `src/app.css`; logical-property utilities (`ps-*`/`pe-*`) for RTL; `dark:` via `prefers-color-scheme`; `focus-visible:ring-*` + `sr-only` for the a11y bar. Verified via docs-researcher; pinned T15. MIT. |
| `@simplewebauthn/browser` 13.3.0 | Browser WebAuthn ceremony (`startRegistration`/`startAuthentication`, v13 `{ optionsJSON }` shape) — pairs with `@simplewebauthn/server` 13.3.1 (browser pkg latest in the 13.3 line is 13.3.0). **Statically imported** (SSR-safe: no top-level browser globals) so the ceremony call stays inside the user-activation window. Consumed T15. MIT. |
| `intl-messageformat` 11.2.8 | Runtime ICU MessageFormat (FormatJS) for `{adminName}`-style catalog copy (`src/lib/i18n`); instance-cached per (locale, key); SSR-safe. RTL direction via `Intl.Locale().getTextInfo()`. Consumed T15. BSD-3-Clause. |
| Radix Svelte / melt-ui | A11y primitives — **deferred to spec 008** (admin dashboard tables/dialogs/menus). T15's four button/status screens use semantic HTML only (a11y-bar: semantic HTML first), so no primitives lib is pulled yet. |
| TanStack Table | Tables (spec 008) |
| TanStack Query | Server state (spec 008) |
| Zod | Schema validation |
| `@playwright/test` 1.60.0 | E2E tests. **First used spec 001 T09** for the AC20 admin-WebAuthn ceremony via Chromium's CDP **virtual authenticator** (`WebAuthn.addVirtualAuthenticator`/`setUserVerified`) on a secure-context `http://localhost` page → real attestation/assertion bytes through the real verifier. Chromium-only. Browser fetched in CI via `playwright install chromium`. devDep, exact-pinned (lock = ground truth). MIT/Apache-2.0. |
| `vitest` 4.1.8 | Unit/integration tests. **First used spec 001 T09** for the WebAuthn verification module's pure legs (AC16 invite TTL/consume, KV challenge one-time-use, options policy, multi-cred/recovery, error-code registry parity). devDep, exact-pinned. MIT. |
| `@types/node` 25.9.1 | Node typings for the Vitest/Playwright harness + the edge module. devDep, exact-pinned (T09). MIT. |
| `yaml` 2.9.0 | YAML parser. **First used spec 001 T10** in the AC7 contract-freeze test (`web/tests/contract/api-contract.test.ts`) — parses the frozen `api/openapi.yaml` and asserts `client_min_version`+`client_recommended_version` are required on every `/api/auth/*` response. The web tier is the openapi-typescript consumer, so the OpenAPI contract check lives here (the proto leg is a dep-free Rust test in `core/sync`). Built-in TS types, ESM-clean (verified via docs-researcher). devDep, exact-pinned (lock = ground truth). ISC. |
| `@axe-core/playwright` 4.11.3 (+ peer `axe-core` 4.11.4) | A11y CI lint (AC11b). **Consumed spec 001 T15**: `AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']).analyze()` asserts zero violations on every admin onboarding route × {default, dark, RTL}. devDep, exact-pinned. MPL-2.0 / Apache-2.0. |
| `@simplewebauthn/server` 13.3.1 | Admin WebAuthn (passkey) Relying-Party verification on the Cloudflare edge — WebCrypto-based, runs in the Workers runtime (MIT). Challenges held in KV (5-min TTL). Chosen over a native `webauthn-rs` sidecar (which can't run in Workers wasm). See ADR-0017. **Consumed spec 001 T09** (`web/src/lib/server/webauthn`): v13 shapes verified via docs-researcher (`requireUserVerification` defaults true; `registrationInfo.credential = {id, publicKey: Uint8Array, counter, transports}`; helpers at `@simplewebauthn/server/helpers`). Verified 2026-06-04/05 via docs-researcher; lock = ground truth. |

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

> **Contracts FROZEN spec 001 T10** (`api/openapi.yaml` + `api/boundless.proto`; AC7 enforced by
> `web/tests/contract/api-contract.test.ts` + `core/sync/tests/proto_contract.rs`). The generator
> toolchains above (`buf`/`protoc`, `swift-openapi-generator`, `openapi-generator`, `uniffi-bindgen`)
> are **not yet installed** here, so the **actual codegen is deferred to T10-shell** — wired per
> target alongside the consuming UI tasks (T11–T15). See `DEFERRED.md` → "API contracts / codegen (T10)".

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
| Gradle Managed Devices | Android instrumented UI tests (later; the bring-up's Paparazzi is JVM-side, no emulator) |
| cargo-ndk | 4.1.2 — cross-compiles `core/ffi-kotlin` to the 4 Android ABI `.so`s for the `:core-bridge` AAR (`scripts/build-corebridge.sh`). Pinned at the Android bring-up. |
| Android SDK / NDK | cmdline-tools `latest` (20.0) + platform 34 + build-tools 34.0.0 + **NDK 28.2.13676358**. CI installs via `android-actions/setup-android@v3` + `sdkmanager`; the `android` job is GitHub-only. |
| Wrangler | Cloudflare Workers deploy |
