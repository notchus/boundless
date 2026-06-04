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
| Rust | TODO (use stable, latest at project init; pin in `rust-toolchain.toml`) | `rust-toolchain.toml` |
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
| `serde` + `serde_json` | TODO | Domain type (de)serialization |
| `uniffi` | TODO (latest stable) | Swift/Kotlin binding generation |
| `wasm-bindgen` | TODO | wasm target for admin web (limited) |
| `tokio` | TODO | Async runtime (server side only) |
| `proptest` | TODO | Property-based tests for matching |
| `insta` | TODO | Snapshot tests for serialization |
| `chrono` or `time` | TODO (pick one — file ADR if both used) | Date/time |
| `uuid` | TODO | Stable IDs |
| `sodiumoxide` / `dryoc` | TODO (pick one — libsodium binding) | PII encryption (sealed boxes) |
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
| `sqlx` | Postgres via Hyperdrive |
| `serde_json` | JSON |
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
