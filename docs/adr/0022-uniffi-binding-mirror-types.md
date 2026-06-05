# ADR-0022: UniFFI binding crates use mirror types + exhaustive conversions (core stays uniffi-free)

- **Status:** Accepted
- **Date:** 2026-06-05
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0001 (Rust core is the source of truth; bindings generated, never hand-rolled); P4 (no hand-rolled type duplicates); spec 001 plan, tasks T10 (T10-shell, Swift leg → BoundlessKit), T11–T14 (the native UIs)
- **Resolves:** how `core/ffi-swift` (and, later, `core/ffi-kotlin`) expose the core's types/functions across the UniFFI boundary **without** adding a `uniffi` dependency to the wasm-safe core crates.

## Context

The Apple/Android clients are **generated** from the Rust core via UniFFI (P4 / ADR-0001); the
`core/ffi-swift` crate produces the `BoundlessKit` XCFramework consumed by `apple/`. To generate
Swift, UniFFI needs the exported types/functions annotated with its proc-macros
(`#[uniffi::export]`, `#[derive(uniffi::Enum)]`, `uniffi::setup_scaffolding!()`).

Two hard constraints collide:

1. **The core crates must stay `uniffi`-free.** `boundless-domain`, `boundless-auth`,
   `boundless-crypto`, and `boundless-server-core` all compile to `wasm32-unknown-unknown` (the
   Cloudflare Worker + browser paths). `uniffi` (and its transitive deps) **does not build for
   `wasm32-unknown-unknown`** — verified: `cargo build --target wasm32-unknown-unknown -p
   boundless-ffi-swift` fails to compile `uniffi_core`. Adding `uniffi` to any core crate would
   break the wasm build and violate the "no ambient host tooling in core" discipline.

2. **UniFFI's cross-crate re-export needs the home crate to already be a UniFFI crate.** The
   `uniffi::use_remote_type!` mechanism (the only surviving cross-crate macro in 0.29+) requires the
   type's *home* crate to export it via UniFFI — exactly what constraint (1) forbids. (Verified
   against the uniffi 0.31.1 docs via docs-researcher, 2026-06-05.)

So the core cannot be annotated, and the bindings crate cannot forward un-annotated core types.

## Decision

`core/ffi-swift` (and later `core/ffi-kotlin`) **mirror** the client-relevant core enums/records
with their own `#[derive(uniffi::Enum)]` / `#[derive(uniffi::Record)]` types, and bridge each to its
core counterpart with **exhaustive `From` conversions in both directions**. The exported
`#[uniffi::export]` free functions are thin wrappers that convert in, call the core, and convert out.

The exhaustive `match` in each conversion is the **parity guard**: if a core variant is added,
removed, or renamed, the bindings crate **fails to compile** until the conversion is updated. Paired
with per-variant round-trip unit tests (mirror ⇄ core for every variant), this makes drift between
the generated Swift/Kotlin and the core a compile-or-test failure — *stronger* than a runtime check.

This is therefore **not** the "hand-rolled type duplicate" P4 forbids: P4 bans clients re-implementing
core logic and bans *hand-written client-side* type copies that can silently disagree. Here (a) the
mirrors are the *input to codegen*, not hand-written Swift/Kotlin; (b) the generated Swift/Kotlin is
never hand-edited; and (c) the mirror cannot silently diverge — the compiler enforces variant parity
and the conversions carry no logic (the core remains the sole decision-maker, P4). The alternative
(`uniffi` in core) is simply infeasible (constraint 1).

### Scope of the exported surface

Only the **client-relevant** surface is mirrored — the `boundless_auth::state` onboarding state
machine (`OnboardingState`, `OnboardingEvent`, `LaunchDecision`, `SignInResult`, `BindResult`,
`Role`) and its free functions (`launch`, `on_event`, `is_terminal`, `allows_offline_overlay`,
`reauth_state_for`, `should_flag_notifications_off`). Clients **render** these states; they receive
server-side *outcomes* (code/refresh/version evaluation, sessions, device-token invalidation) over
the HTTP/WS API (T07) and feed them in as events — those server decisions are **not** exported. No
tainted/PII type and no secret crosses the boundary (P2).

## Consequences

**Positive**
- Core stays wasm-safe and tooling-free; uniffi is confined to the `ffi-*` crates (off the wasm path).
- Variant drift is a compile error; conversion fidelity is unit-tested per variant; the generated
  Swift is exercised on the iOS simulator (the BoundlessKit smoke test).
- The pattern is identical for `core/ffi-kotlin` (T13/T14) — one decision covers both platforms.

**Negative / costs**
- The mirror enums restate the core variants in the `ffi-*` crate. Mitigated: it's the codegen input,
  compiler-checked for exhaustiveness, and round-trip tested — mechanical, not logic.
- Adding a core variant means updating the mirror + conversions (the compiler points at exactly where).

**Build artifacts** — the XCFramework and the generated Swift wrapper are **reproducible build
outputs** of `core/ffi-swift` (which is itself tracked by the binding-drift gate via `core/**`), so
they are git-ignored and produced by `scripts/build-boundlesskit.sh`, not committed — distinct from
the committed, drift-gated `api/generated/**` wire bindings. (Reversible if toolchain-free
consumption is later wanted.)

## Alternatives considered

- **`uniffi` in the core crates** — infeasible: breaks `wasm32-unknown-unknown` (constraint 1).
- **`uniffi::use_remote_type!`** — infeasible: requires the home crate to be a UniFFI crate (constraint 2).
- **A UDL file describing the core types** — same wasm/coupling problem and a second source of truth to
  keep in sync; the proc-macro mirror is compiler-checked, a UDL is not.
- **Hand-writing the Swift/Kotlin types** — the very thing P4/ADR-0001 forbid.
