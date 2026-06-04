# ADR-0001: Single Shared Rust Core for Domain Types and Business Logic

- **Status:** Accepted
- **Date:** 2026-05-27
- **Author:** Boundless founder
- **Deciders:** Boundless founder, with input from Claude during architecture planning

## Context

Boundless ships to:
- iOS, iPadOS, watchOS, macOS, visionOS (Apple)
- Android phone, Wear OS, Glance widgets (Google)
- SvelteKit admin web (browser)
- Cloudflare Workers + Durable Objects (edge/server)

Any business logic that exists in multiple places will eventually diverge. The matching algorithm, the privacy invariants, the chain construction, the encryption boundaries, the API request/response shapes — if these live in seven codebases, they will drift, and drift in this domain produces *privacy bugs* (the kind we cannot ship). We need a single source of truth for the *intelligent* parts of the system, with platforms acting as thin presentation layers.

Constraint: it must work natively on every target, with no performance compromise on mobile, and with first-class accessibility on every platform (which rules out cross-platform UI frameworks — see ADR-0002).

## Decision

Implement the Boundless domain layer once, in **Rust**, as a workspace under `core/`. Generate native bindings:

- **Swift/iOS family:** UniFFI → XCFramework, distributed as a Swift Package (`BoundlessKit`).
- **Kotlin/Android:** UniFFI → AAR, consumed via `core-bridge` Kotlin module.
- **Cloudflare Workers/edge:** `workers-rs` — the same Rust code compiles directly to WebAssembly and runs at the edge.
- **TypeScript (admin web):** `wasm-bindgen` for validation logic only (heavy logic stays in Workers, called from the web client).

Shared types (`Rider`, `Driver`, `Group`, `Chain`, `OptOut`, `Gathering`, `Address`, `PhoneNumber`, `DeviceToken`, `MatchingContext`, etc.) are defined once in `core/domain/`. Hand-rolled platform-side duplicates of these types are a CI failure.

## Considered alternatives

### Option A — Polyglot (status quo for most apps)

Write Swift, Kotlin, TypeScript, and Rust independently with handwritten translation between them.

**Pros:**
- No FFI complexity.
- Each language gets idiomatic code without binding constraints.
- No cross-platform build pipeline.

**Cons:**
- Privacy invariants must be re-implemented and re-tested on every platform.
- High risk of subtle divergence (e.g., a chain ordering tie broken differently on iOS vs the server).
- Cryptography must be audited four times.
- Matching algorithm changes require 4× the code review effort.
- The matching algorithm at the edge (Workers) would still need a fifth implementation.

### Option B — Kotlin Multiplatform (KMP) for shared logic

Use KMP to share business logic between Android, iOS (via Kotlin/Native), and web (via Kotlin/JS).

**Pros:**
- One language for client logic.
- Mature build tooling.
- Good Android-side ergonomics.

**Cons:**
- iOS interop produces Objective-C-flavored APIs that feel awkward in SwiftUI.
- Server side (Cloudflare Workers) doesn't run JVM bytecode — would need a separate JVM-on-Workers strategy or a separate server implementation, defeating the purpose.
- Kotlin/JS is not a strong fit for the web; Kotlin/Wasm is still emerging.
- Encryption story relies on `kotlinx.crypto` or platform calls — less mature than Rust's `dryoc` / `sodiumoxide`.

### Option C — TypeScript everywhere (Node on server, React Native on mobile, etc.)

Skip native UI entirely; share TS code between server and mobile via React Native.

**Pros:**
- Largest ecosystem.
- Single language end-to-end.

**Cons:**
- React Native accessibility lags native (deal-breaker per P1 — see constitution).
- Performance margins on older iPhones used by elderly users.
- No path to Wear OS / watchOS as first-class native experiences.

### Option D — C/C++ core

C or C++ for the shared core, bindings to each platform.

**Pros:**
- Mature ABI story.
- Smallest binary footprint.

**Cons:**
- Memory-safety hazards in code that processes addresses, phone numbers, and crypto keys. Not acceptable for a privacy-first app.
- Higher friction on async / actor-pattern matching engine.

### Option E (chosen) — Rust core, native UI per platform

**Pros:**
- Memory safety + performance.
- Compiles to native binaries on every target *and* to Wasm for the edge — same code runs everywhere intelligent logic lives.
- Excellent crypto libraries (`dryoc`, `sodiumoxide`).
- UniFFI generates idiomatic Swift and Kotlin bindings.
- `workers-rs` is first-party Cloudflare-supported.
- Type system catches a class of bugs (no `null`, exhaustive matching).

**Cons:**
- Rust learning curve for contributors.
- UniFFI imposes constraints on shared types (no traits with generics across the boundary, etc.).
- Build pipeline has multiple stages (Rust → bindings → platform integration).
- Some platforms get less idiomatic APIs than hand-written (mitigated by a thin per-platform "bridge" module).

## Consequences

### Positive

- **Privacy invariants are tested once, enforced everywhere.** A change to address encryption is a single PR with one test suite to update.
- **Matching algorithm exists in exactly one place** — including at the edge in Workers.
- **Cross-platform consistency is structural, not procedural.**
- **The `platform-parity` subagent has a meaningful job** — it can compare generated bindings to the Rust truth.

### Negative / costs

- **Bindings must be regenerated** on every change to shared types. A `make generate-bindings` target and a CI check enforce this.
- **The build is more complex.** Mitigation: each platform's developer experience is unaffected — they just `import BoundlessKit` or `import dev.boundless.core` like any other dependency.
- **Onboarding contributors who don't know Rust** is harder. Mitigation: most contributors will work in the platform layer (Swift/Kotlin/TS), not the core. Core changes go through extra review.
- **Debugging crosses the FFI boundary** — error messages can be cryptic. Mitigation: structured error types in Rust with stable error codes (constitution P12).

### Neutral / follow-ups

- ADR-0002 documents the native-UI-everywhere decision that follows from this one.
- ADR-0003 documents that the same Rust core runs at the Cloudflare edge — a key compounding benefit of this choice.
- The wasm-bindgen path for admin web is limited initially; most admin-side logic calls the Worker API rather than running locally. May expand if useful.
- visionOS support comes for free via the iPad SwiftUI codebase.

## Compliance

- **Constitution change:** None — this is consistent with P4 ("The Rust core is the source of truth"), which was authored anticipating this decision.
- **Stack matrix:** `docs/stack-matrix.md` lists the Rust crates used by `core/`.
- **Migration plan:** N/A — this is the founding decision.

## References

- [UniFFI documentation](https://mozilla.github.io/uniffi-rs/)
- [workers-rs documentation](https://developers.cloudflare.com/workers/languages/rust/)
- [wasm-bindgen documentation](https://rustwasm.github.io/docs/wasm-bindgen/)
- Constitution P4
- ADR-0002 (Native UI on every platform) — follows from this decision
- ADR-0003 (Cloudflare edge) — compounds with this decision
