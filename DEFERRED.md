# Boundless — Deferred Work

> A living checklist of things **decided but not yet done**. The point is that
> nothing falls through the cracks between sessions.
>
> When a decision is made that can't be acted on right now, it goes here with a
> **WHEN** trigger — *not* as a `// TODO` in code (the pre-commit hook rejects
> those) and *not* left to memory.
>
> **At the start of any session:** skim this file. If an item's WHEN trigger has
> arrived, do it or surface it. When you finish an item, check it off and note
> the date. When you defer something new, add it here under the right theme.

---

## Licensing

- [ ] **App Store additional-permission exception (AGPLv3 §7)** — add as a
      `LICENSE-EXCEPTION` file.
  - **WHEN:** before preparing the first iOS build.
  - **Why:** AGPL/GPL conflicts with Apple's EULA ("non-transferable,
    device-limited" terms). As sole copyright holder I can grant the exception
    (Signal's model).

- [ ] **DCO or lightweight CLA for outside contributors** — so licensing stays
      manageable as the project takes contributions.
  - **WHEN:** before accepting the first external pull request.

- [x] **Write ADR-0013** recording the AGPL-3.0 license decision and the three
      options weighed:
      AGPL-everywhere + exception / AGPL-server + Apache-clients / plain GPL.
  - **WHEN:** now-ish, via `/adr license`.
  - **DONE:** 2026-06-04 — see `docs/adr/0013-license.md` (Status: Accepted).

---

## Apple

- [ ] **Critical Alerts entitlement** — SUBMITTED and pending Apple review
      (1–3 weeks). Bundle ID: `app.boundless.rider`.
  - **WHEN:** watch for Apple's email; respond promptly to any follow-up
    questions.

- [ ] **Register the Driver app Bundle ID:** `app.boundless.driver`.
  - **WHEN:** when starting the Driver app.

- [ ] **Generate APNs `.p8` key** — note the Key ID + Team ID, store in
      Cloudflare Secrets Store.
  - **WHEN:** before implementing push notifications.

---

## Cloudflare / Infra

- [ ] **Create FCM service account JSON** for Android push; store in Cloudflare
      Secrets Store.
  - **WHEN:** before implementing Android push.

- [ ] **Store Cloudflare API token in GitHub Actions secrets** for CI deploys.
  - **WHEN:** setting up the deploy workflow.
  - **Note:** Not needed for local MCP — that uses OAuth.

- [x] **Re-confirm the network allow-list (AC13/I8) as the web dep tree grows.** At T01
      the web tier IS now scanned: pinning `@simplewebauthn/server` produced a committed
      `web/pnpm-lock.yaml`, and `scripts/check-network-allowlist.sh` scans it (currently
      clean — no trackers). When T15 builds the SvelteKit app and `pnpm install` expands
      that lock with the full dep tree (SvelteKit, Tailwind, Vitest, Playwright, axe-core,
      …), re-run/confirm the allow-list still passes and tighten patterns if needed.
  - **DONE:** 2026-06-05 (T15). The full SvelteKit + Tailwind 4 + svelte-check + @axe-core/playwright
    + @simplewebauthn/browser + intl-messageformat tree was installed; `scripts/check-network-allowlist.sh`
    is **clean across 5 lock files** (no trackers). The **final** AC13 sweep across all platforms is T16.

- **Cloudflare API MCP is authorized READ-ONLY by design.** Infra mutations go
  through Wrangler (human/CI gate), not the agent. If a task needs MCP write
  access, re-auth with Custom scopes for just that task, then revert. Never
  grant standing full access.

- [ ] **Fill in the TODO version numbers in `docs/stack-matrix.md`** with the
      actual installed versions.
  - **WHEN:** as each part of the stack gets initialized.
  - **Progress:** Rust toolchain filled (1.95.0) at spec 001 T01. `core/domain` deps
    filled at spec 001 T02 from the lock: `serde` 1.0.228 (+ `serde_core`), `serde_json`
    1.0.150, `uuid` 1.23.2, `insta` 1.47.2, `static_assertions` 1.1.0. `core/crypto` deps
    filled at spec 001 **T03**: `hmac` 0.13.0, `sha2` 0.11.0, `base64` 0.22.1 (dev), and
    the `getrandom` 0.4.2 wasm32 shim (`wasm_js`). `proptest` 1.11.0 (dev) filled at spec 001
    **T04** (first property tests, `core/auth`). Swift, Kotlin, TypeScript, Xcode, Android
    Studio, pnpm, and the remaining Rust deps (`uniffi`, `tokio`, `chrono`/`time`, `geo`,
    `petgraph`, …) remain TODO until those parts are initialized. Postgres **engine** pinned at **18**
    (Database section) at the 16→18 bump (2026-06-08; CI + local Docker + Neon origin parity — proven on
    real PG 18.4 by the migration + `boundless-server-store` suites).

- [ ] **Re-pin `dryoc` to 0.9.0 (or the then-latest)** once it is *published* to
      crates.io. At T01 the stack-matrix's `dryoc 0.9.0` was found to be unpublished
      (0.9.0 exists only on the dryoc `main` branch); the pin was corrected to the
      latest published release **0.8.0** (MIT, MSRV 1.89, pure-Rust/wasm32 — same
      properties). 0.8.0 is fully sufficient; this is a "keep current" follow-up, not a
      blocker.
  - **WHEN:** when implementing `core::crypto` (T03), check crates.io for a newer
    published dryoc and bump if available; update `docs/stack-matrix.md` to match the lock.
  - **Checked 2026-06-04 (T03):** 0.8.0 is still the latest *published* release (lib.rs:
    released 2026-05-15; docs.rs has nothing newer). **No bump.** Re-check at the next
    crypto task. NB: dryoc 0.8.0 hard-depends (un-feature-gated) on `rand`→`getrandom 0.4`,
    handled via the wasm32 `wasm_js` shim — see the Crypto register below and ADR-0018.

---

## Spec-Driven tooling

- [x] **Spec Kit `/speckit.*` commands were not installed upstream.** Instead added
      local command shims (`.claude/commands/speckit.{plan,tasks,implement}.md`) that
      drive the same constitution-aware, subagent-based flow as the custom commands.
  - **DONE:** 2026-06-04. If full GitHub Spec Kit is later wanted, install via the
    `specify` CLI — but expect to reconcile its templates with the custom constitution
    wiring and the existing `.claude/commands/` set.

---

## Auth / Onboarding (spec 001 plan deferrals)

- [x] **(1) `auth_refresh_rotation_replay_detected`** — a replayed pre-rotation refresh
      credential is rejected and kills the session family (the sole control behind
      ADR-0016's no-forced-expiry decision).
  - **DONE:** 2026-06-04 — shipped in **T05** (`core/auth/tests/session.rs`). The
    refresh-rotation control is recorded under **I4** in `docs/privacy-invariants.md` (it
    underwrites I4 rather than being a new numbered invariant — the doc is PII-scoped).

- [ ] **Two remaining new privacy-invariant tests — implement WITH their code** (P9: the
      implementing test ships in the same PR):
      (2) extend the I12 forgetting property test to the new auth artifacts (phone
      hash + ciphertext, device tokens, sessions/refresh, outstanding Onboarding /
      Recovery codes, admin WebAuthn creds);
      (3) a named delete-leg device-token invalidation test, distinct from
      `i4_tokens_invalidated_on_reonboarding` and `…_on_logout`.
  - **WHEN:** implementing `core::deletion` (the account-deletion flow is out of scope for
    spec 001 — spec §Out of scope).

- [ ] **Critical Alerts capability-upgrade path** — onboarding currently requests
      *standard* notifications (interim, spec 001 OQ6). Once the Critical Alerts
      entitlement lands, upgrade the rider doorbell path to Critical Alerts.
  - **WHEN:** Apple approves the Critical Alerts entitlement (see **Apple** section).

- [ ] **Admin WebAuthn verification host** — if the decision lands on a native Rust
      sidecar (`webauthn-rs` can't run in Workers wasm — `openssl-sys` C-FFI), that
      adds one always-on service to deploy/monitor; if it lands on edge-TS, no infra
      is added.
  - **WHEN:** resolved by the in-flight edge-TS verification → ADR-0017.

---

## Crypto / `core::crypto` (spec 001 T03 — out-of-scope register)

> T03 implemented exactly: HMAC-SHA256 phone-lookup + Onboarding/Recovery code-at-rest
> hashing with constant-time verify (I3/AC3), and Ed25519 manifest verification + the
> ADR-0014 tiered fallback (AC10). Everything below was **deliberately left out** of that
> slice (per the approved plan + the "keep track of everything out of scope" instruction).
> Each carries a WHEN trigger so it is picked up at the right task.

- [ ] **Per-Group field-level PII encryption (I1).** dryoc-based encryption of `Address` (and
      any field-level PII) at rest. The `core/crypto` doc named this as the crate's eventual
      scope; spec 001 T03 did **not** implement it (addresses are entered at admin *issuance*,
      not on the onboarded device). **Now governed by ADR-0025** — the `sealed-box`-vs-`secretbox`
      hedge is resolved to **secretbox** (symmetric, XSalsa20-Poly1305; sealed boxes reserved for
      I9's live tracker).
  - **WHEN:** **spec 008 T02** (this is the load-bearing slice): `core/crypto/src/secretbox.rs`
    (`encrypt_field`/`decrypt_field`, `GroupKey`/`Kek` wrap/unwrap, zeroized) + the tainted
    `Address`/`MemberName` + the `core/crypto/tests/invariants.rs::i1_addresses_encrypted`/
    `i1_name_encrypted` tests. The per-Group key + KEK (Secrets Store) **wiring** + `Address`
    persistence land across T03 (columns) / T04 (bootstrap) / T07 (DB) / T09 (Worker KEK binding).

- [ ] **Onboarding/Recovery code generation, TTL, rate-limit, single-use + regenerate-
      invalidates-prior.** T03 ships only the at-rest **hash** primitive (`*_code_hash` /
      `*_code_matches`). The lifecycle/validation logic is separate.
  - **WHEN:** **T04** (`core::auth` code logic, server-time semantics) + **T07** (server
    enforcement). Tests: `prop_onboarding_code_single_use_ttl_ratelimit`, `ac17_*`.

- [ ] **Phone-number normalization (E.164) before hashing.** `core::crypto` hashes the exact
      bytes it is handed; the caller must normalize so the *same* human phone always yields
      the same `phone_lookup_hash`. Not done in T03 (no normalization in the crypto layer).
  - **WHEN:** **T04** (`core::auth`) / **T07** (server sign-in lookup path).

- [ ] **`HmacKey` provisioning + rotation.** The per-instance secret is *passed in*
      (`HmacKey::from_bytes`); loading it from Cloudflare **Secrets Store** and any rotation
      policy are infra/server, not core (forbidden-patterns: no hardcoded secrets).
  - **WHEN:** **T07** (server) + infra (Secrets Store wiring).

- [ ] **Manifest-mint Worker (server-side signing) + signing-key management + bundled
      public key.** T03 implements client-side **verification**, the canonical-bytes contract
      (`canonical_manifest_bytes` = sorted-key compact JSON), and a reproducible test vector.
      Still out of scope: the production Ed25519 *signing* Worker, the signing key in Secrets
      Store, quarterly rotation, and embedding the trusted **public** key in each client
      binary. **The signer MUST canonicalize identically** to `canonical_manifest_bytes`
      (sorted-key compact JSON) **and keep the manifest integer-only — no floats** (floats have
      no canonical cross-impl serialization), or signatures won't verify. A float-valued
      manifest field is a breaking change to the signing contract.
  - **WHEN:** a server/infra task for ADR-0014's manifest pipeline (not in spec 001's task
    list — surface when the manifest service is built).

- [ ] **Key/secret zeroization on drop.** `HmacKey` has no `Debug`/`Display` (can't be
      logged), but its bytes are not zeroized on drop. Marginal here (it lives the process
      lifetime, loaded once from Secrets Store), deferred as hardening (consider `zeroize`).
  - **WHEN:** a crypto-hardening pass before GA.

- [ ] **Workspace RNG-backend policy.** dryoc transitively pulls `rand`→`getrandom 0.4`;
      T03 enabled `getrandom`'s `wasm_js` backend on wasm32 *only to compile* (it uses zero
      randomness). Decide workspace-wide whether to keep `wasm_js` (for the server's eventual
      real RNG — code/nonce generation) or install a custom *erroring* backend until then, to
      keep "no ambient randomness" literally enforced. See ADR-0018.
  - **WHEN:** **T07** (server), when server-side randomness is first genuinely needed.
  - [x] **CI gate (sec-audit F6) — DONE 2026-06-08.** The "no ambient randomness in `core`" invariant is
    now mechanically gated, not manual. Gate = `scripts/check-wasm-no-getrandom.sh` (+ meta-test
    `scripts/test-wasm-no-getrandom.sh`, the `test-binding-drift.sh` pattern), wired as a step in the
    existing `rust-core` CI job (the wasm32 target is installed there — zero new toolchain wiring). For
    `boundless-server-core` + `boundless-crypto` it (a) builds for `wasm32-unknown-unknown` (fail-closed
    on any non-wasm-safe dep) and (b) scans the crate's **forward** `-e no-dev` wasm tree, failing if any
    `getrandom v0.3.x` node is present, with a positive control that the allowed `0.4.x` (`wasm_js`-shim)
    edge IS present (non-vacuity). The earlier draft used a version-pinned `-i getrandom@0.3.4` query;
    the **reviewer + security-auditor both caught** that a `cargo update` to e.g. `0.3.10` would make the
    pinned query "not match" → silently pass — so the shipped gate is **version-agnostic within the
    major** (forward tree + `grep '(^| )getrandom v0\.3\.'`), closing that false-pass. Locally proven:
    gate green, meta-test green (indented-0.3.x / 0.3.10 patch-bump / 0.4.x-only / positive-control
    present+absent / word-boundary), and a transient negative check (point the detector at the present
    0.4.x → FAIL → revert). 0.3.x reaches the graph **only** via dev-deps (proptest), never the non-dev
    wasm path; the CI *job* run is GitHub-only (not locally gated), but the scripts themselves are fully
    locally verifiable. The broader **Workspace RNG-backend policy** decision above (keep `wasm_js` vs an
    erroring backend) stays open — out of scope for this gate.
  - [ ] **Extend the gate's `CRATES` coverage (sec-audit F1, reviewer).** The gate audits
    `boundless-server-core` (which transitively covers domain/auth/crypto) + `boundless-crypto`, per the
    F6 names. **Not** covered: `boundless-ffi-wasm` (the literal browser `cdylib` artifact),
    `boundless-sync`, `boundless-logging` — and the `server/` workspace's deployed `boundless-worker`
    (sec-audit F2). All are **dependency-free / out of scope today** (no getrandom possible), so this is a
    latent gap, not a live break. Add each to `CRATES` when it gains a getrandom edge — **`ffi-wasm` at
    T10** is the highest-value (it ships to the browser and gains wasm-bindgen/validation deps there). NB
    the positive control assumes the audited crate legitimately carries the `0.4.x` shim, so a
    randomness-free crate can't simply be added — decide per-crate (carry-the-shim → add as-is; truly
    randomness-free → assert *no* getrandom at all instead). For `server/`, note ADR-0021 *allows* the
    Worker a real getrandom-backed CSPRNG, so the invariant there is narrower (the *core crates compiled
    into* the Worker stay injection-only, the Worker itself may not).
  - **WHEN:** **T10** (ffi-wasm) / when sync·logging·worker gain deps / a CI-hardening pass.

- [ ] **`core/crypto/tests/invariants.rs` enumerating *every* privacy invariant (P9 goal).**
      T03 covers **I3** (+ the AC10 manifest tiers). I1/I2/etc. get their named tests when
      their primitives exist.
  - **WHEN:** as each invariant's primitive lands (I1 → spec 008; I2 → matching, spec 004+).

---

## `core::auth` (spec 001 T04 — out-of-scope register)

> T04 implemented exactly the **device-side pure logic**: the `OnboardingState` machine,
> `AppVersion` vs `client_min_version` comparison (O4) + the N-2 window (O1), and the
> Onboarding/Recovery **code lifecycle decision** (single-use / TTL / rate-limit /
> regenerate-invalidates-prior, AC17; driver-only recovery, AC19) on an injected `Clock`.
> Everything below was **deliberately left out** of that slice (per the approved plan + the
> "keep track of everything out of scope" instruction). Each carries a WHEN trigger.

- [ ] **Server-time enforcement of the code lifecycle.** T04 ships the pure *decision*
      (`evaluate_onboarding_code`/`evaluate_recovery_code`); the server is the authority that
      feeds it **server time**, persists the `onboarding_codes`/`recovery_codes` rows, runs
      the rate-limit **window** bookkeeping (5 attempts / 15 min — T04 only models the lock
      decision `recent_attempts ≥ max`), wires **Turnstile**, and emits the lock admin alert.
      **Carry-forward from the T04 security review (must land in T07):** (a) **atomic
      consume-on-accept** — the core renders `Accepted`, but the server must mark
      `consumed`/`superseded` transactionally with the accept, or two concurrent presentations
      of one live code could both see `Accepted`; (b) **sign-in response-timing/shape parity**
      for matched-vs-unmatched phone (no existence leak — the constant-time hash compare is
      necessary but the *response* must not branch on existence either); (c) the production
      `Clock` impl **must** supply server time (a device clock re-enters the threat model
      otherwise). The driver-only recovery role gate is now enforced **inside** the core
      (`evaluate_recovery_code` takes `role`), so it is no longer a caller contract — but T07
      should still short-circuit via `recovery_available_for` before loading a challenge.
  - **WHEN:** **T07** (member-auth endpoints + DO) and **T06** (the code tables).

- [ ] **N-2 support policy across a *major* version bump.** `minimum_supported(current, n)`
      deliberately floors within the current major (it does not roll back across a major) —
      so the moment the server ships `2.0.0`, every `1.x` client falls below the window. This
      is a defensible reading of O1 ("N-2 *minor* versions"), but it tensions with P13/O1's
      "a rider on a 4-month-old build still works" across a major boundary. Make the
      across-major support policy an explicit **ADR** decision before the first `2.0.0`.
  - **WHEN:** before shipping a `2.0.0` server (or when the compat harness, AC9, first spans
    a major bump).

- [x] **Sessions, silent refresh-token rotation, device-token binding (ADR-0016 D2, I4).**
      Indefinite sessions, rotation with replay/lineage detection, the
      `(member_id, platform, app_version)` device-token binding + invalidation triggers, and
      the new **`auth_refresh_rotation_replay_detected`** privacy-invariant test were the
      sibling `core::auth` slice, explicitly *not* in T04.
  - **DONE:** 2026-06-04 — shipped in **T05** (see the T05 out-of-scope register below for
    what T05 itself deferred onward).

- [ ] **UniFFI export of the `core::auth` surface.** T04's types are UniFFI-shaped (plain
      enums/structs, no exotic generics) but carry **no `#[uniffi::export]`/UDL** yet — codegen
      to Swift/Kotlin is the T10 contract-freeze (matches T02/T03). The injected `Clock` is a
      Rust-side server concern, not part of the client UniFFI surface (clients don't validate
      codes — that's server-side).
  - **WHEN:** **T10** (API contracts + generated bindings).

- [ ] **`chrono`-vs-`time` crate decision.** T04 uses a homegrown `UnixSeconds(i64)` (UTC) +
      a `Clock` trait for the only time need so far (TTL `<` comparisons), keeping the
      stack-matrix `chrono`/`time` TODO ("pick one — file ADR if both used") correctly
      deferred. Pick the crate when real wall-clock/formatting/parsing is first needed.
  - **WHEN:** **T07** (server, real server-time) or the first locale-aware time display.

- [ ] **Promote `Clock`/`UnixSeconds` to a shared crate** if `core::sync`/`core::server`/
      matching need the same time abstraction (today it lives in `core::auth`).
  - **WHEN:** when a second crate needs an injected clock.

---

## `core::auth` (spec 001 T05 — out-of-scope register)

> T05 implemented exactly the **device-side pure session logic**: the indefinite-session model
> (`Session::is_live` time-independent, `needs_refresh` on an injected `Clock`), the
> refresh-rotation **policy** with replay detection (`evaluate_refresh` /
> `RefreshVerdict::ReplayDetectedKillFamily` → family revoked), the `(member_id, platform,
> app_version)` device-binding tuple (`DeviceBinding`), the exhaustive admin-mediated
> invalidation triggers (`invalidation_for`, `reonboarding_invalidation`, AC4/AC18), and the
> §10-F secure-store contract (`required_refresh_store`). Added `SessionFamilyId` to
> `core::domain`. Everything below was **deliberately left out** of that slice.

- [ ] **Server-side refresh persistence + lineage classification.** T05 ships the pure
      *policy* (`evaluate_refresh` over a `RefreshPresentation`). The server (T07) owns: the
      Postgres `sessions` rotation lineage chain, the refresh credential's **at-rest HMAC
      hashing**, and the **DB lookup + constant-time compare** that classifies a presented
      credential as `Current`/`Superseded`/`Unknown` (the input to the policy). The
      replay→kill-family verdict must be persisted **atomically** with the family revoke.
      **Carry-forward from the T05 security + test review (must land in T07):**
      (a) **rate-limit** `/api/auth/refresh` on `Rejected`/`Unknown` outcomes per source
      (mirror the R4 code rate-limit) and keep the rejected response **timing/shape-identical**
      to a revoked-family reject, so it leaks no lineage-existence signal (sec-audit F1);
      (b) **atomic rotate-vs-replay** — a concurrent presentation of the current credential and
      a replay of a superseded one must resolve to a revoked family, never a second valid
      rotation (TOCTOU; integration test `concurrent_rotate_and_replay_resolves_to_revoked`);
      (c) **classification correctness** — a credential rotated N times ago must classify as
      `Superseded` (so replay *kills*), not `Unknown` (which would merely reject);
      (d) **family-kill persistence** — assert `sessions.revoked_at` is written and the
      *legitimate current* credential is rejected on its next refresh (the AC18 promise the
      core test only asserts at the model level);
      (e) `AUTH_DEVICE_TOKEN_INVALIDATED` is **silent** (no catalog key) — assert it is
      logged/audited but never surfaced to the client.
  - **WHEN:** **T07** (member-auth endpoints + DO) and **T06** (the `sessions`/`device_tokens`
    tables).

- [ ] **Multi-device-per-member policy for the `PriorDevice` invalidation scope.** `core::auth`
      `reonboarding_invalidation` compares a single `prior` binding to the `new` one and returns
      `PriorDevice` scope; the `AllForMember` scope (revoke/logout/delete) already covers
      multi-device correctly. ADR-0016 D2 does not bound device count per member. T07 must
      decide whether a member may hold multiple concurrent device bindings and, if so, enumerate
      **all** prior bindings on re-onboard/revoke rather than a single `prior` (else a stale
      token could survive). Test (T07): `reonboarding_with_multiple_prior_bindings_invalidates_all`
      (or assert the documented single-device constraint). (sec-audit F5.)
  - **WHEN:** **T07** (member-auth endpoints + DO).

- [ ] **Access-token issuance/signing + the ~15-min wall-clock TTL.** T05 models only the
      access-token *expiry instant* (`Session::access_token_expires_at`) and the
      `needs_refresh` decision against an injected clock; minting/signing the token and
      supplying real server time are server concerns.
  - **WHEN:** **T07** (server, real server-time — ties into the `chrono`-vs-`time` pick above).

- [ ] **Actual push device-token registration (APNs/FCM).** T05 owns the binding *tuple* and
      its *invalidation policy*; registering/deregistering the real push token with
      APNs/FCM and persisting it is server + platform work.
  - **WHEN:** the Doorbell push spec (**007**) / **T07**.

- [ ] **UniFFI export of the session/device surface.** Like the T04 types, the new
      `Session`/`RefreshVerdict`/`DeviceBinding`/… are UniFFI-shaped but carry no
      `#[uniffi::export]`/UDL yet — codegen to Swift/Kotlin is the **T10** contract-freeze.
  - **WHEN:** **T10** (API contracts + generated bindings).

- [ ] **`SecureStoreClass` wiring per platform (plan §10-F).** T05 ships the contract
      (`required_refresh_store` → Keychain / Keystore / httpOnly-Secure-SameSite cookie); the
      actual platform secure-store reads/writes of the `RefreshToken` are the UI tasks.
  - **WHEN:** **T11–T15** (the five UIs).

- [x] **P9 process: guarantee `core/<crate>/proptest-regressions/` is tracked in CI — DONE 2026-06-08.**
      The P9 "reproducible seeds checked into the repo" guarantee is now mechanical, not conventional.
      Did **both** the check *and* the `.gitkeep` the item offered: committed
      `core/auth/proptest-regressions/.gitkeep` + `core/server/proptest-regressions/.gitkeep` (the two
      crates declaring a `proptest` dev-dep — repo `.gitkeep` convention), gated by
      `scripts/check-proptest-regressions.sh` (+ meta-test `scripts/test-proptest-regressions.sh`, the
      `test-binding-drift.sh`/F6 pattern), wired as a step in the existing `rust-core` CI job. The gate
      **auto-discovers** proptest crates from `core/**/Cargo.toml` (section-aware awk: a `proptest` dep
      under a `[…dependencies]` table — inline OR dotted-section `[…dependencies.proptest]`, with header
      comments/whitespace tolerated — but NOT under the `[workspace.dependencies]` registry, which the
      root uses to register the version for members), so a future proptest crate is covered without
      editing the gate; per crate it asserts the `proptest-regressions/` dir is git-tracked AND a future
      seed is not gitignored; fail-closed + a zero-crates non-vacuity guard. Locally proven: gate green,
      meta-test green, **two transient negative checks bit** (untracked `.gitkeep` → FAIL; gitignored dir
      → FAIL) then reverted, binding-drift unchanged at 75 inputs (`.gitkeep` is not a drift input),
      allow-list clean. The discovery hardening (dotted-section + commented-header false-pass vectors)
      and the section-aware awk itself were **both flagged by the reviewer + security-auditor** (the
      naive `grep '^proptest'` first draft wrongly matched the workspace root) and closed in-slice with
      meta-test cases. Reviews: reviewer "fix H1 [stage the scripts] then ship" (done); security-auditor
      "ship it" (the dangerous `.gitkeep`-tracked-but-`*.txt`-ignored trap is closed). NB: the gate's own
      runtime non-vacuity guard only fires at **zero** crates — the *meta-test's* live-discovery
      assertion (auth+server found, root not) is what catches a *known*-crate drop, so that list must be
      kept current as proptest crates are added.

---

## Server / migrations (spec 001 T06 — out-of-scope register)

> T06 shipped exactly the **schema**: the 8 reversible migrations (`server/migrations/0001…0008`),
> a dependency-free static convention test (`server/tests/migrations.rs`), a self-skipping live
> psql apply/RLS/revert script (`scripts/test-migrations.sh`), and the CI wiring (server step
> `build`→`test`; a `postgres:16` `server-migrations` job). No endpoint logic, no row writes, no
> new Rust dependencies. The live cycle was verified against real `postgres:16` (apply → bytea +
> forced-RLS + isolation/deny/WITH-CHECK smoke → revert → clean teardown). Everything below was
> deliberately left out; each carries a WHEN trigger.

- [ ] **RLS GUC must be set per *request transaction* on the Hyperdrive/Worker connection.**
      Tenant isolation depends on `SET LOCAL app.current_group_id = '<group>'` inside each request's
      transaction (the resolver `current_group_id()` maps unset/empty → NULL → deny, so the failure
      mode is fail-*closed*). The trap is **pooled-connection reuse**: Hyperdrive pools physical
      connections, so a value set without `SET LOCAL` (or never reset) could carry a prior tenant
      into the next request. T07 must use `SET LOCAL` within the request txn (resets at COMMIT/
      ROLLBACK), or explicitly reset on checkout. **Highest-leverage carry-forward.** (reviewer M2 /
      sec-audit R1)
  - **WHEN:** **T07** (member-auth endpoints + DO connection layer).

- [ ] **The runtime DB role must be non-superuser and non-`BYPASSRLS`.** `FORCE ROW LEVEL SECURITY`
      covers the table owner, but a superuser / `BYPASSRLS` role bypasses RLS regardless. The
      Hyperdrive/Worker role must be a plain role. (sec-audit R3)
  - **WHEN:** **T07** / infra (DB role provisioning).

- [~] **Atomic supersede-then-insert for the four partial-unique indexes.** The schema *enforces*
      "at most one live row" via partial unique indexes on `onboarding_codes`/`recovery_codes`
      (one live code per member), `sessions` (one current credential per family), and
      `admin_invitations` (one live invite per admin). A regenerate/rotate that inserts the new row
      before superseding the prior in the **same transaction** will hit a unique violation — must be
      ordered supersede-then-insert atomically (the DB twin of T04's "atomic consume-on-accept"
      carry-forward). (reviewer / sec-audit R5-adjacent)
  - **Sessions/recovery DONE (T07-shell slice A, 2026-06-05):** `rotate_session` /
    `consume_and_rotate_recovery` in `PgAuthStore` (proven vs real `postgres:16`).
  - **`admin_invitations` DONE (T08, 2026-06-05):** `PgAuthStore::reissue_admin_invitation`
    supersedes-then-inserts under an admin-scoped `pg_advisory_xact_lock` (so concurrent re-issues
    serialize rather than racing the one-live index); proven by
    `server/store/tests/admin_invitations.rs::concurrent_reissue_keeps_exactly_one_live`.
  - **Onboarding-code regenerate (issuance):** the *consume*-on-bind is atomic (T07-core); the
    *regenerate*-invalidates-prior write is issuance-side. **WHEN:** **spec 008** (admin issuance).

- [ ] **`audit_log` table + admin-PII-read audit (I5).** This slice provides only `created_by`
      (write-side actor). The `audit_log` table and the `#[require_audit]` read-path obligation must
      exist before any endpoint returns `phone_encrypted` to an Admin. (sec-audit R9)
  - **WHEN:** **T07** / **spec 008** (admin member-management).

- [x] **~~sqlx `Migrator` + the `sqlx` pin~~ — SUPERSEDED by ADR-0019 (sqlx dropped).** Research at
      T07-shell slice A found **`sqlx` cannot run in the Workers wasm runtime**, so it is not on the
      Worker→Postgres path. The driver is **`tokio-postgres` over a Hyperdrive Socket** (ADR-0019).
      Migrations stay plain reversible `NNNN_*.{up,down}.sql` applied **out of band** (CI `psql` /
      `scripts/test-migrations.sh`; the store tests apply them via `batch_execute`) — there is no
      `sqlx::migrate!` and **no `sqlx` dependency**. `docs/stack-matrix.md` updated (sqlx row dropped;
      `tokio-postgres`/`tokio` added). Keep LF line endings regardless.
  - **DONE/decided:** 2026-06-05 (ADR-0019).

- [ ] **PostGIS / `pgcrypto` extensions + `address_encrypted` + per-Group key/KEK columns (I1).**
      Onboarding tables have no geometry and no address (crypto is core-owned, §10-H). Address
      persistence, the per-Group encryption key, and the KEK (Secrets Store) land with issuance.
  - **WHEN:** **spec 008** (admin issuance) — adds the `i1_addresses_encrypted` enforcement.

- [~] **Actual row writes** (group/member issuance, sign-in lookup, device-bind, refresh rotation,
      recovery re-bind, admin invite mint/consume) — the schema defines the columns; the writes are
      the endpoint slices.
  - **Session/code/member-read writes DONE:** T07-shell slice A (`PgAuthStore`).
  - **Admin invite MINT + pending-admin write DONE (T08, 2026-06-05):** `PgAuthStore`
    `create_pending_admin_with_invitation` (member role=`admin`, no phone, + invitation) and
    `reissue_admin_invitation`. The invite **consume** (on first WebAuthn registration) is **T09**.
  - **WHEN (remaining):** **spec 008** (group/member *issuance* + phone writes) / **T09** (invite consume).

---

## Server / core (spec 001 T07 — out-of-scope register)

> T07 shipped exactly **Layer A**: the pure, deterministic, wasm-safe member-auth orchestration
> engine in `core/server` (`boundless-server-core`) — the four `/api/auth/*` endpoints
> (`sign_in`/`bind_device`/`refresh`/`recovery_rebind`) + `record_notification_decision` +
> `note_session_invalidated` + the `GroupHubState` decision state + the PII-free `AdminAlert`s —
> behind port traits (`AuthStore`/`AdminAlertSink`/`SecretSource`) with in-memory stubs, composing
> T03/T04/T05. Plus the one in-scope crypto primitive (refresh-credential at-rest hash) and the
> E.164 `normalize_phone`. 48 server-core tests + 3 new crypto tests; **no new external deps**;
> wasm32-clean. The deployable **Worker shell (T07-shell)** was deliberately deferred (user
> decision 2026-06-04: "core engine only"). Everything below is its scope + the port contracts it
> must satisfy. Closes the server-logic legs of AC4/AC7(data)/AC8/AC14/AC15/AC17/AC18/AC19.

> **Slice A DONE (2026-06-05):** the **Postgres `AuthStore` adapter** (`boundless-server-store`,
> `server/store/`) is built + proven against real `postgres:16` — see the dedicated
> "Server / store (T07-shell slice A)" register below. Driver = **`tokio-postgres` over a Hyperdrive
> Socket** (sqlx dropped, ADR-0019). The rest of T07-shell is **T07-shell-B** below.

> **Async-port bridge DONE (2026-06-05, ADR-0020):** `core/server`'s store ports are now **`async` +
> fallible** (shared `StoreBackend::Error`); the device-token methods split into a separate
> **`DeviceStore`** port (its Postgres impl is blocked on spec-008 token encryption); **`PgAuthStore`
> now `impl`s `AuthStore`**; and `AuthService` is proven end-to-end against real `postgres:16`
> (`server/store/tests/service_pg.rs`). The 48 T07-core tests were adapted (host `pollster::block_on`).
> So the remaining **T07-shell-B** below is the *deployable Worker runtime only* (+ the `DeviceStore`
> Postgres impl, with encryption).

> **T07-shell-B slice 2 DONE (2026-06-05) — the host/real-PG-testable port impls + the access-token
> decision:** (1) **ADR-0021** resolves the plan §10-D OPEN access-token wire format → **opaque-random
> 32-byte bearer** verified by a constant-time keyed-HMAC store lookup (not EdDSA-JWT — it honors the
> time-independent, family-status-gated revocation model with **zero new key-mgmt infra**; decided via a
> 4-reader/4-judge analysis, 3–1). (2) **W2 boot guard** `boundless_server_store::ensure_least_privilege`
> (sec-audit's highest-impact item) — refuses if `current_user` is superuser/`BYPASSRLS`; proven both
> legs vs real `postgres:16`. (3) **Access-token at-rest hash primitive** `core::crypto::access_token_hash`
> /`access_token_matches` + `AccessTokenHash` (new domain tag `boundless:access-token:v1`; no
> `Debug`/`Display`/`Serialize`/`PartialEq`). (4) **Production `SecretSource`** `RngSecretSource<R: RngCore
> + CryptoRng>` in `core/server` — opaque tokens from an **injected** CSPRNG (core stays randomness-free +
> wasm32-safe; seeded `ChaCha20Rng` in tests). Pins: `rand_core` 0.9.5 (traits-only, no getrandom) prod,
> `rand_chacha` 0.9.0 dev — both already in the lock, **no new crate versions**. All host/real-PG tested;
> no Worker toolchain needed. The remaining T07-shell-B (Worker runtime + the access-token store
> column/verify lookup + `PgDeviceStore`) stays below.

- [~] **T07-shell-B — the deployable workers-rs Worker. SLICE 1 (toolchain bring-up + Worker
      skeleton) DONE 2026-06-07; the Postgres/deploy legs remain (slices 2+).**
  - **DONE — slice 1 (Worker skeleton, 2026-06-07):** Stood up the Worker toolchain (`cargo install
    worker-build` 0.8.3; `wrangler` 4.98.0 + `@cloudflare/vitest-pool-workers` 0.16.13 + `vitest`
    4.1.8 via pnpm — all crates.io/npm-reachable through the sandbox; **no Cloudflare account**) and a
    **real, miniflare-tested Worker skeleton** in `server/` (`boundless-worker`): the `#[event(fetch)]`
    + `worker::Router`, the `GroupHub` Durable Object, and the KV + Queues bindings, composing the
    **real core `AuthService::sign_in`** (P4) over a clearly-labelled **scaffold in-memory store**
    (`runtime/scaffold_store.rs`, to be deleted when `PgAuthStore` is wired). Routes: `GET /healthz`
    (version handshake AC7/O4 + a real KV `MANIFEST` read), `POST /api/auth/signin` (the frozen
    `api/openapi.yaml` `SignInResponse` wire shape — matched/not-on-file/below-min, no existence leak —
    + the below-min admin alert drained to the `ADMIN_ALERTS` Queue, §10-E), `POST /api/auth/bind-device`
    (forwarded to the `GroupHub` DO, which applies the core §10-E rate-limit window + persists a counter
    via `state.storage()`, AC17). **Crate is cfg-split** (`worker` is a `cfg(target_arch="wasm32")`
    dep; the runtime is wasm-only) so the native `store` + `compat` tests stay green; `default-members
    = ["."]` so `worker-build` (which builds default members) never drags the native `store` crate onto
    wasm. Proof: `worker-build --release` compiles a 24 kB wasm; **6 miniflare tests green** (no
    account); native `cargo clippy/test --workspace` + wasm clippy clean. Pins → `docs/stack-matrix.md`
    (Edge/Server section). CI: new `worker` job. **Contract defect flagged** (see the new bullet below).
  - **REMAINING — the PG/deploy legs (need a local Postgres + a Cloudflare account, or spec-008):**
    - **Replace the scaffold store with `PgAuthStore`-over-Hyperdrive.** Drive the already-built,
      `AuthStore`-implementing `PgAuthStore` over a `hyperdrive.connect()` `worker::Socket`. **Driver
      choice DECIDED → ADR-0024 (2026-06-08):** stay on the **published `tokio-postgres` 0.7.17** and
      issue every query via the **unnamed-statement `query_typed*` family** (`query_typed_one`/
      `query_typed_opt`/`query_typed`/`query_typed_raw`; `simple_query`/`batch_execute` for no-param/DDL)
      — **no fork**. (The blog-cited `unnamed-statement` *fork* is dead/404 — supply-chain risk on the
      auth path; the published 0.7.17 already exposes the API, whose own doc names "Cloudflare Workers
      with Hyperdrive." NB: Hyperdrive *added* named-statement support June 2024, but Cloudflare hedges
      for non-node drivers and a fresh-`Client`-per-request gains nothing from the named cache — unnamed
      is the prudent, account-free-decidable path. ADR-0019's Context "sharp edge" bullet wording
      ("pooler dislikes *unnamed*") is corrected in ADR-0024 — it's the *named/persistent* statements
      that break.) **Store-side prep DONE (slice "store wasm-prep", 2026-06-08):** the 9 `PgAuthStore`
      methods (+ `begin`'s RLS `set_config` + `ensure_least_privilege` + the advisory-lock SELECTs — 21
      call sites) were migrated to the unnamed `query_typed_one`/`query_typed_opt`/`execute_typed` family
      (each `$n`'s `Type` supplied inline), proven on **real PG18** (all 25 store tests green on the typed
      path — a wrong `Type` fails loudly there), and `boundless-server-store` is now **wasm32-buildable**:
      target-split `tokio-postgres` (wasm = `default-features=false` + `["with-uuid-1","js"]` — the
      `runtime`-gated `rand`-using `connect` module is excluded, and `js` wires the existing getrandom
      0.4.2 `wasm_js` backend, +1 lock line: postgres-protocol→getrandom edge, no new crate);
      `cargo build --target wasm32 -p boundless-server-store` green + a new CI step in `server-store`. The
      `tests/common` superuser scaffolding stays on the named path **by design** (direct, never-pooled
      connection — named is correct there; the production methods the Worker runs ARE the typed-path
      fidelity). **TRANSPORT DONE (2026-06-08, PgAuthStore-over-Hyperdrive slice).** `server/src/runtime/
      pg.rs`: `connect_pg` drives `PgAuthStore` over `env.hyperdrive("HYPERDRIVE")?.connect()? ->
      worker::Socket` → `tokio_postgres::Config::from_str(connection_string).connect_raw(socket, NoTls)`
      with the `Connection` future spawned via `worker::wasm_bindgen_futures::spawn_local` (workers-rs
      0.8.3 HAS a first-class Hyperdrive binding — the "needs a forked driver" note was wrong, corrected in
      `wrangler.toml`). Sign-in now runs the real `AuthService::sign_in` over `PgAuthStore` (+ an in-memory
      `DeviceStore` half mirroring `service_pg.rs`); `HMAC_KEY`/`GROUP_ID` load from Worker bindings.
      **Proven account-free** via `@cloudflare/vitest-pool-workers` (it DOES emulate the Hyperdrive
      binding) against the local PG18: `/readyz` `db:ok` (connect_raw + spawn_local + W2 guard in workerd)
      + sign-in `phone_not_on_file` over the real store/transport (`scripts/setup-worker-test-db.sh`
      provisions the non-superuser app role + migrations; the CI `worker` job gains a `postgres:18`
      service). **Remaining = the deploy legs only** (the `wrangler deploy` bullet below): the real
      Hyperdrive `id` (`wrangler hyperdrive create` against the user's Neon URL) + `wrangler secret put
      HMAC_KEY` + the real `GROUP_ID` + `wrangler deploy` — the human gate (Cloudflare MCP is read-only).
      **Deploy-prep DONE (2026-06-08, deploy-prep slice):** the one non-`wrangler` prerequisite (Neon
      DB provisioning) is now scripted + tested account-free — `scripts/provision-neon.sh` (non-destructive,
      idempotent: mints the locked-down `boundless_app` role, applies migrations if empty, grants least
      privilege, prints the app-role Hyperdrive connection string) + `scripts/test-provision-neon.sh` (the
      meta-test, see the store register below) — and the exact ordered `wrangler` sequence is the new
      **`docs/runbooks/deploy-worker.md`** (P12's first runbook). The deploy path was validated
      account-free: `wrangler deploy --dry-run` (bundle + all bindings resolve) and a **live `wrangler dev`
      + `scripts/smoke-deployed-edge.sh`** round-trip (real HTTP → Worker → local PG over the Hyperdrive
      socket: `/readyz db:ok`, sign-in `phone_not_on_file`, no credential leak). So the operator's remaining
      work is exactly: run `provision-neon.sh` once, then the runbook's `wrangler` commands.
    - **Neon-owner fix (2026-06-09, surfaced during the operator's first live run).** The provisioner +
      meta-test were corrected for the real Neon privilege boundary that the local-superuser meta-test had
      masked: Neon's `neondb_owner` is a `neon_superuser` member with `CREATEROLE`+`BYPASSRLS` but is **NOT a
      true superuser**, so `ALTER ROLE boundless_app … NOSUPERUSER NOBYPASSRLS` is **rejected** ("only roles
      with the SUPERUSER attribute may change the SUPERUSER attribute"). Fix: `provision-neon.sh` now creates
      the role with the safe **defaults** (NOSUPERUSER/NOBYPASSRLS are the defaults), re-asserts only LOGIN +
      password, and **verifies** `rolsuper=f / rolbypassrls=f / rolcanlogin=t` (fail-closed if drifted — only
      a superuser could fix a drifted privileged role, same posture as the W2 guard). It also gained a
      **pre-flight** (connectivity + a `CREATEROLE`-on-`current_user` check that rejects the limited
      `authenticator` role with a clear message) and a non-fatal **`-pooler` warning** (DDL wants Neon's
      DIRECT/unpooled endpoint). The **meta-test** now runs the provisioner **AS `neon_owner_sim`** — a
      CREATEROLE/NOSUPERUSER/BYPASSRLS role that mirrors `neondb_owner` (the superuser only bootstraps: mints
      the BYPASSRLS sim, transfers DB+`public` ownership to it, grants it `ADMIN OPTION` on `boundless_app`
      since **PG16+ requires CREATEROLE + ADMIN OPTION on the target role to ALTER it**, and seeds the two
      tenants). Proven both ways on real PG18: green as the non-super owner; **red with the bad `ALTER`**
      (reproduces the operator's exact error — the regression guard the old superuser-run test couldn't
      provide). `docs/runbooks/deploy-worker.md` gained a "Troubleshooting step 0" section for the three
      role/permission errors + the direct-endpoint guidance. (NB the user's own `neondb_owner` flow is
      unaffected by the PG16 admin-option rule — it *created* `boundless_app`, so it already holds admin on it.)
      A 3-lens review (reviewer ship / security + operator fix-then-ship) hardened the slice before commit:
      the **P2 owner-password leak** the old `${OWNER_URL%%@*}` log produced is fixed (log a redacted
      `scheme://host/db`, never userinfo) + guarded by a meta-test stderr no-leak assert; the `-pooler` check
      is now **fatal** (was a warning — a re-run skips migrations, so a pooled host could have ridden silently
      into `wrangler hyperdrive create`; `ALLOW_POOLER=1` overrides); the verify now also rejects
      **REPLICATION/CREATEROLE/CREATEDB** (REPLICATION can stream the WAL = all-tenant PII, bypassing RLS); the
      pre-flight accepts superuser-OR-CREATEROLE; the meta-test gained a **negative** proving the verify refuses
      a pre-existing BYPASSRLS role; and the runbook's ADMIN-option remedy now leads with DROP-and-recreate (the
      only Neon-runnable path — Neon has no superuser login, so the prior `GRANT … WITH ADMIN OPTION` advice was
      a dead end). **The runtime half of the verify gap — DONE 2026-06-10.** The Rust boot guard
      `boundless_server_store::ensure_least_privilege` (`server/store/src/lib.rs`) now also checks **`rolreplication`**
      (added `COALESCE((SELECT rolreplication FROM pg_roles WHERE rolname = current_user), false)` to the probe + a
      third `PrivilegeTooHigh("current_user has REPLICATION")` reject) — so a `boundless_app` that *drifts* to
      REPLICATION *after* provisioning is now refused at boot, not silently accepted (REPLICATION can stream the WAL =
      all-tenant PII, bypassing RLS, even on a NOSUPERUSER NOBYPASSRLS role). The guard now mirrors the three *direct*
      RLS-bypass attributes the provisioner verifies (superuser/BYPASSRLS/REPLICATION); it deliberately does **not**
      also reject CREATEROLE/CREATEDB (those are *escalation* vectors, enforced at provisioning, not a way to read
      another tenant's rows on the live connection — the boot guard's job is the narrower "refuse a connection that
      bypasses RLS *now*", documented in the fn). New store test leg
      `least_privilege.rs::ensure_least_privilege_rejects_replication_role` (mints a NOLOGIN NOSUPERUSER NOBYPASSRLS
      **REPLICATION** role, `SET ROLE`s into it, asserts the guard rejects it) — proven on real **PG18** (26 store
      tests green; a transient non-vacuity check confirmed the new leg bites — guard returns `Ok(())` without the
      check); clippy `-D warnings` / fmt / wasm32 build all green; no new dependency. **Out of scope (still
      deferred):** the sec-audit F5 **live deployed-edge cross-tenant isolation** smoke as the real `boundless_app`
      role (needs ≥2 seeded Groups → spec 008) — that proves RLS isolation end-to-end on the deployed Worker; this
      guard only proves the role *cannot bypass* RLS, the necessary precondition.
    - **FIRST LIVE DEPLOY DONE (2026-06-09).** The operator ran the full runbook end-to-end against their
      real Neon DB + Cloudflare account. Step 0 (Neon provisioning) was run by the agent directly on
      explicit user authorization (the one sanctioned DB mutation — `provision-neon.sh` against the real
      `neondb_owner` DIRECT endpoint: minted the locked-down `boundless_app`, applied the 8 migrations,
      granted least privilege, verified NOSUPERUSER/NOBYPASSRLS/NOREPLICATION/…); steps 1–7 were the
      operator's `wrangler` commands (`hyperdrive create` → `kv namespace create MANIFEST` → `queues
      create boundless-admin-alerts` → `secret put HMAC_KEY` → `deploy`), with the agent pasting the
      returned resource ids into `server/wrangler.toml`. **Two `wrangler.toml` changes the live deploy
      forced:** (a) `new_classes` → **`new_sqlite_classes`** for the `GroupHub` DO (SQLite-backed DOs are
      REQUIRED on the Workers Free plan + recommended on Paid; the `state.storage()` KV API is unchanged
      across backends) — a real correctness fix for ANY deployer, committed; (b) the real Hyperdrive +
      KV ids pasted over the `REPLACE_AT_DEPLOY_*` placeholders. Worker live at
      `https://boundless-worker.notchus.workers.dev`; **`scripts/smoke-deployed-edge.sh` GREEN against the
      live edge**: `/healthz` ok · **`/readyz db:ok`** (the W2 `ensure_least_privilege` guard passes LIVE
      → `boundless_app` is locked down and RLS is actually enforced in production over the real
      Hyperdrive→Neon path) · sign-in `phone_not_on_file` · no credential leak. **What this proves vs
      doesn't:** it proves the live connect + the least-privilege guard + the sign-in wire shape end to
      end; it does **not** yet prove (i) **cross-tenant RLS isolation** as the live role (no seeded
      multi-tenant data exists — needs ≥2 Groups → **spec 008** issuance, or a manual seed) nor (ii) the
      **golden-fixture contract-conformance replay** against the deployed Worker. **Follow-ups surfaced by
      the live deploy:** (1) **genericize the committed `wrangler.toml` resource ids back to
      `REPLACE_AT_DEPLOY_*` placeholders before the repo is made public** — they are account identifiers,
      not secrets, and committing them on the current private single-instance repo keeps the tree clean +
      redeploys frictionless, but an open-source repo should not carry one instance's ids (WHEN: before
      open-sourcing). (2) **`preview_urls = false`** in `server/wrangler.toml` — **DONE-in-config 2026-06-10
      (commit pending), takes effect at the next `wrangler deploy`.** Privacy: every deployed *version*
      otherwise gets its own public `<preview>-boundless-worker.<sub>.workers.dev` URL. Verified via
      docs-researcher/Cloudflare-docs: wrangler **≥ 4.44.0** defaults `preview_urls` to *match* the
      `workers_dev` setting (which is on → preview URLs on), so it MUST be set explicitly to `false`; the
      stable `workers.dev` route stays on (`workers_dev` unset) for the single canonical URL. `wrangler
      deploy --dry-run` accepts the key (account-free). **Remaining:** the operator's next `wrangler
      deploy` actually applies it (human gate) — until then the live Worker still serves per-version
      preview URLs. (3) the wrangler first-run telemetry notice printed despite
      `send_metrics = false` — benign (a one-time CLI notice; the setting still disables telemetry).
    - **Two review backstops tracked here** (3-lens review of the deploy-prep slice; both latent, no live
      break): (i) **`provision-neon.sh`'s `GRANT … ON ALL TABLES` is point-in-time** — when migration 0009+
      lands, a re-run on an already-migrated DB hits the "8/8 → skip" path and the new table is neither
      created nor granted (RLS still denies → fail-closed, not a privacy break). When 0009+ lands, either add
      `ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT … TO boundless_app` or bump the expected-table count
      (the twin of the `mig_count` "bump when 0009+ lands" note in `setup-worker-test-db.sh`). (ii) **the
      `smoke-deployed-edge.sh` no-leak grep** (`postgres(ql)?://|password|bypassrls`) catches a full
      connection-string leak but not a *bare* secret echo (just the HMAC key / a token) — fine today because
      the server maps every error to value-free strings (P2), so a secret can't reach the wire; fold the
      "assert the `db` field never contains a substring of the known HMAC_KEY/connection-string" check into
      the deployed-edge `/readyz` hardening item below when the smoke has those values.
    - **Access-token verify path (ADR-0021; mint side DONE slice 2):** add the `access_token_hash bytea`
      column + the per-request verify lookup (re-reads family status), folded into the request's
      group-scoped RLS txn or served from `GroupHub` DO in-memory state with **write-through/evict on
      revoke** (authoritative-on-revoke, not TTL). **WHEN:** with the PgAuthStore slice.
    - [x] **W2 boot-guard call site — DONE 2026-06-08 (PgAuthStore slice).** `build_service`
      (`server/src/runtime/mod.rs`) calls `ensure_least_privilege(&client)` after `connect_pg`, before any
      `PgAuthStore` query, and fails closed (generic 500); `/readyz` reports `db:"role_too_privileged"` on
      reject. The miniflare `db:ok` test proves the guard accepts the non-superuser app role in workerd.
      **Provisioning SCRIPTED + tested (2026-06-08, deploy-prep slice):** `scripts/provision-neon.sh`
      mints the non-superuser / non-`BYPASSRLS` `boundless_app` role (the guard rejects Neon's default
      `neondb_owner`, which has `BYPASSRLS`), and `scripts/test-provision-neon.sh` proves it account-free
      against local PG18 — the **account-free analog of the deployed-edge cross-tenant proof** (it acts as
      the *real* `boundless_app` LOGIN role over `public`, a strictly stronger RLS proof than the Rust
      harness's `SET ROLE`-on-superuser/per-test-schema `rls_isolates_reads_by_tenant`: role-locked · single
      table owner · least-privilege SQL both-false · RLS isolates cross-tenant · fail-closed). **Provision
      + deploy DONE 2026-06-09** (see "FIRST LIVE DEPLOY DONE" above): `provision-neon.sh` was run against
      the real Neon owner URL, and the live `/readyz db:ok` proves the guard accepts the provisioned
      `boundless_app` role **in production**. **Remaining (the single highest-impact pre-GA proof,
      sec-audit F5):** the **live deployed-edge cross-tenant *isolation* smoke** as that role — assert a
      request scoped to Group A cannot read Group B's rows on the deployed Worker. It needs ≥2 seeded
      Groups, which do not exist until issuance. **WHEN:** **spec 008** (issuance seeds multi-tenant data),
      or a one-off manual seed against the deployed edge before GA.
    - **Route the sign-in below-min alert dedup through the `GroupHub` DO** (reviewer H1, slice 1). The
      §10-E once-per-day dedup (`GroupHubState.should_alert`) lives in `AuthService.hub`, which
      `build_service()` re-creates **per request** — so on the sign-in path the dedup does not persist
      (every below-min sign-in re-enqueues a `BelowMinVersion` alert; harmless today, an R12 alert-flood in
      production). The bind path already persists via the DO; sign-in must too once `AuthService` is hosted
      in / fronted by the DO. **WHEN:** the DO-fronting (bind-device) slice — still per-request after the
      PgAuthStore slice.
    - [x] **Fail-closed guard so the scaffold store/key can never be silently deployed (security-auditor
      F1) — DONE 2026-06-08 (build-time leg).** The scaffold (the hardcoded `SCAFFOLD_HMAC_KEY = [0x7b; 32]`
      + one seeded demo member) is now gated behind a **non-default `scaffold` cargo feature** (chose the
      compile-time option over a boot-time dev-var check — strictly stronger: a release build that forgets it
      *fails to compile*). `server/src/lib.rs` gates `mod runtime` on `all(target_arch="wasm32",
      feature="scaffold")` and emits a `compile_error!` on `all(target_arch="wasm32", not(feature=
      "scaffold"))`; the **local/test** path opts in (`server/package.json` `build` = `worker-build --release
      --features scaffold`, used by `pnpm test` locally + the CI `worker` job), while the **deploy** path
      (`server/wrangler.toml` `[build]` = `worker-build --release`, what `wrangler deploy` runs) stays
      featureless → a deploy build **fails closed**. Mechanically enforced by `scripts/check-worker-deploy-
      guard.sh` (asserts a featureless wasm build fails AND fails *via* the compile_error sentinel;
      non-vacuity covers both "guard missing" and "broke for another reason"), wired as a step in the CI
      `worker` job (after `pnpm test`, reusing the warm release deps). Locally proven: `pnpm test` green (6
      miniflare tests), featureless build fails with the compile_error, gate green, **two transient
      non-vacuity checks bit** (sentinel-mismatch → "not via the guard"; build-forced-to-succeed → "guard
      missing"), native `cargo test/clippy/fmt --workspace` green (the wasm-gated guard never fires off-wasm),
      binding-drift unchanged (server/ not a drift input), allow-list clean (no dep change; `Cargo.lock`
      untouched). Reviews: `reviewer` + `security-auditor` both "ship it" (0 crit/high/med). The cargo
      feature unification escape-hatch was checked — no other crate enables `boundless-worker/scaffold`, so a
      featureless deploy build cannot unify it in. **F3 nit folded in (DONE):** `DEMO_PHONE` `+15551230000`
      → `+15555550100` (NANP reserved fictional `555-01XX`, consistent with `fixtures/compat/**`); no residual
      `+15551230000` anywhere.
    - [x] **REAL close — delete the scaffold + retire the guard — DONE 2026-06-08 (PgAuthStore slice).**
      Deleted `server/src/runtime/scaffold_store.rs` (the hardcoded `SCAFFOLD_HMAC_KEY` + seeded member) and
      `scripts/check-worker-deploy-guard.sh` + its CI step; retired the `scaffold` cargo feature + the
      `compile_error!` (`server/src/lib.rs` now gates `mod runtime` on plain `cfg(target_arch="wasm32")`;
      `package.json` / `wrangler.toml` build is featureless). A real store compiles featureless, so the
      deploy path no longer fails closed at compile time — **fail-closed moved to RUNTIME**: the W2 guard
      rejects a superuser/`BYPASSRLS` role, and a missing `HMAC_KEY`/`GROUP_ID`/`HYPERDRIVE` binding errors
      the request. No residual hardcoded auth key on the deploy path (security-auditor verified via `git
      grep`; the only `from_bytes([0x42;32])` is test-only in `store/tests/common`).
    - **`PgDeviceStore` (device-token persistence) + APNs/FCM registration.** Needs spec-008 device-token
      at-rest encryption (the in-memory `DeviceStore` stands in until then). **WHEN:** push spec 007 / issuance 008.
    - **Turnstile** (code-guess + refresh throttle) + the **per-source refresh-rejection 429** network
      enforcement (the `GroupHubState` counter exists; the Worker enforces). **WHEN:** with the PgAuthStore slice.
    - **The `boundless::logging::emit()` sink + no-raw-`tracing` lint + Logpush replay** (P2/I10) — also a
      T16-shell item; route `StoreError` + the invite-token URL segment through it. **Carry-forward
      (ADR-0023 / sec-audit F1, 2026-06-07):** now that the wire carries the plaintext `phone`, the Worker
      must keep the **inbound raw `String` phone** — `body.phone` in the window *before* `normalize_phone`
      tints it into a `PhoneNumber` — off the log path and never echo it in an error response (the current
      generic `"bad phone"`/`"internal"` strings are correct; keep them generic). Add a Worker integration
      test asserting sign-in/bind/recovery error responses for a malformed/unmatched phone contain **no
      substring of the submitted phone** (the testable form of ADR-0023's "hash, use, drop" obligation,
      which also covers the AC3 "drops/never-logged" leg the core constant-time test doesn't). **WHEN:** this same shell track.
    - **Real signed-manifest KV serving** (ADR-0014; the skeleton only reads a manifest index key). **WHEN:** the manifest-service spec.
    - **`wrangler deploy` DONE 2026-06-09** (first live deploy — see "FIRST LIVE DEPLOY DONE" above:
      Worker live at `boundless-worker.notchus.workers.dev`, `smoke-deployed-edge.sh` green). **Remaining
      = the live deployed-edge *contract-conformance* E2E** (replay the golden `fixtures/auth/*` against
      the deployed Worker through the real Hyperdrive pooler — the smoke checks `/healthz` + `/readyz` +
      one sign-in shape + no-leak, not the full fixture matrix). **WHEN:** the deploy-hardening pass.
    - **Rate-limit / Access-gate the `/readyz` DB probe** (reviewer M1 / sec-audit F2, PgAuthStore slice).
      `/readyz` opens a per-call Hyperdrive connection; an unauth caller can drive DB connects (a cheap
      pooler-amplification vector). `/healthz` is now dependency-free (liveness); `/readyz` carries the DB
      probe. Rate-limit it or put it behind Cloudflare Access, and add a deployed-edge assertion that the
      `db` field never contains a substring of the connection string. **WHEN:** the deploy-hardening pass.
    - **`PgService` drift gate** (reviewer M2, PgAuthStore slice). `server/src/runtime/pg.rs::PgService`
      is a verbatim copy of `server/store/tests/service_pg.rs::PgService` (real `PgAuthStore` + in-memory
      `DeviceStore` half), guarded only by a comment. Low real risk (the `AuthStore` half is pure
      pass-through; the in-memory device half is transitional). When `PgDeviceStore` lands (spec 008) the
      two diverge **by design** and this concern dissolves; until then, if a third consumer appears,
      extract the shared shape into a `boundless-server-store` module both import. **WHEN:** spec 008 /
      `PgDeviceStore` (or sooner if a third copy appears).
    - **Worker-layer coverage of the `signin_wire` `MemberMatched` branch** (reviewer L4, PgAuthStore
      slice). The slice dropped the member_matched miniflare assertion (the DB is unseeded; matched is
      proven in `service_pg.rs` + the web `oneOf` contract test), so `signin_wire`'s matched JSON shape
      (`next_step`/`manifest_pointer`) is no longer Worker-integration-tested — a key-name typo there would
      pass CI. Restore by seeding one member (a Rust-computed `phone_lookup_hash` matching the test
      `HMAC_KEY`) when the bind-device slice adds seeding for its bind tests, or add a wasm unit test of
      `signin_wire`. **WHEN:** the bind-device slice (which seeds the worker test DB).
    - **`wrangler.toml` committed-credential grep gate** (sec-audit F1, optional hardening). `wrangler.toml`
      carries a LOCAL TEST `localConnectionString` (`boundless_app:boundless_app@localhost`) — inert at
      deploy (deploy uses the real Hyperdrive `id`), consistent with the committed `postgres:postgres` in
      `store/tests/common`. Optional: extend the secret/allow-list scan to assert `wrangler.toml` never
      gains a real Hyperdrive `id` (UUID) or a non-`localhost` connection string, so a real Neon URL can't
      be committed here. **WHEN:** a CI-hardening pass / the deploy slice.

- [x] **OpenAPI auth-request ↔ I3 contract defect — RESOLVED 2026-06-07 (ADR-0023).** The frozen
      `api/openapi.yaml` carried a client-computed **`phone_lookup_hash`** on all three auth requests
      (`SignInRequest`/`BindDeviceRequest`/`RecoveryRebindRequest`), but I3 keys that hash with a
      **per-instance server secret** (`core/crypto/src/hashing.rs`) a client cannot hold, and
      `core::server::sign_in` already takes the raw (normalized) **phone** and hashes server-side — so the
      contract was internally inconsistent with I3 (platform-parity F1/F2; flagged in `server/src/runtime/mod.rs`).
  - **DONE:** **Option A** (amend the OpenAPI requests → `phone`, E.164 over TLS; server normalizes +
    HMAC-hashes + drops the plaintext, never logged P2 / only the keyed hash + `phone_encrypted` stored,
    I3/I5) — see **ADR-0023**. Rejected Option B (client keyless pre-hash: unsalted low-entropy phone hash
    is brute-forceable + forces client-side normalization, violating P4 single-source `normalize_phone`).
    Changed: the 3 OpenAPI request schemas + the signin path/field descriptions; the 3 illustrative
    `fixtures/compat/**` bodies (`phone_lookup_hash` → `phone`); spec 001 §C step 2 + AC3 wording (server-
    side model; AC3 checkbox unchanged); a regression test (`web/tests/contract/api-contract.test.ts` —
    each auth request requires `phone`, exposes no `phone_lookup_hash`); the drift lock. **No Rust source
    change** (the engine was already correct). This **unblocks** the T10-shell OpenAPI request codegen and
    the T07-shell-B PgAuthStore sign-in lookup, which now consume a consistent contract.

- [x] **Live DB-level integration tests of the atomic contracts (postgres:16) — DONE in slice A.**
      The true DB-level TOCTOU proofs the in-memory stub only *modelled* now exist in
      `server/store/tests/integration.rs` against real Postgres: single-consume under real
      concurrency, **`concurrent_rotate_and_replay_resolves_to_revoked`** (which *caught a real bug*
      — see the slice-A register), classification-correctness (rotated-N-times-ago ⇒ `Superseded`),
      family-kill persistence (`sessions.revoked_at` + legit-current-then-refused), and RLS
      isolation + fail-closed. **DONE:** 2026-06-05. Remaining → T07-shell-B: the *Worker-level*
      proof through the Hyperdrive Socket + pooler (miniflare/workerd), once that wiring exists.

- [ ] **Multi-device (phone + watch + iPad) concurrent bindings.** T07-core **decided: single
      active device per member** — re-onboarding invalidates **all** of a member's prior device
      bindings (sec-audit F5 "invalidate all"; no stale token survives; matches AC4 device-
      replacement). When watch/Wear/iPad pairing is specced (out of scope this spec), revisit
      whether a member may hold multiple concurrent bindings and scope invalidation per-platform
      instead of all-for-member.
  - **WHEN:** the watch/Wear-pairing spec.

> **Resolved in T07-core (2026-06-04)** — moved here from the T03/T04/T05/T06 registers above:
> phone **E.164 normalization** before hashing (`normalize_phone`, single-source for spec-008
> issuance); the **refresh-credential at-rest hash** primitive (`core::crypto::refresh_token_hash`);
> the **code-lifecycle decision composition** + **atomic consume-on-accept *contract*** +
> **sign-in response shape parity** (no existence leak); the **refresh rotation/replay *policy***
> + **classification *port*** + **shape-identical reject** (no lineage leak); the **per-member-
> per-day alert dedup** (AC8/AC15) and the **rate-limit *window* logic** (AC17). Their *DB/Worker
> enforcement* (persistence, real server-time, atomic SQL, Turnstile, Queues, 429) is T07-shell
> above. `chrono`/`time` stays deferred — T07-core needs only integer epoch math.

---

## Server / store (spec 001 T07-shell slice A — out-of-scope register)

> Slice A shipped **`boundless-server-store`** (`server/store/`, a NATIVE crate, member of the new
> `server/` workspace): `PgAuthStore` — the `tokio-postgres` (0.7.17, `with-uuid-1`) SQL +
> transaction layer for nine of the `AuthStore` contract methods (member lookup; onboarding
> load/consume; refresh classify/rotate/revoke/create-family; recovery load/consume-rotate), with
> per-request RLS tenant scoping (`set_config('app.current_group_id', $1, true)`). Methods are
> `async + Result` (mirroring the sync `AuthStore` 1:1) and proven against real `postgres:16` —
> **13 integration tests** (incl. the 3 reviewer-added: revoked-family-marks-superseded, onboarding
> consume-ignores-TTL, onboarding-superseded-not-live), self-skipping without `DATABASE_URL`; CI job
> `server-store`. The
> rotate-vs-replay TOCTOU test **caught a real bug** (a concurrent rotate's new current row escaped
> a concurrent revoke under READ COMMITTED); fixed with a `pg_advisory_xact_lock` on the family in
> both `rotate_session` and `revoke_family`. Driver decision = **ADR-0019** (tokio-postgres over a
> Hyperdrive Socket; sqlx dropped). Everything below was deliberately left out.

- [x] **The async-port bridge.** **DONE 2026-06-05 (ADR-0020).** `core/server`'s store ports are now
      **`async` + fallible** (shared `StoreBackend::Error`); `PgAuthStore` **implements `AuthStore`**;
      `AuthService` is proven end-to-end over the real `PgAuthStore` (`server/store/tests/service_pg.rs`,
      5 tests). The 48 T07-core tests were adapted to drive the async endpoints via host
      `pollster::block_on`. The device-token methods were split into a separate **`DeviceStore`** port
      so `PgAuthStore` could ship the session/code/member half without the (deferred) device encryption.

- [ ] **`DeviceStore` Postgres impl** (`current_device_bindings` / `invalidate_device` /
      `register_device`). Now isolated behind the `DeviceStore` port (ADR-0020); `PgAuthStore` does
      **not** implement it. `register_device` must write `token_encrypted bytea`, and **device-token
      at-rest encryption** is now *unblocked*: the I1-adjacent **secretbox** primitive (ADR-0025) lands
      at **spec 008 T02** (`encrypt_field` + the per-Group `GroupKey`), so the push spec (007) can
      encrypt the token under the same per-Group key. The DeviceToken is PII (P2) — storing it without
      encryption would violate "encrypt before writing." The Worker (T07-shell-B) and the orchestration
      tests currently compose `PgAuthStore` with an **in-memory** `DeviceStore`; implement the Postgres
      `PgDeviceStore` when the encryption primitive lands. **Guard-rail (sec-audit F3):** when it lands,
      assert at the SQL layer that the token column is `bytea` (the `_encrypted` contract) and add a
      `static_assertions` check that any persisted device-token wrapper exposes no `Serialize`/`Display`
      — so the test doubles' in-memory convenience (which holds the raw `DeviceToken`) can never be
      mistaken for the production storage shape.
  - **WHEN:** push spec **007** / issuance spec **008** (whichever brings device-token encryption).

- [x] **wasm32 feature flags + pooler-safe `query_typed*` + the `worker::Socket` transport — DONE.**
      (ii) the query migration + wasm feature flags: slice "store wasm-prep" 2026-06-08; (i) the transport:
      the PgAuthStore-over-Hyperdrive slice 2026-06-08. Originally Slice A used `tokio-postgres` default
      (native) features + the idiomatic *named* path (`query_one`/`query_opt`/`execute(&str,…)`). Now:
  - **(ii) DONE.** Every production query in `PgAuthStore` (the 9 port methods + `begin`'s RLS
    `set_config` + `ensure_least_privilege` + the advisory-lock SELECTs = 21 sites) issues via the
    **unnamed-statement `query_typed*`** family (`query_typed_one`/`query_typed_opt`/`execute_typed`,
    each `$n`'s `Type` inline) — **ADR-0024**, no fork. Proven on **real PG18** (25 store tests green on
    the typed path; the native tests therefore now DO exercise the typed family — the slice-B "fidelity"
    follow-up). The `tests/common` superuser scaffolding stays named **by design** (direct, never-pooled).
    (ADR-0019's Context "pooler dislikes *unnamed*" polarity was inverted — it's the *named/persistent*
    statements that break across pooled connections; *unnamed* `query_typed*` is the fix.)
  - **wasm feature flags DONE.** `server/store/Cargo.toml` target-splits `tokio-postgres`: wasm =
    `default-features=false` + `["with-uuid-1","js"]`. `cargo build --target wasm32 -p
    boundless-server-store` green; a CI step in `server-store` guards it.
  - **(i) DONE (2026-06-08, PgAuthStore slice).** `server/src/runtime/pg.rs::connect_pg` connects via
    `env.hyperdrive("HYPERDRIVE")?.connect()? -> worker::Socket` →
    `tokio_postgres::Config::from_str(connection_string).connect_raw(socket, NoTls)`, spawning the
    `Connection` future with `worker::wasm_bindgen_futures::spawn_local`. The store lib needed no change
    (it takes an already-connected `Client`). Proven account-free via vitest-pool-workers against PG18.

- [ ] **Real server-time `now`.** `PgAuthStore` takes `now: UnixSeconds` and binds it (no
      `SystemTime::now` in the lib — server-time is injected, T04/T05 carry-forward). The Worker
      supplies real server time; ties into the still-deferred `chrono`-vs-`time` pick (only needed
      when wall-clock formatting/parsing is required — integer epoch math suffices today).
  - **WHEN:** **T07-shell-B**.

- [~] **Connection lifecycle + non-superuser role provisioning (sec-audit W2 — highest-impact).**
      **Boot-guard SHIPPED (slice 2, 2026-06-05):** `boundless_server_store::ensure_least_privilege(&client)`
      returns `Err(StoreError::PrivilegeTooHigh)` if `current_setting('is_superuser')` is `on` or
      `rolbypassrls` is true for `current_user`; both legs are proven vs real `postgres:16`
      (`server/store/tests/least_privilege.rs` — superuser rejected, `boundless_app` accepted). **Remaining
      → T07-shell-B:** the Worker must (a) actually **call** it immediately after `hyperdrive.connect()`,
      before constructing any `PgAuthStore`, and **fail closed** (+ a CI smoke test); and (b) the infra must
      provision the **non-superuser / non-`BYPASSRLS`** runtime DB role. **If the Worker's Neon/Hyperdrive
      credential is a superuser or has `BYPASSRLS` (the Neon default `postgres` role often is), RLS is fully
      bypassed → cross-tenant PII read/write** — the single highest-impact way the privacy model fails in
      production; the guard now exists to catch it, but is inert until the Worker invokes it.
      **Neon specifics (verified 2026-06-08, docs-researcher):** Neon's default role `neondb_owner` **is**
      a `neon_superuser` member **with `BYPASSRLS`** → it would (correctly) trip the guard, so the Worker
      must connect as a **dedicated, non-superuser, non-`BYPASSRLS`, non-table-owner** app role. Because
      PG15+ **removed the implicit `PUBLIC` `CREATE` on the `public` schema** (carried through PG18), that
      app role inherits **no** privileges — the provisioning slice must **explicitly `GRANT USAGE` + the
      needed table/sequence privileges** to it (the local test harness's `boundless_app` setup is the
      template). Add a **deployed-edge smoke** that connects as the *real* app role and asserts
      `ensure_least_privilege` passes AND a cross-tenant read returns zero rows (the live analog of
      `rls_isolates_reads_by_tenant`). (sec-audit F2 at the PG16→18 bump.) NB the `least_privilege` test
      proves non-superuser/non-BYPASSRLS but **cannot** prove "not the table owner" — that exemption is
      covered by `FORCE ROW LEVEL SECURITY`, so keep FORCE on every PII table.
      **(a) call-site DONE (2026-06-08, see the T07-shell-B register). (b) provisioning SCRIPTED + tested
      account-free (2026-06-08, deploy-prep slice):** `scripts/provision-neon.sh` mints the app role with
      exactly `GRANT CONNECT + USAGE ON SCHEMA public + table DML` (no sequence/EXECUTE grants needed — uuid
      PKs, PUBLIC-EXECUTE functions), and `scripts/test-provision-neon.sh` asserts (as the real `boundless_app`
      LOGIN role over `public`) the literal `ensure_least_privilege` SQL returns both-false, cross-tenant
      reads return zero, fail-closed holds, **and `count(DISTINCT tableowner)=1`** — the last catches the
      ownership-split that a local superuser would mask (so `ON ALL TABLES` can't silently under-grant on
      Neon, where the owner is `neondb_owner`). **Remaining:** the operator runs `provision-neon.sh` against
      real Neon + the **live** deployed-edge cross-tenant smoke as that role.
  - **WHEN:** the operator's deploy (`docs/runbooks/deploy-worker.md`) — the scripted/tested legs are DONE.

- [ ] **Route `StoreError` through the scrubbed log path (sec-audit W4).** `StoreError::Db` wraps a
      `tokio_postgres::Error` whose `Display`/`Debug` includes the SQL + the Postgres server message
      — for a unique-violation that message echoes the **conflicting `bytea` key value** (e.g. a
      `refresh_token_hash` / `phone_lookup_hash`). That is a keyed hash, not plaintext PII, but a
      stored credential hash in a log is a hardening concern. The Worker (T07-shell-B) must log
      `StoreError` only via `boundless::logging::emit()` (P2/I10) — never `{e}`/`{:?}` of a `Db`
      error raw — and the I10 scrubber suite should gain a fixture with a synthetic unique-violation
      `DETAIL` carrying a `\x…` hex blob, asserting the emitter drops it.
  - **WHEN:** **T07-shell-B** (logging wiring) + the I10 scrubber suite.

---

## Server / admin-provisioning (spec 001 T08 — out-of-scope register)

> T08 shipped the **core + Postgres-store legs** of developer Admin creation + invitation mint
> (AC1(a) authz decision, AC16 mint/TTL, AC9 N-2 compat) — all host/real-`postgres:16` testable, **no
> `wrangler`/`worker-build`/Email-Workers toolchain needed**. The deployable Worker endpoint + the
> Developer hardware-key WebAuthn verification + Email Workers delivery + the invite *consume* are the
> deferred shell. Everything below was deliberately left out; each carries a WHEN trigger.

- [ ] **The deployable `/api/dev/admins` Worker endpoint + the HTTP-level AC1(a) integration test.**
      T08-core ships the authorization *decision* (`authorize_developer` → un-forgeable
      `DeveloperAuthority`, which `create_admin` requires by type) and the mint orchestration; the
      `#[event]`/Router route that classifies the request into a `DevCaller`, calls `authorize_developer`,
      then `create_admin`, is the shell. The AC1(a) **integration** test (real unauth + admin-auth HTTP
      requests to `/api/dev/admins` are both rejected) lands with that route — the core test
      (`ac1_admin_creation_rejects_unauth_and_admin`) proves the decision; the HTTP proof needs the Worker.
  - **WHEN:** **T08-shell** (the deployable Worker; alongside T07-shell-B — needs the same
    workers-rs/Hyperdrive wiring + `docs-researcher` for workers-rs + Email Workers).

- [ ] **Developer hardware-key WebAuthn verification (constructs the `DeveloperAuthority`).** I11
      requires Developer auth to be a hardware-key-backed WebAuthn credential. T08-core models the
      *capability* (`DeveloperAuthority`) but does **not** verify the hardware key — `authorize_developer`
      trusts a `DevCaller::Developer` the Worker must establish. The actual dev-WebAuthn registration +
      assertion verification (likely `@simplewebauthn` on the edge, like admin WebAuthn T09/ADR-0017, but
      a **separate developer credential store**) is unbuilt. Until it exists, no caller can legitimately
      become `DevCaller::Developer`.
  - **WHEN:** **T08-shell** / a dev-auth task (relate to ADR-0017's WebAuthn pattern).

- [ ] **Email Workers delivery + the email-body-no-PII/credential wire assertion (R9 / ADR-0015).**
      T08-core returns the `AdminInvitation` (opaque token + opaque admin id + expiry; PII-free **by
      construction** — the tainted token makes the struct un-serializable). Building the registration URL,
      sending it via **Email Workers**, and the test asserting the email body carries **only** the opaque
      token (no PII, no credential material — ADR-0015's 6 constraints) are the shell's.
  - **WHEN:** **T08-shell** (Email Workers binding).

- [~] **Invite consume on first WebAuthn registration (single-use, AC16 consume leg) → T09.** T08 shipped
      only the **mint** + the constant-time at-rest hash (`admin_invitation_token_matches`). **T09
      (2026-06-05) shipped the consume *logic*:** `evaluateInvite` (server-time TTL + single-use →
      `ADMIN_INVITE_EXPIRED`/`ADMIN_INVITE_CONSUMED`, routes to `InviteExpired`) and `verifyRegistration`
      consuming the invite (`InviteStore.markConsumed`) on a successful WebAuthn registration, behind the
      `InviteStore` port + tested (Vitest `ac16_*` + Playwright consume assertion). **Remaining → T09-shell
      (with T15):** the **real DB consume** — the Worker/Postgres `InviteStore` that hashes the presented
      token with the per-instance HMAC and compares against `admin_invitations.token_hash` (the P4 tension:
      resolved per ADR-0017's documented WebAuthn carve-out — that crypto stays server-side, routed through
      the core's `admin_invitation_token_matches`, NOT in edge-TS), and the **atomic** `consumed_at` stamp
      (the T06 supersede-then-insert twin).
  - **WHEN:** **T09-shell / T15** (deployable SvelteKit routes + KV/Postgres bindings).

- [ ] **`created_by` = the Developer's identity (write-side audit, I5).** T08 writes `created_by = NULL`
      (system) on the pending-admin member + invitation rows, because the Developer is not authenticated
      in this slice (the dev-WebAuthn verification that would establish the actor is deferred). Populate
      `created_by` with the verified Developer id once dev-auth lands, so the audit trail names who minted
      each Admin.
  - **WHEN:** **T08-shell** (with the dev-WebAuthn verification).

- [ ] **`fresh_admin_invitation` human-facing format (if any).** Like the Recovery Code, the invite
      token is currently a 256-bit opaque hex draw. ADR-0015 only requires it be opaque + no-PII (it rides
      in a URL, not typed by a human), so hex is fine — but if a human-typable form is ever wanted, that is
      a UX decision. No change expected.
  - **WHEN:** revisit only if the registration UX (T15) wants a typable token.

---

## Server / admin-WebAuthn (spec 001 T09 — out-of-scope register)

> T09 shipped the **framework-agnostic WebAuthn verification core** (`web/src/lib/server/webauthn/`) +
> tests (23 Vitest + 4 Playwright-virtual-authenticator) — the registration/assertion verification, invite
> consume *logic* (AC16), UV enforcement (R11/AC20), multi-cred + Developer-re-invite revoke (D4), KV
> challenge one-time-use (ADR-0017 D3) — all behind ports with in-memory fakes, **no `wrangler`/SvelteKit
> runtime needed**. Everything below was deliberately left out (the deployable shell); each carries a WHEN.

- [ ] **Additive backup-key enrollment (the second half of AC20 / ADR-0016 D4 "register a backup key").**
      T09's `verifyRegistration` is the **invite-gated** path and is **revoke-and-replace** by design
      (initial registration + lost-key **recovery** — a Developer re-invite revokes the prior credential(s),
      D4). That makes two *simultaneously-active* credentials unreachable via the invite path. Enrolling an
      *additional* key without revoking the first is an **authenticated** add-credential flow (the admin is
      already signed in, no invite) — it needs the **post-assertion session** (deferred shell, §10-F), so it
      lands with **T15**. The `CredentialStore` already supports >1 active credential per admin (a non-invite
      `insert` does not revoke); only the new entry point + UI are missing. Plan §7 (AC→test map, line ~158)
      names a second Playwright test **`ac20_register_passkey_and_backup_key`** for this additive flow — it is
      deferred here with the flow (T09 ships `ac20_webauthn_requires_uv_no_attestation_multi_credential`, which
      covers UV/attestation/recovery-revoke; the additive-backup test lands with the authenticated add-key UI).
  - **WHEN:** **T15** (admin onboarding/settings UI — needs the authenticated admin session).

- [ ] **The deployable SvelteKit `+server.ts` routes.** `/api/admin/auth/{invite,register,signin}` (or the
      SvelteKit-idiomatic equivalents) that wire the verification functions to HTTP requests/responses, set
      the post-assertion session, and map `WebAuthnError.code` → catalog copy + `routesTo`. Needs the
      scaffolded SvelteKit app (**T15**).
  - **WHEN:** **T15** (admin onboarding UI) / **T09-shell**.

- [x] **Real Cloudflare **KV** `ChallengeStore` impl. DONE 2026-06-07 (T15-shell leg A).** The production
      one-time-use, 5-min-TTL challenge store now runs on real Cloudflare KV (`KvChallengeStore`),
      account-free-proven against Miniflare KV via `getPlatformProxy()` and live in `vite dev`/Playwright
      under adapter-cloudflare. See "Admin web / SvelteKit onboarding (T15)" → leg (A) for details.

- [ ] **Real Postgres `InviteStore` + `CredentialStore` via the Worker.** Reads/writes of
      `admin_invitations` (load + atomic `consumed_at` stamp) and `admin_webauthn_credentials` (list active
      / insert / revoke-all-for-admin / bump sign_count) through the deployable Worker. **Includes the
      invite-token HMAC compare routed through the core** (`admin_invitation_token_matches`) per ADR-0017's
      P4 carve-out — that crypto stays server-side, **not** in edge-TS (the T08-flagged tension). The
      credential `public_key`/`credential_id` are `bytea`; storing them is not PII but follows the
      `_encrypted`/`bytea` conventions.
  - **WHEN:** **T15 / T09-shell** (Hyperdrive/Postgres binding) — pairs with the T07-shell-B Worker runtime.

- [ ] **Post-assertion session establishment (plan §10-F).** The **httpOnly + Secure + SameSite=Strict**
      server-side session cookie minted after a successful WebAuthn assertion (the admin session; separate
      and shorter-lived than member sessions, ADR-0016). T09 returns the verified `adminId`; the cookie is
      the shell's.
  - **WHEN:** **T15 / T09-shell**.

- [ ] **AC11b — admin-web a11y (axe-core + keyboard ceremony).** Zero axe violations on each admin
      onboarding route, keyboard-operable WebAuthn ceremony, `aria-live` on invite-expired/error, 200%/400%
      reflow, RTL/dark. This is a **UI** concern (the screens don't exist until T15) — Playwright+axe lives
      with the UI, not the verification core.
  - **WHEN:** **T15** (admin onboarding UI).

- [ ] **Invite token rides in the URL path — harden the shell against log/referrer leakage** (sec-audit
      F1, surfaced at the T10 contract freeze). The frozen contract's `GET /api/admin/auth/invite/{token}`
      carries the single-use invitation token in the **URL path** (it must be openable from an email
      click). The token is opaque + no-PII + single-use + 72h-TTL + consumed-on-first-registration (so the
      blast radius is bounded, ADR-0015) — **not** a contract defect — but a live invite token in an access
      log / `Referer` / browser history is a credential-in-logs concern (the P2/I10 reasoning that makes
      device tokens PII). The shell must: (a) never emit the `{token}` segment to the structured log path —
      route through `boundless::logging::emit()` and add an **I10 scrubber fixture** for a URL-embedded
      opaque token (assert the segment is redacted); (b) set **`Referrer-Policy: no-referrer`** on the
      registration page so the path can't leak via sub-resource `Referer`; (c) keep the single-use consume
      atomic so a leaked-but-consumed token is inert (already the T09 consume design).
  - **WHEN:** **T15 / T09-shell** (the deployable invite route) + the I10 scrubber suite.

- [ ] **Live deployed-edge E2E + the `webauthn-rs`-sidecar fallback (ADR-0017).** A smoke test against the
      deployed SvelteKit Worker (Miniflare/workerd), and — only if `@simplewebauthn`'s "unofficially
      supported" Workers status ever breaks — the documented fallback to a native `webauthn-rs` sidecar.
      Not built now.
  - **WHEN:** the deploy/CI-hardening pass (with T07-shell-B / T15) — or if the edge runtime breaks.

---

## API contracts / codegen (spec 001 T10 — out-of-scope register)

> T10 **froze** the wire contracts (`api/openapi.yaml` + `api/boundless.proto`) and closed AC7's
> contract leg with two host-testable parsing tests (`web/tests/contract/api-contract.test.ts` +
> `core/sync/tests/proto_contract.rs`) + the regenerated binding-drift lock — all in the installed
> toolchains. The **actual binding generation** was deliberately deferred: the codegen toolchains
> (`buf`/`protoc`, `swift-openapi-generator`, `openapi-generator`, `uniffi-bindgen`/`wasm-pack`) are
> **not installed**, and the UIs that consume the generated bindings (T11–T15) are themselves
> toolchain-blocked — so generating committed artifacts we cannot build/verify end-to-end would
> violate "evidence > intuition". The freeze is the substantive gate; codegen is downstream mechanics
> reproducible from the frozen contract. Everything below is the T10-shell.

> **BoundlessKit / T10-shell (Swift leg) — DONE 2026-06-05.** The UniFFI binding from the Rust core
> to Swift is built and verified end-to-end on the iOS simulator (the prerequisite that unblocks T11
> at the FFI level). The earlier "toolchain not installed" reading was wrong for Apple: Xcode 26.5 is
> installed but not `xcode-select`'d (sudo unavailable), so the build uses
> `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`. Shipped:
> - `core/ffi-swift` — `uniffi` 0.31.1; mirror enums (`OnboardingState`/`OnboardingEvent`/`LaunchDecision`/
>   `SignInResult`/`BindResult`/`Role`) + exhaustive `From` conversions + `#[uniffi::export]` free fns
>   (`launch`/`on_event`/`is_terminal`/`allows_offline_overlay`/`reauth_state_for`/
>   `should_flag_notifications_off`); `crate-type=["lib","staticlib","cdylib"]`; the `uniffi-bindgen`
>   CLI is a `[[bin]]` behind a host-only `bindgen` feature. 5 host round-trip/transition tests. The
>   wasm core stays uniffi-free (ADR-0022); `cargo build --target wasm32 -p {domain,crypto,auth,server-core}`
>   still clean; ffi-swift is deliberately NOT wasm-buildable and never on the wasm path.
> - `scripts/build-boundlesskit.sh` (cargo iOS device+sim `.a` + host cdylib → `uniffi-bindgen --library`
>   → `module.modulemap` rename → `xcodebuild -create-xcframework`) + `scripts/test-boundlesskit.sh`
>   (auto-detects an available iPhone sim) → `apple/BoundlessKit/` SwiftPM package + a 5-assert smoke
>   test that **passes on the iPhone 17 simulator** (Rust→UniFFI→Swift on-device).
> - The XCFramework binary + the generated Swift wrapper are **git-ignored build artifacts**
>   (reproducible from `core/ffi-swift`, which the binding-drift gate already tracks via `core/**`) —
>   distinct from the committed `api/generated/**` wire bindings. **Reversible** if toolchain-free
>   consumption is later wanted (commit the xcframework + extend the drift gate to the uniffi output).
> - New CI job `boundlesskit` (`macos-15`: rustup iOS targets → `scripts/test-boundlesskit.sh`).
>   **GitHub-only — not locally verifiable** (like `server-migrations`/`web`); the simulator
>   destination is auto-detected so it survives the runner's Xcode/sim version.
> - ADR-0022 records the mirror-types decision; `docs/stack-matrix.md` filled (`uniffi` 0.31.1; Apple
>   `BoundlessKit` row). **Still deferred:** the OpenAPI Swift HTTP client (swift-openapi-generator,
>   → T11); `core/ffi-kotlin` AAR (→ T13/T14); proto-Swift (→ realtime spec); committing the
>   xcframework + drift-gating the uniffi Swift (optional hardening). T11 (the Rider SwiftUI screens,
>   String Catalog, ×4 snapshot variants, VoiceOver, no-signup/no-toggle inspection tests) is NOT
>   started — it is the next slice, now FFI-unblocked.

- [ ] **Real per-target codegen + the `generate-bindings.sh` "real generators" block.** Wire each
      generator into `scripts/generate-bindings.sh` (replacing the scaffold-mode hash-only step) and
      commit the produced `api/generated/<lang>/` trees + the refreshed drift lock:
      - **Swift:** ~~the UniFFI **`BoundlessKit` XCFramework**~~ **DONE 2026-06-05** (see the
        "BoundlessKit / T10-shell (Swift leg)" register below). Still deferred: the **OpenAPI Swift
        HTTP client** (`swift-openapi-generator` + `protoc-gen-swift` → `api/generated/swift/`) — the
        network layer the Rider UI drives. **WHEN: with T11** (needs the SwiftPM build-tool plugin;
        versions pre-pinned via docs-researcher — swift-openapi-generator 1.12.2 / runtime 1.6.0 /
        urlsession 1.4.0 — confirm at use).
      - **Kotlin:** `openapi-generator` (kotlin) + `protoc-gen-kotlin` → `api/generated/kotlin/` + the
        UniFFI **`core-bridge` AAR**. **WHEN: with/before T13–T14** (Compose UIs; needs Android
        Studio/Gradle + `openapi-generator` + `uniffi-bindgen`).
      - **TypeScript:** `openapi-typescript` + `ts-proto` → `api/generated/typescript/` +
        `web/src/lib/api/generated/`. `openapi-typescript` is Node-only (could be wired early since
        Node/pnpm exist), but `ts-proto` needs `protoc`/`buf` — so the **full TS set** lands together.
        **WHEN: with T15** (SvelteKit admin UI).
      - Each landing needs `docs-researcher` to pin the generator versions (lock = ground truth) +
        fill `docs/stack-matrix.md`. Until then `api/generated/**` stays committed `.gitkeep`
        placeholders and the drift gate runs in scaffold mode (hash-only).

- [ ] **Carry T02's platform-parity UniFFI mapping notes into the codegen.** When the UniFFI
      XCFramework/AAR are generated: `AppVersion` record-vs-string mapping, `MemberId` UniFFI
      custom-type mapping, and the tainted-type formatter-free binding surface (no `Debug`/`Display`
      leaking across the FFI — P2/I3). Flagged by `platform-parity` at T02; actionable at codegen.
  - **Status (T10-shell Swift, 2026-06-05):** the BoundlessKit surface **deliberately excludes**
    `AppVersion`, `MemberId`, and **all** tainted/PII types (only the state-machine enums + `Role` +
    `bool` cross — see ADR-0022 scope), so none of these mappings were needed yet. They become
    actionable when a UI task first needs one of those types across the FFI.
  - **WHEN:** the Swift/Kotlin codegen above (T11–T14) — whenever `AppVersion`/`MemberId`/a tainted
    type is first exported.

- [ ] **Strict fixture↔OpenAPI conformance test (host-only hardening).** The T10 AC7 tests check the
      *version-handshake* invariant on every `/api/auth/*` response, and the core↔wire `ManifestPointer`
      drift that platform-parity caught was fixed at the source (core `ManifestPointer` now carries
      `locale_key_prefix`). A stronger guard would validate each `fixtures/auth/*.json` against its
      corresponding frozen OpenAPI schema (de-`$ref`'d `oneOf` member) so ANY field-name/shape drift
      between a golden fixture and the contract fails CI — e.g. a small Vitest test using a JSON-Schema
      validator (`ajv`). Deferred because it adds a new dep beyond the freeze slice; the actual drift it
      targets is already closed.
  - **WHEN:** a contract-hardening pass (could ride with **T15**, when the web dep tree expands anyway).

- [ ] **Live deployed-edge contract-conformance E2E.** Replay the golden fixtures against the
      deployed Worker to prove the runtime responses actually conform to the frozen OpenAPI (the T10
      tests check the *contract document*, not a live server — which doesn't exist yet).
  - **WHEN:** with the Worker runtime (**T07-shell-B**) / the deploy-hardening pass.

## Apple / Rider UI (spec 001 T11 — out-of-scope register)

> T11 shipped the **Rider onboarding UI slice**: `apple/BoundlessRider` (`RiderShared` lib + tests,
> no `.xcodeproj`) — every onboarding screen rendered from the `core::auth` state machine via
> `BoundlessKit` (P4), the String Catalog, Rider Settings, and the full named test suite (68 ×4
> a11y snapshot baselines + 27 logic tests, green on the iPhone 17 Pro sim). Same functional-core /
> imperative-shell split as T07–T10. Everything below was deliberately left out; each carries a WHEN.

- [ ] **The deployable iOS app shell (`.xcodeproj` app bundle).** A pure SwiftPM package cannot
      produce a runnable iOS `.app` (App lifecycle, `Info.plist`, entitlements, bundle id
      `app.boundless.rider`, launch screen). The shippable app target — the composition root that
      instantiates `OnboardingViewModel` with the *real* conformers and hosts `OnboardingRouter` —
      is deferred. The AC tests are all view/model-level and need no app bundle.
  - **WHEN:** when preparing the first iOS build (ties to the Apple licensing/entitlement items above).

- [ ] **The OpenAPI Swift HTTP client (the real `OnboardingNetworking`).** `swift-openapi-generator`
      (pre-pinned via docs-researcher: generator **1.12.2** / runtime **1.6.0** / urlsession **1.4.0**
      — confirm at use) → the `/api/auth/{signin,bind-device}` client that feeds real
      `SignInResult`/`BindResult` into the view model. Deferred because the deployable Worker it calls
      does **not exist yet** (T07-shell-B) — building it now is untestable "this should work" code. The
      `OnboardingNetworking` protocol + a stub already isolate it; the real impl drops in untouched.
  - **WHEN:** **T07-shell-B** lands (a live Worker to integration-test against) / first iOS build.

- [ ] **Keychain refresh-token storage (plan §10-F) + APNs registration + signed-manifest fetch/verify.**
      The real `ManifestProviding` (KV manifest fetch + libsodium verify + cache, ADR-0014, providing
      `{adminName}`), the real `NotificationPermissionRequesting` (`UNUserNotificationCenter`; Critical
      Alerts once the entitlement lands — DEFERRED), the APNs device-token registration, and the
      **Keychain** refresh-credential store (never `UserDefaults`/`@AppStorage`, forbidden-patterns).
      All behind injected protocols today; conformers are the shell.
  - **WHEN:** the iOS app shell / push spec **007** / **T07-shell-B**.

- [ ] **Recorded VoiceOver walkthrough + Accessibility Inspector pass (manual).** The automated AC11
      leg asserts the model-level reading order (labels/headings/order, "auto-update enabled" as a
      state not a button). swift-snapshot-testing has no a11y-tree strategy, so the **recorded**
      VoiceOver/Switch-Control walkthrough + Xcode Accessibility Inspector run remain a manual
      checklist item (plan §7 "hard-to-test"). Optional automation: add CashApp **AccessibilitySnapshot**
      for an a11y-hierarchy image+text snapshot (a new dep — weigh against the model-level assertion).
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** The 68 baselines were recorded locally on the iPhone 17
      Pro sim / iOS 26.5, pinned to the `iPhone13` device config with `perceptualPrecision 0.98`. If the
      `macos-15` runner's simulator runtime renders fonts/AA differently enough to exceed the tolerance,
      the baselines need a one-time CI re-record (a well-known snapshot-testing operational reality). The
      `boundlessrider` job is **GitHub-only / not locally verifiable** (like `boundlesskit`).
  - **WHEN:** first CI run of the `boundlessrider` job (re-record from the runner if it diverges).

- [ ] **Added copy beyond the spec's 14 screen-copy keys — product-owner review.** T11 added 11
      catalog keys (catalog total 25), all voice-and-tone-compliant and trivially editable pre-release:
      (a) **7 affordance/settings** keys, because P8 forbids hardcoded strings and the a11y design
      mandates "a single large control per step" — `onboarding.action.{continue,try_again}`,
      `onboarding.permissions.{allow,decline}` ("Turn on notifications" / "Not now"), Rider Settings
      rows `settings.{title,notifications,help}`; (b) **4 name-less fallback** keys (review-driven fix,
      mirroring the spec's own `auth.below_min_version_generic`) so the four name-bearing screens render
      a generic sentence — not an empty `%1$@` slot — when no manifest/admin name is cached:
      `onboarding.signin.phone_not_on_file_generic`, `onboarding.binding.{code_prompt,code_invalid}_generic`,
      `onboarding.permissions.notifications_declined_generic`.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [~] **`auth.signin_again` (Driver re-auth) + the two `admin.onboarding.*` keys** are authored in the
      Rider catalog for completeness (AC12) but rendered elsewhere. **`auth.signin_again` DONE (T12,
      2026-06-05):** the `L10n.signInAgain` accessor was added (the seam T11 left) and the Driver
      re-auth `PhoneEntry` renders it (`apple/BoundlessDriver`). The two `admin.onboarding.*` keys still
      have no L10n accessor by design — rendered by the SvelteKit admin UI.
  - **WHEN (remaining):** **T15** (admin web).

## Apple / Driver UI (spec 001 T12 — out-of-scope register)

> T12 shipped the **Driver onboarding UI slice**: `apple/BoundlessDriver` (`DriverShared` lib + tests,
> no `.xcodeproj`) — the Driver self-onboarding flow rendered from `core::auth` via `BoundlessKit`,
> **reusing the `RiderShared` kit** (screen model/renderer, `L10n`, `OnboardingViewModel`, the injected
> protocols, the role-neutral screen factories) and adding only the Driver deltas: the self-onboard
> intro, the **Recovery Code one-time capture** screen (ADR-0016 D3 / AC19 capture leg), and the
> interactive re-auth `PhoneEntry` (`auth.signin_again`, AC15 Driver branch). 21 tests green on the
> iPhone 17 Pro sim (76 ×4 a11y snapshot baselines + logic). Two minimal, non-breaking extensions to the
> shared `RiderShared` kit: the `L10n.signInAgain` accessor (the seam T11 left) and a new
> `BodyElement.code` element (prominent monospaced/selectable — for the Recovery Code); T11's 68
> baselines re-verified green. Same functional-core / imperative-shell split as T07–T11. Everything
> below was deliberately left out; each carries a WHEN trigger.

- [ ] **The deployable `.xcodeproj` Driver app bundle.** A pure SwiftPM package cannot produce a
      runnable iOS `.app` (App lifecycle, `Info.plist`, entitlements, bundle id **`app.boundless.driver`**
      — see the Apple section's "Register the Driver app Bundle ID" item, now triggered). The shippable
      app target that instantiates `DriverOnboardingViewModel` with the real conformers and hosts
      `DriverOnboardingRouter` is deferred. The AC tests are all view/model-level and need no app bundle.
  - **WHEN:** when preparing the first Driver iOS build (ties to the Apple licensing/entitlement items).

- [ ] **The real `RecoveryCodeProviding` + the OpenAPI Swift client (incl. `/api/auth/recovery/rebind`).**
      T12 ships the `RecoveryCodeProviding` protocol + a stub; the real impl reads `fresh_recovery_code`
      off the `/api/auth/bind-device` (and rebind) response. Deferred because the deployable Worker those
      calls target does **not exist yet** (T07-shell-B). Drops in behind the protocol untouched.
  - **WHEN:** **T07-shell-B** lands (a live Worker) / first Driver iOS build.

- [ ] **The self-serve re-bind ENTRY UI** (phone + Recovery Code on a *new* device → re-bind, old token
      invalidated, fresh code issued). The onboarding **state machine has no recovery-rebind state** to
      render, so building a UI for it now would be UI not driven by the core (against P4). The AC19
      server/logic legs are **done** (T04 `evaluate_recovery_code`, T05/T07 rebind + fresh-code +
      old-token invalidation); T12 closed AC19's **capture** leg. The re-bind entry needs either a new
      core state or a separate flow — surface when that flow is specced.
  - **WHEN:** a recovery-rebind flow spec (or when the Driver app shell adds a "new phone" entry point).

- [ ] **Keychain refresh-token storage (§10-F) + APNs registration + signed-manifest fetch/verify.**
      The Driver reuses the same injected `ManifestProviding` / `NotificationPermissionRequesting`
      boundaries as the Rider; the real conformers (KV manifest fetch + libsodium verify + cache;
      `UNUserNotificationCenter`; the Keychain refresh store) are the shell. Same as the T11-shell items.
  - **WHEN:** the Driver iOS app shell / push spec **007** / **T07-shell-B**.

- [ ] **Recorded VoiceOver walkthrough + Recovery-Code spell-out a11y (manual / polish).** The automated
      AC11 leg asserts the model-level reading order (incl. the code as static text). A **recorded**
      VoiceOver/Switch-Control walkthrough remains a manual checklist item; an optional polish is reading
      the Recovery Code **character-by-character** (a per-character `accessibilityLabel`) rather than as a
      single token — weigh when the persona-acceptance/a11y review runs.
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** The 76 Driver baselines were recorded locally on the iPhone
      17 Pro sim / iOS 26.5, pinned to `iPhone13` + `perceptualPrecision 0.98` (same as T11). If the
      `macos-15` runner's simulator runtime renders differently enough to exceed the tolerance, the
      baselines need a one-time CI re-record. The `boundlessdriver` job is **GitHub-only / not locally
      verifiable** (like `boundlessrider`/`boundlesskit`).
  - **WHEN:** first CI run of the `boundlessdriver` job (re-record from the runner if it diverges).

- [ ] **4 added catalog keys beyond the spec's table — product-owner review.** T12 added a Driver
      catalog (`DriverOnboarding.xcstrings`, table `DriverOnboarding`) with 4 voice-and-tone-checked
      keys the Driver flow needs but the spec's i18n table didn't enumerate: `onboarding.driver.intro`
      ("Let's get you set up." — self-onboard, vs the Rider's helper-facing "…together"), and the
      Recovery Code capture trio `onboarding.recovery.{title,explanation,saved}` ("Save your Recovery
      Code." / "You'll need this to set up Boundless on a new phone. Keep it somewhere safe." / "I've
      saved it"). All trivially editable pre-release.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [ ] **`DriverShared` reuses `RiderShared` directly (no extracted "OnboardingKit" module).** T12 made
      two minimal additive extensions to `RiderShared` (the `signInAgain` accessor + `BodyElement.code`)
      rather than extracting a third shared module. If a future consumer needs the kit without the
      "Rider" name (none today — Compose/web are separate platforms), consider extracting a neutral
      `BoundlessOnboardingKit` SwiftPM module. Not needed now (YAGNI).
  - **WHEN:** if/when a third Apple consumer of the onboarding kit appears.

## Android bring-up (spec 001 — DONE 2026-06-06; unblocks T13/T14)

> The Android toolchain + `core/ffi-kotlin` UniFFI AAR + `android/` Gradle project — the Kotlin
> analog of T10-shell's BoundlessKit — are now **built and proven end-to-end** (the prerequisite
> that turned T13/T14 from "toolchain-blocked" to "just write the screens + their tests"). Shipped:
> - **`core/ffi-kotlin`** — the UniFFI surface, mirroring `core/ffi-swift` exactly (mirror enums
>   `Role`/`OnboardingState`/`OnboardingEvent`/`LaunchDecision`/`SignInResult`/`BindResult` +
>   exhaustive `From` parity guard + the 6 `#[uniffi::export]` fns; `crate-type=["lib","cdylib"]`;
>   `uniffi-bindgen` `[[bin]]` behind a host-only `bindgen` feature). 5 host round-trip tests
>   (`cargo test -p boundless-ffi-kotlin`). The wasm core stays uniffi-free — `cargo build
>   --target wasm32 -p {domain,crypto,auth,server-core}` still clean (ADR-0022; ffi-kotlin is
>   never on the wasm path).
> - **Android toolchain** installed under `$HOME` (no sudo): cmdline-tools `latest` (20.0) +
>   platform-34 + build-tools-34.0.0 + **NDK 28.2.13676358**; the 4 Rust Android targets
>   (aarch64/armv7/x86_64/i686-linux-android); **cargo-ndk 4.1.2**. Proven by a 4-ABI `.so`
>   cross-compile.
> - **`scripts/build-corebridge.sh`** (Kotlin analog of build-boundlesskit.sh): host cdylib →
>   `uniffi-bindgen` Kotlin → `cargo ndk` 4-ABI `.so` into `:core-bridge` jniLibs + the host cdylib
>   for the JVM smoke test. Generated Kotlin + `.so` are **git-ignored build artifacts** (reproducible;
>   tracked via `core/**` in the drift gate), exactly like the BoundlessKit xcframework.
> - **`android/` Gradle project** (wrapper-pinned **Gradle 8.7**; **AGP 8.4.2 · Kotlin 2.0.21 ·
>   Paparazzi 1.3.5 · Compose 1.7.5/Material3 1.3.1 · compileSdk 34 · JNA 5.17.0** — the
>   proven-Paparazzi-green set, ground truth = Paparazzi 1.3.5's catalog): `:core-bridge` (the P4
>   "BoundlessCore" AAR) + `:rider:app` (T13 home) + `:driver:app` (T14 home). **Proven green:** the
>   `:core-bridge` host-JVM **FFI smoke test** (Rust→UniFFI→Kotlin/JNA, no emulator — 1 test pass),
>   the `:rider:app` **Paparazzi** sample record+verify, and `assembleDebug` for both apps (the
>   per-ABI `libboundless_ffi_kotlin.so` is packaged into the APK alongside JNA's libjnidispatch).
> - **CI:** new `android` job (Ubuntu; `android-actions/setup-android` + sdkmanager + rust targets +
>   cargo-ndk → `scripts/test-corebridge.sh`). GitHub-only / not-locally-gated (like `boundlesskit`).
> - `docs/stack-matrix.md` filled (Android section + Kotlin/cargo-ndk/SDK-NDK rows); ADR-0022 already
>   covers the mirror-types decision (no new ADR — the Kotlin leg is the documented Android twin).
>
> **T13 and T14 are now UNBLOCKED** — each is "write the Compose screens (rendered from `:core-bridge`)
> + the ×4 a11y Paparazzi snapshots + TalkBack/no-signup/no-toggle tests", mirroring T11/T12.
>
> Out-of-scope register for this bring-up (each with a WHEN):

- [x] ~~Install the Android SDK + NDK~~ — **DONE 2026-06-06** (cmdline-tools 20.0, platform/build-tools
      34, NDK 28.2.13676358 under `~/Library/Android/sdk`).
- [x] ~~Build the `core/ffi-kotlin` UniFFI AAR~~ — **DONE 2026-06-06** (see above; ADR-0022 parity).
- [x] ~~Stand up the `android/` Gradle project~~ — **DONE 2026-06-06** (`:core-bridge` + `:rider:app` +
      `:driver:app`, Paparazzi + FFI smoke proven green).

- [ ] **Committed `gradle.lockfile`(s) + fold the Android tree into `check-network-allowlist.sh` (I8/AC13).**
      The Android dep tree is **already gated** by an **interim** CI step (`scripts/check-android-trackers.sh`,
      run in the `android` job): it resolves the three modules' dependency closures and greps them against
      `ci/forbidden-trackers.txt` (currently clean — only androidx/jna/paparazzi/kotlin), so a transitive
      bump that pulls a tracker fails CI, not just review. What's still deferred is the *committed*
      `gradle.lockfile` + having `check-network-allowlist.sh` scan it like the other 5 locks (it already
      globs `gradle.lockfile`, so it's "commit the lockfile + re-run"). Enabling Gradle dependency locking
      across an AGP multi-module build has known footguns (`lockAllConfigurations` makes every config
      require a lock → builds break if generation missed one; LENIENT mode + a `resolveAndLockAll` task is
      the safer shape), so the *lockfile* deserves its own focused slice — but the tracker risk is covered
      now by the interim grep.
  - **WHEN:** with **T13/T14** (when the Android dep tree stabilizes with the real Compose/Hilt/Turbine
    set) or the next CI-hardening pass. The interim grep covers the gap until then.

- [ ] **`sdkmanager`/cmdline-tools writes SDK XML v4; AGP 8.4.2's tooling understands up to v3** — a
      benign build **warning** ("This version only understands SDK XML versions up to 3 but … version 4
      was encountered"), seen because cmdline-tools `latest` (20.0) is newer than AGP 8.4.2. Build/test/
      assemble all succeed regardless. If it ever becomes more than cosmetic, pin an older cmdline-tools
      or bump AGP (which is gated on Paparazzi — see the version note in stack-matrix).
  - **WHEN:** only if it stops being a pure warning (or when AGP is next bumped).

- [ ] **Snapshot-baseline CI-runtime pin.** The Paparazzi sample baseline was recorded locally
      (macOS/this machine). It is **text-free** (a solid Material3-color box), so it should be
      byte-stable across the Ubuntu runner (no font hinting) — but the `android` job is **GitHub-only /
      not locally verifiable** (like `boundlessrider`), so the first CI run is the real proof; re-record
      from the runner if it diverges. T13/T14's real screens (with text) will face the usual snapshot
      cross-runtime tolerance question — handle as on iOS.
  - **WHEN:** first CI run of the `android` job; and at T13/T14 for the real screens.

- [ ] **Kotlin OpenAPI/proto codegen → `api/generated/kotlin/`** (the T10 codegen register's Kotlin
      leg: `openapi-generator` kotlin + `protoc-gen-kotlin`). NOT part of the bring-up (that wired only
      the **UniFFI** Kotlin, the domain/auth state machine — the network client is separate, exactly as
      the Swift OpenAPI client was deferred from the BoundlessKit T10-shell). Re-run the network
      allow-list against the new `gradle.lockfile`(s) when it lands.
  - **WHEN:** with **T13/T14** (the Compose UIs that consume the generated network client).

- [ ] **Carry the T02 platform-parity UniFFI mapping notes into the Kotlin codegen** (`AppVersion`
      record/string, `MemberId` custom-type, tainted-type formatter-free surface — same as flagged for
      Swift). The bring-up's `core/ffi-kotlin` surface deliberately excludes all of these (only the
      state-machine enums + `Role` + `bool` cross — ADR-0022 scope), so none were needed yet; they become
      actionable when a Kotlin UI task first needs one of those types across the FFI.
  - **WHEN:** **T13/T14**, whenever `AppVersion`/`MemberId`/a tainted type is first exported to Kotlin.

- [x] **`core/ffi-kotlin` ⇄ `core/ffi-swift` surface parity is a convention, not yet a gate.** The two
      mirror crates MUST stay identical (same enums/variants/fns), enforced today only by the shared core
      they both mirror (a core change breaks both compiles) + the `platform-parity` review. A cheap CI
      guard (e.g. assert the exported fn/enum sets match across the two crates) would make the lock-step
      mechanical rather than reviewer-dependent.
  - **DONE:** 2026-06-07. Gate = the host test **`core/ffi-swift/tests/parity_with_kotlin.rs`**: it
    `include_str!`s both crates' `lib.rs`, normalizes each (the production region before `#[cfg(test)]`,
    minus `//!`/`///`/`//` full-line comments + blank lines + trailing whitespace) and asserts the two
    are **byte-identical**, reporting the first differing line on mismatch. Whole-region byte-identity is
    strictly stronger than a fn/enum-set check (it also pins the symmetric `From` mappings + variant
    field names/order) and needs no Rust parser — robust because the post-edit hook keeps both files
    `cargo fmt`-canonical. It **complements** (does not overlap) the compile-time `From`-`match` guard:
    that catches *core↔mirror* drift, this catches *mirror↔mirror* (FFI-only) drift the compile cannot
    see. Rides the existing `rust-core` CI job (`cargo test --workspace`) — no new CI wiring. Hardened per
    the platform-parity review (F3): asserts `#[cfg(test)]` appears **exactly once** so a future
    cfg(test)-gated *production* item can't silently shrink the compared surface (a false-pass) — proven
    by a transient negative check (`found 2` → loud fail). Header pointers added to both `lib.rs` files.
    Reviews: `platform-parity` + `reviewer` both "ship it" (0 crit/high/med). **Out of scope (still
    deferred):** sharing the mirror types from a single source — ADR-0022 keeps the two crates separate
    deliberately (the wasm-free-core constraint); this gate makes the hand-maintained lock-step
    mechanical, nothing more.

## Android / Rider UI (spec 001 T13 — out-of-scope register)

> T13 shipped the **Compose Rider onboarding UI slice** in `android/rider/app` (package
> `app.boundless.rider`): every onboarding screen rendered from `core::auth` via the `:core-bridge`
> UniFFI AAR (P4), the `strings.xml` catalog + `RiderStrings`, the view-model/router, Rider Settings,
> and the full test suite — 68 ×4 a11y Paparazzi baselines + 10 logic/a11y/content test classes
> (`./gradlew test` debug+release green; `verifyPaparazziDebug` green; both apps `assembleDebug`).
> Same functional-core / imperative-shell split as T07–T12. Everything below was deliberately left out
> (the **T13-shell**); each carries a WHEN trigger.

- [ ] **The deployable launcher `MainActivity` (the composition root).** A `com.android.application`
      module assembles without an Activity (proven), so T13 ships the screens/model/VM/router/catalog +
      tests as "UI legs" with **no launcher Activity** — the Android twin of T11 deferring the iOS
      `.xcodeproj` app bundle. The shippable `MainActivity` that instantiates `OnboardingViewModel` with
      the real conformers, hosts `OnboardingRouter`, and wraps it in `RiderTheme` is deferred.
  - **WHEN:** when preparing the first Android build (ties to the FCM/Play-auto-update items below).

- [ ] **The production `AndroidRiderStrings` (R.string resolver) + its wiring.** T13 ships the
      `RiderStrings` interface + `Keys` + `strings.xml`; the production impl over Android `Resources`
      (`getString(R.string.x, *args)`) is the shell (it needs a `Context`, so it is constructed by
      MainActivity). Tests/snapshots use `CatalogRiderStrings` (parses the same strings.xml). When
      `AndroidRiderStrings` lands, an instrumented/Robolectric smoke test could exercise the real
      resource path (today it is compile-checked only — its `key→R.string` map references must resolve).
  - **WHEN:** **T13-shell** (with MainActivity).

- [ ] **The real `OnboardingNetworking` (OpenAPI Kotlin HTTP client).** T13 ships the
      `OnboardingNetworking` interface + a fake; the real impl (`openapi-generator` kotlin →
      `/api/auth/{signin,bind-device}`) feeds real `SignInResult`/`BindResult` into the view model.
      Deferred because the deployable Worker it calls does **not exist yet** (T07-shell-B); building it
      now is untestable "this should work" code. The Kotlin OpenAPI/proto **codegen** itself is the
      T10-codegen register's Kotlin leg (`api/generated/kotlin/`) — re-run the network allow-list against
      the (still-deferred) committed `gradle.lockfile` when it lands.
  - **WHEN:** **T07-shell-B** (a live Worker) + the T10 Kotlin codegen.

- [ ] **`NotificationManager` permission flow + FCM registration + signed-manifest fetch/verify +
      Keystore refresh storage (§10-F).** The real `NotificationPermissionRequesting`
      (`POST_NOTIFICATIONS` runtime permission on API 33+; Critical-Alerts-equivalent N/A on Android),
      FCM device-token registration, the `ManifestProviding` impl (KV manifest fetch + libsodium verify
      + cache, ADR-0014, providing `{adminName}`), and the **EncryptedSharedPreferences / Keystore**
      refresh-credential store (never plain `SharedPreferences`, forbidden-patterns) are all behind
      injected boundaries today; the conformers are the shell.
  - **WHEN:** the Android app shell / push spec **007** / **T07-shell-B**.

- [ ] **Onboarding-Code / phone field input-security (security-auditor M1).** The `OutlinedTextField`
      sets only `KeyboardType` today. When the real input flow is wired (a helper types the code), the
      code field must be marked as a **one-time code** (`Modifier.semantics { contentType =
      ContentType.SmsOtpCode }`, a **Compose 1.8** API) + **no-personalized-learning** /
      autocorrect-off, and the phone field `ContentType.PhoneNumber`, so the single-use binding secret
      and the phone number do not leak into the keyboard's learning dictionary / autofill store (P2
      spirit). The iOS twin gets this via `.textContentType(.oneTimeCode/.telephoneNumber)`. Deferred
      because (a) T13 wires no real input (the field is rendered in snapshots only; real entry is the
      shell's MainActivity) and (b) the clean content-type API needs Compose 1.8 (1.7.5's `autoCorrect`
      is deprecated) — so it lands with the real input flow + a possible Compose bump. Add a Compose
      semantics test asserting the content types when it does.
  - **WHEN:** **T13-shell** (MainActivity real input) / a Compose 1.8 bump.

- [ ] **Submit re-entrancy guard (reviewer LOW).** The router launches `viewModel.submitPhone/
      submitCode/decideNotifications` via `scope.launch`; a double-tap before the suspend resolves can
      fire two in-flight coroutines (faithful parity with iOS `Task { await … }`; harmless with the
      instant test fakes). With a slow real network this could double-submit. Disable the action while
      in-flight (or guard on a "submitting" flag) when the real networking lands.
  - **WHEN:** **T13-shell** (real `OnboardingNetworking`).

- [ ] **Optional Confirmation/Banner icon parity (platform-parity M1).** The Android `Confirmation`
      ("Automatic updates are on.") and `Banner` render text + shape/tint only; the iOS twin adds a
      `checkmark.circle` / `info.circle`. The a11y bar is already met (shape+text, not color-only — see
      the renderer comments), and adding the icons would pull `material-icons-extended` (a large dep) —
      so T13 keeps them iconless by design. If visual parity is later wanted, add the two icons (weigh
      the dep, or ship a tiny local vector) in the shell.
  - **WHEN:** **T13-shell** (only if visual icon parity is desired).

- [ ] **Recorded TalkBack walkthrough + Accessibility Scanner pass (manual).** The automated AC11 leg
      asserts the model-level reading order (labels/traits/order; the auto-update confirmation as a
      state, not a button). Paparazzi has no semantics-tree strategy, so the **recorded** TalkBack /
      Switch-Access walkthrough + the Accessibility Scanner run remain a manual checklist item (plan §7).
      Optional automation: a Robolectric + `compose-ui-test` `createComposeRule` reading-order test
      (a new dep — weigh against the model-level assertion, which already covers order).
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** The 68 baselines were recorded locally on macOS via
      Paparazzi's bundled layoutlib. layoutlib renders with its own bundled fonts (more portable than
      device snapshots), but the `android` CI job is **GitHub-only / not locally verifiable** (Ubuntu
      runner), so the first CI run is the real proof of cross-runtime text rendering; re-record from the
      runner if it diverges (a known snapshot-testing reality, as on iOS T11/T12). Paparazzi's default
      `maxPercentDifference` (0.1) applies.
  - **WHEN:** first CI run of the extended `android` job (`:rider:app:verifyPaparazziDebug` with real screens).

- [ ] **11 added catalog keys beyond the spec's 14 — product-owner review.** T13's `strings.xml` mirrors
      the iOS `RiderShared` catalog exactly (25 keys), which already includes the 11 affordance/settings +
      name-less `*_generic` fallback keys T11 added and flagged. Same wording, surfaced again here for the
      Android catalog. The two `admin_onboarding_*` + `auth_signin_again` keys are present for AC12
      completeness but rendered by the admin web / Driver, not the Rider.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording (keep in lock-step with the iOS catalog).

## Android / Driver UI (spec 001 T14 — out-of-scope register)

> T14 shipped the **Compose Driver onboarding UI slice** in `android/driver/app` (package
> `app.boundless.driver`) + the behavior-preserving extraction of the role-neutral kit into a new
> `com.android.library` **`:rider:shared`** (the Android twin of iOS's `RiderShared` library; both apps
> depend on it — an app module can't depend on another app module). The Driver reuses the shared kit and
> adds only the three deltas from T12 (self-onboard intro, one-time Recovery-Code capture, interactive
> re-auth PhoneEntry), all rendered from `core::auth` via `:core-bridge` (P4). 44 tests green (19 screens
> ×4 = 76 Paparazzi baselines + 25 logic/a11y/content); `:rider:app`'s 68 baselines + logic tests
> re-verified green **unchanged** (proof the extraction changed nothing). Same functional-core /
> imperative-shell split as T07–T13. Everything below was deliberately left out (the **T14-shell**); each
> carries a WHEN trigger.

- [ ] **The deployable launcher `MainActivity` (the Driver composition root).** Like T13, T14 ships the
      screens/model/VM/router/catalog + tests as "UI legs" with **no launcher Activity** (a
      `com.android.application` module assembles without one). The shippable `MainActivity` that
      instantiates `DriverOnboardingViewModel` with the real conformers, hosts `DriverOnboardingRouter`,
      and wraps it in `DriverTheme` — bundle id **`app.boundless.driver`** (Apple/Android register-bundle
      items) — is deferred.
  - **WHEN:** when preparing the first Driver Android build.

- [ ] **The real `RecoveryCodeProviding` + the OpenAPI Kotlin HTTP client (`OnboardingNetworking`).** T14
      ships the `RecoveryCodeProviding` interface + a `FakeRecovery`; the real impl reads
      `fresh_recovery_code` off the `/api/auth/bind-device` (and rebind) response. Likewise the real
      `OnboardingNetworking` (`openapi-generator` kotlin → `/api/auth/{signin,bind-device}`). Both deferred
      because the deployable Worker they call does **not exist yet** (T07-shell-B); building them now is
      untestable "this should work" code. They drop in behind the existing interfaces untouched.
      **Carry-forward (T14 security review, low):** the Recovery Code is a secret but is a bare `String?`
      everywhere (faithfully mirroring the iOS twin `recoveryCode: String?`), so a future caller could log
      it with no compiler friction. When the real provider lands, wrap it in a thin `RecoveryCode` newtype
      with no `toString`/`Debug` returning the raw value (the `DeviceToken`/`PhoneNumber` discipline, P2) +
      add a CI/lint grep asserting no `Log.*`/`print` of the value — and coordinate with the iOS twin so
      both platforms gain the same guard. (No live break today: zero logging in the shipped slice.)
  - **WHEN:** **T07-shell-B** (a live Worker) + the T10 Kotlin OpenAPI codegen.

- [ ] **The self-serve re-bind ENTRY UI** (phone + Recovery Code on a *new* device → re-bind, old token
      invalidated, fresh code issued). The onboarding **state machine has no recovery-rebind state** to
      render, so building a UI for it now would be UI not driven by the core (against P4) — exactly as iOS
      T12 deferred it. The AC19 server/logic legs are **done** (T04/T05/T07); T14 closed AC19's **capture**
      leg. Needs either a new core state or a separate flow.
  - **WHEN:** a recovery-rebind flow spec (or when the Driver app shell adds a "new phone" entry point).

- [ ] **`NotificationManager`/FCM + signed-manifest fetch/verify + Keystore refresh storage (§10-F).** The
      real `NotificationPermissionRequesting` (`POST_NOTIFICATIONS` on API 33+), FCM device-token
      registration, the `ManifestProviding` impl (KV manifest fetch + libsodium verify + cache, ADR-0014,
      providing `{adminName}`), and the **EncryptedSharedPreferences / Keystore** refresh-credential store
      (never plain `SharedPreferences`) — all behind injected boundaries today; the conformers are the
      shell. Same as the T13-shell items (the Driver reuses the shared boundaries).
      **Carry-forward (T14 security review, nit):** before the shell persists ANY credential, set
      **`android:allowBackup="false"`** (or a tight `dataExtractionRules`/`fullBackupContent` that excludes
      the Keystore-backed store) on **both** the Driver and Rider app manifests — Android Auto Backup
      otherwise ships app-private files to the user's Google account (a P2/I12 cloud-exfiltration vector).
  - **WHEN:** the Driver app shell / push spec **007** / **T07-shell-B**.

- [ ] **Onboarding-Code / phone field input-security + submit re-entrancy guard (same as T13-shell).** When
      the real input flow is wired (MainActivity), mark the code field as a one-time code
      (`ContentType.SmsOtpCode`, a **Compose 1.8** API) + no-personalized-learning, and the phone field
      `ContentType.PhoneNumber`, so the single-use secret and the phone number don't leak into the keyboard
      learning store (P2 spirit). Also disable the action while a `submitPhone`/`submitCode` coroutine is
      in-flight (double-tap guard). T14 wires no real input (fields are rendered in snapshots only), and the
      clean content-type API needs Compose 1.8 — so both land with the real input flow.
      **Carry-forward (T14 security review, nit):** while the **Recovery-Code capture** screen is shown,
      set **`FLAG_SECURE`** on the window (block screenshots / screen-recording of the secret) and ensure
      the code value never enters the autofill / clipboard-history learning store — same content-type
      discipline as the OTP field above.
  - **WHEN:** **T14-shell** (MainActivity real input) / a Compose 1.8 bump.

- [ ] **Recorded TalkBack walkthrough + character-by-character Recovery-Code spell-out (manual / polish).**
      The automated AC11 leg asserts the model-level reading order (incl. the code read as static text). A
      **recorded** TalkBack / Switch-Access walkthrough remains a manual checklist item; an optional polish
      is reading the Recovery Code **character-by-character** (a per-character semantics label) rather than
      as one token — weigh at the persona-acceptance/a11y review (the iOS twin flagged the same).
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** The 76 Driver baselines were recorded locally on macOS via
      Paparazzi's bundled layoutlib. The `android` CI job is **GitHub-only / not locally verifiable**
      (Ubuntu runner), so the first CI run is the real cross-runtime proof; re-record from the runner if the
      text rendering diverges (a known snapshot reality, as on iOS T11/T12 and Android T13). The CI `android`
      job must be extended to run `:driver:app:verifyPaparazziDebug` + `:driver:app:testDebugUnitTest` +
      `:rider:shared:assembleDebug` alongside the rider ones.
  - **WHEN:** first CI run of the extended `android` job; extend the job's task list with the Driver + `:rider:shared`.

- [ ] **4 added Driver catalog keys beyond the spec's table — product-owner review.** T14 added a Driver
      `strings.xml` (4 keys) mirroring the iOS `DriverOnboarding.xcstrings` verbatim:
      `onboarding_driver_intro` ("Let's get you set up."), and the Recovery-Code capture trio
      `onboarding_recovery_{title,explanation,saved}` ("Save your Recovery Code." / "You'll need this to set
      up Boundless on a new phone. Keep it somewhere safe." / "I've saved it"). All voice-and-tone-checked,
      trivially editable pre-release. Keep in lock-step with the iOS Driver catalog.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [ ] **Move the shared test resolver/fakes to AGP `testFixtures` once KGP ≥ 2.1.** T14 wanted the catalog
      resolver + fakes single-sourced in `:rider:shared`'s `testFixtures`, but **KGP 2.0.21 doesn't compile
      Kotlin in `testFixtures` source sets** (that landed in Kotlin 2.1.0; the AGP feature exists but the
      Kotlin task `compileDebugTestFixturesKotlin` is absent). 2.0.21 is pinned by Paparazzi 1.3.5 / AGP
      8.4.2, so it can't be bumped now. Fallback (shipped): `CatalogRiderStrings` lives in
      `:rider:shared/src/main` (inert in production — the shipping resolver is the deferred
      `AndroidRiderStrings`), and the trivial fakes (`FakeNetworking`/`…`) are duplicated per app test. When
      the toolchain advances (Paparazzi 2.x stable → Kotlin 2.1+), move the resolver + fakes into
      `:rider:shared/src/testFixtures` and consume via `testImplementation(testFixtures(project(":rider:shared")))`
      to drop the resolver out of `main` and de-duplicate the fakes.
  - **WHEN:** the next Android toolchain bump (Paparazzi 2.x / Kotlin 2.1+).

## Admin web / SvelteKit onboarding (spec 001 T15 — out-of-scope register)

> T15 shipped the **admin onboarding UI slice**: the SvelteKit app scaffold + the four onboarding
> surfaces (invite-link landing → WebAuthn **registration ceremony** → `InviteExpired` → WebAuthn
> **sign-in**, no password) + the i18n catalog/runtime + the §10-F session cookie, all wired to the
> T09 verification core through its **in-memory** port fakes and proven end-to-end with Playwright
> (Chromium CDP **virtual authenticator** → real bytes → real `@simplewebauthn/server`) + axe-core.
> Closes **AC2, AC11b, AC1(b-web)**. Same functional-core / imperative-shell split as T07–T12.
> 11 e2e (4 T09 + 7 T15) + 54 vitest green; `pnpm typecheck`/`build` clean; allow-list clean (5
> locks); binding-drift unchanged (68 inputs — `web/` is not a drift input). Everything below was
> deliberately left out (the **T15-shell**); each carries a WHEN trigger.

> **INVESTIGATION (2026-06-06, per user request "investigate the T15-shell more").** The shell was
> being treated as one monolithic, deploy-blocked lump ("wrangler not installed"). It is **not**:
> the load-bearing finding is that **`wrangler`'s `getPlatformProxy()` emulates Cloudflare bindings
> (KV, etc.) in-process via Miniflare/workerd with NO Cloudflare account and NO `wrangler login`** —
> it reads the bindings from `wrangler.toml`/`.jsonc` and returns `{ env, cf, ctx }`; only
> `wrangler deploy` needs an account. SvelteKit's `@sveltejs/adapter-cloudflare` itself invokes
> `getPlatformProxy()` during `vite dev`, so server code that reads `platform.env.<BINDING>` runs the
> SAME paths off-edge. (Sources, verified 2026-06-06 via docs-researcher:
> developers.cloudflare.com/workers/wrangler/api/ ; svelte.dev/docs/kit/adapter-cloudflare ;
> developers.cloudflare.com/kv/api/{write,read,delete}-key-value-pairs/.) So the shell splits cleanly:

- [x] **(A) Locally buildable + testable NOW — no account. DONE 2026-06-07.** Shipped: `KvChallengeStore`
      (`web/src/lib/server/webauthn/kv-challenge-store.ts`) over the existing `ChallengeStore` port —
      `put` with `expirationTtl` (60s-floor clamp `kvExpirationTtl`; only caller passes 300 = the ADR-0017
      D3 5-min challenge), `take` = `get('text')`→`delete` (best-effort consume-once; documented + defended
      by the one-time httpOnly cookie + `@simplewebauthn` challenge binding + sign-count). Proven against
      **real Miniflare KV with no account** via `getPlatformProxy()` (`kv-challenge-store.test.ts`:
      round-trip / consume-once / absent→null / isolation + the clamp unit). Made **live**: swapped
      `@sveltejs/adapter-node` → **`@sveltejs/adapter-cloudflare` 7.2.8** (its `vite dev` injects
      `platform.env` via `getPlatformProxy` — verified from the adapter source; binding read from
      `web/wrangler.toml`), `webauthn-deps.ts` selects `KvChallengeStore` when `platform.env.CHALLENGES`
      is bound else the in-memory fallback (Vitest/adapterless), the 3 `+server.ts` routes thread
      `event.platform`, `App.Platform` typed in `app.d.ts`. The Playwright register→sign-in ceremony now
      round-trips the challenge through real KV. Pins (lock = ground truth, exact): `wrangler` 4.98.0,
      `@sveltejs/adapter-cloudflare` 7.2.8, `@cloudflare/workers-types` 4.20260606.1 (adapter-node removed)
      → `docs/stack-matrix.md`. `web/pnpm-workspace.yaml` approves the workerd/esbuild/sharp build scripts
      (pnpm 11). Gates: `pnpm typecheck` 0/0 · `pnpm test` 66 (6 real-KV) · `pnpm build` (adapter-cloudflare,
      no account) · `pnpm test:e2e` 13 · **allow-list clean across 6 locks** (the AC13 web re-scan). Closes
      the T09-register "Real Cloudflare KV `ChallengeStore` impl" item (below). **NB the getPlatformProxy
      gotchas (for the next Worker/KV slice):** it boots an in-process workerd over a Node↔workerd loopback
      bridge — run a single instance (orphaned workerd from prior runs make it hang), don't pipe its stdout
      through `tail` (buffering), and `dispose()` it; with those, boot is ~0.1s.
- [ ] **(B) Genuinely deploy/account-blocked — rides with T07-shell-B.** The **Postgres**
      `InviteStore`/`CredentialStore` over **Hyperdrive** (`env.HYPERDRIVE.connectionString`/`.connect()`
      → a TCP Postgres driver; incl. the invite-token HMAC compare routed through the core's
      `admin_invitation_token_matches` per ADR-0017's P4 carve-out) couples to the **unbuilt Rust Worker
      runtime** (T07-shell-B) and a real Hyperdrive/Neon, so it cannot be proven this side of that. Plus
      the actual **`wrangler deploy`** (needs a Cloudflare account + OAuth/API-token) and the live
      deployed-edge E2E. `getPlatformProxy` can emulate a KV-backed *interim* `InviteStore`/`Credential
      Store` locally, but the **production** stores are Postgres (the schema + RLS already exist, T06),
      so building a throwaway KV version of them is not worth it — defer the real ones to the Worker.
      **Also fold into this deploy slice (from the leg-A review):**
      (i) **Make `challengeStore` fail-closed — DONE 2026-06-07 (own local session).** Selection is now
      the pure `selectChallengeStore(kv, fallback, allowInMemoryFallback)` in
      `web/src/lib/server/webauthn/kv-challenge-store.ts`: real KV when the `CHALLENGES` binding is present;
      the per-isolate in-memory fallback ONLY when `dev` (`$app/environment`, statically inlined to `false`
      in any `vite build` — verified in the built `chunks/webauthn-deps.js` call site
      `selectChallengeStore(platform?.env?.CHALLENGES, challenges, false)`); else it **throws** (a PII-free
      operator 500 naming ADR-0017 D3). `webauthn-deps.ts:challengeStore` delegates, passing `dev`. 3 Vitest
      cases (throws prod-no-kv / falls back dev-no-kv / real-Miniflare-KV when bound). Gates: typecheck 0 ·
      vitest 69 · `pnpm build` (no account) · e2e 13 (the dev ceremony still round-trips the KV branch — no
      dev behaviour change) · allow-list clean (6 locks). reviewer + security-auditor: ship (0
      crit/high/med). Two forward-looking notes carried to the deploy/logging slice:
        • **(F1, I10 — when the web logging backstop lands).** The throw is raised *before* the route `try`,
          so it propagates as an uncaught endpoint error → SvelteKit's default `handleError` `console.error`s
          it (the message is suppressed to the client outside dev). The message is PII-free, so **no leak
          today** — but when the **web scrubbed `emit()` sink + a `handleError` hook + the no-raw-`console`
          lint** land (T07-shell-B / T16-shell), route uncaught endpoint throws through `emit()` and add an
          I10 scrubber fixture for this exact operator string (assert zero redactions). **WHEN:** the web
          logging slice.
        • **(F2, optional hardening).** The *pure* selector is fully tested; the *wiring* (built `dev`→`false`
          so the prod-no-binding path actually throws) is asserted only by the manual built-chunk grep. A
          build-artifact test asserting `dev` is inlined to `false` at the `selectChallengeStore` call site
          (the automated form of that grep) would pin it. Not added (reviewers: not required this slice; it
          needs a prior `pnpm build`). **WHEN:** the deploy slice.
      (ii) **Generate `App.Platform` via `wrangler types`** (instead of the hand-typed `app.d.ts`) once
      the bindings multiply (the Hyperdrive binding lands here), so a `wrangler.toml` binding rename is
      caught by the type system rather than silently disabling KV (forbidden-patterns: generate env types).
      (iii) **Set `send_metrics = false` in `server/wrangler.toml`** too (the Rust Worker harness) — leg A
      added it to `web/wrangler.toml`; server/ should match for the same privacy reason (I8 spirit).
  - **WHEN:** **T07-shell-B** (the deployable Worker + Hyperdrive) / **T15-shell** deploy.
- [x] **Recommendation (flagged for the owner) — RESOLVED 2026-06-07.** Leg **(A)** shipped as its own
      local session (above). Leg **(B)** stays held until **T07-shell-B** stands up the Worker/Hyperdrive,
      so the Postgres invite/credential stores + deploy are tested against something real, not
      "this should work" code.
- [ ] **Persistent server-side session store behind the §10-F cookie.** T15 sets the httpOnly + Secure
      + SameSite=Strict admin-session cookie (proven on the wire: HttpOnly + SameSite=Strict; `secure`
      asserted in `session.test.ts`) but the session *data* lives in an in-memory map
      (`src/lib/server/session.ts`). The shell persists it (KV/Postgres) + adds expiry/rotation for the
      admin session (separate + shorter-lived than member sessions, ADR-0016).
  - **WHEN:** **T15-shell**.
- [ ] **Dev-only `/api/test/{seed-invite,reset}` seams** are gated on `$app/environment`'s `dev` (→ 404
      in any production build). They exist only because Playwright drives a separate process against the
      in-memory backend. Remove them when the real KV/Postgres backend + a proper test-fixture path land.
  - **WHEN:** **T15-shell** (with the real backend).
- [ ] **Authenticated additive backup-key enrollment + `ac20_register_passkey_and_backup_key`.** The
      invite-gated registration path is revoke-and-replace (recovery; D4). Enrolling an *additional* key
      without revoking the first needs the signed-in admin session (no invite) — the authenticated
      add-credential flow + its Playwright test. The `CredentialStore` already supports >1 active cred.
      (Carried from the T09 register; needs the post-assertion session, now set but in-memory.)
  - **WHEN:** **T15-shell** / admin settings UI.
- [ ] **Invite token in the URL path — log/`Referer` hardening at the deployable route** (T09-register
      sec-audit F1): when the real `+server.ts` invite route deploys, never emit the `{token}` segment
      to the structured log path (route through `boundless::logging::emit()`; add an I10 scrubber fixture
      for a URL-embedded opaque token) and set `Referrer-Policy: no-referrer` on the registration page.
      The opaque single-use 72h-TTL token is not a contract defect, but a live token in an access log is
      a credential-in-logs concern. (The in-memory T15 slice does no such logging.)
  - **WHEN:** **T15-shell** (deployable invite route) + the I10 scrubber suite.
- [ ] **AC11b live screen-reader pass + Lighthouse (manual/advisory).** The automated axe + keyboard +
      reflow + aria-live legs are green in CI; the a11y-bar's manual NVDA/VoiceOver walkthrough +
      Lighthouse ≥95 (advisory) remain a pre-GA checklist item.
  - **WHEN:** the persona-acceptance / a11y review pass before GA.
- [ ] **13 catalog keys (2 spec + 11 added) — product-owner review.** T15 authored
      `src/lib/i18n/catalog.ts` with the spec's two admin keys (`admin.onboarding.register_credential`,
      `admin.onboarding.invite_expired`) plus 11 affordance/status/success keys the four screens require
      (`admin.onboarding.{register_explainer,register_action,registering,registered,go_to_signin}`,
      `admin.signin.{title,explainer,action,signing_in,failed}`, `admin.home.signed_in`) — all
      voice-and-tone-checked, trivially editable pre-release. Mirrors the T11/T12 added-keys flag.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.
- [ ] **Real translations + `melt-ui`.** Only the `en` catalog ships; `gsw`/RTL (`ar`/`he`)/`zz-ZZ`
      arrive via the Weblate pipeline + signed KV manifest (ADR-0014). The pseudo-locale (`zz-ZZ`) render
      check is **T16**. `melt-ui`/Radix primitives are deferred to **spec 008** (admin dashboard
      tables/dialogs/menus) — T15's button/status screens need only semantic HTML.
  - **WHEN:** **T16** (pseudo-locale) / translation pipeline / **spec 008** (primitives).
- [ ] **Snapshot/visual baselines + CI runtime.** The new `web` CI job adds `pnpm build` + the T15 e2e
      (axe + virtual authenticator via the auto-started `vite dev`). It is **GitHub-only / not locally
      gated** (like the other CI jobs). No image snapshots are used (axe + structural assertions instead).
  - **WHEN:** first CI run of the extended `web` job.

## Cross-cutting verification (spec 001 T16 — out-of-scope register)

> T16 shipped the suite-wide invariant gates: the I10 PII **scrubber/detector** (`core/logging`) +
> the onboarding log-fixture replay (AC3); the cross-platform catalog **parity** + pseudo-locale
> **generation** gate + a **web `zz-ZZ` render** (AC12); the network allow-list **named test** (AC13);
> and verification of the existing N-2 **compat** replay as the final run (AC9). All host/CI-testable,
> **no new dependencies**. The deployable logging *pipeline* and the per-platform pixel-truncation
> *renders* were deliberately left out (each needs an unbuilt runtime); each carries a WHEN trigger.

- [ ] **`boundless::logging::emit()` sink + no-raw-`tracing` lint + Logpush/latest-run CI replay.**
      T16 ships the **detector** (`boundless_logging::detect_pii`); the deployable *sink* that routes
      every Worker log line through it before persistence, the lint forbidding direct `tracing::*`
      (I10), and the CI step that replays the **latest real run's** logs through the scrubber (vs.
      T16's fixture replay) all need the Worker runtime. **Carry-forward (already flagged at
      T07/T09/T15):** route `StoreError` and the invite-token URL segment through `emit()`, and add
      the I10 scrubber fixtures for a unique-violation `\x…` DETAIL blob + a URL-embedded opaque token.
  - **WHEN:** **T07-shell-B** (the deployable Worker + logging wiring).

- [ ] **Native (iOS / Android) `zz-ZZ` pixel-truncation snapshot variant.** T16 renders the
      pseudo-locale **without truncation on the web surface** (Playwright reflow) and proves catalog
      *completeness* across all five platforms (the parity gate), but the native ×4 a11y snapshot
      matrices (T11–T14) do not yet include a `zz-ZZ` variant. Adding one to each (swift-snapshot /
      Paparazzi) is a per-platform render task — the honest scope flag, consistent with AC11's manual
      VoiceOver/TalkBack walkthrough and AC5/AC6's OS-toggle.
  - **WHEN:** each platform's **-shell** (the app bundle / MainActivity task), or a pre-GA a11y pass.

- [ ] **Scrubber hardening — residual detector gaps** (T16 adversarial-review findings; all **latent**:
      the onboarding flow logs none of these shapes, so they are not active leaks — they matter once the
      live `emit()` sink ships at T07-shell-B). T16 already closed the two highest-impact gaps the review
      found — **dot-separated phones** (`555.123.4567`, now caught with an IPv4 exclusion) and
      **single / separate-JSON-field GPS** (`"lat":37.7749,"lng":-122.4194`, now caught). Remaining,
      deferred deliberately to avoid risky heuristics in a security-critical fn without a full hardening pass:
  - **Sub-40-char token/secret blobs.** `TOKEN_BLOB_MIN = 40` reliably catches this system's actual
    secrets (the ADR-0021 opaque **32-byte** access/refresh bearer = 43–64 chars; device tokens), but
    misses a 16-byte token (~22–32 chars) or a dashed grouped code. Lowering the bare length threshold
    would false-positive on long `SCREAMING_SNAKE` error codes / dotted event names (which the fixture
    replay contains), so it needs an **entropy / mixed-charset heuristic** (flag a ≥~22-char run with
    mixed case+digits; leave all-caps codes and all-lower event names alone), not just a smaller number.
  - **Lowercase-only street addresses** (`47 willow lane`) — `find_street_addresses` requires a
    capitalized word. Addresses enter at admin *issuance* (spec 008), not onboarding, and the tainted
    `Address` newtype is the primary control; relax case here when issuance/logging lands.
  - **Bare-dash digit-glue run-merge** — a phone glued directly onto a UUID via a bare `-`
    (`5551234567-0000-…`) merges into one >15-digit run and escapes the phone window. Contrived (real
    JSON quote-delimits fields, so all realistic adjacent forms are caught — verified); fix when phone
    detection is next reworked (re-scan sub-windows on an over-long run).
  - **General:** Unicode-confusable / homoglyph phone & email; name+DOB heuristics (today structurally
    covered by the tainted newtypes not being `Serialize`/`Display`); threshold tuning against real
    Logpush samples; and a `scrub()` that **redacts** (not just detects) for the `emit()` path.
  - **WHEN:** a privacy-hardening pass with the **T07-shell-B** `emit()` sink (when this detector first
    becomes the live backstop), extending `scrub_redteam.rs` with each closed gap's positive case.

- [ ] **Real `gsw` / RTL / `zz-ZZ` translations via Weblate + signed-KV (ADR-0014).** The `zz-ZZ`
      here is **generated** (pseudoize), not translated; real locales (incl. the shipping Swiss German
      and Arabic/Hebrew RTL) arrive through the translation pipeline + the signed KV manifest.
  - **WHEN:** the translation pipeline / manifest service spec.

## Admin member-management (spec 008 — deferred shells, recorded at T01)

> Spec 008 (admin member-management / issuance) is decomposed into T01–T11 (`specs/008-admin-member-management/tasks.md`).
> These are the **deferred shells** the plan (§10/§15) and `tasks.md` enumerate — the legs that cannot be
> built/proven host-only, plus the spec's explicit out-of-scope items. Each carries a WHEN trigger.
> Per-task out-of-scope registers (the T02/T03/… "what this slice deliberately left out") are appended as
> each task lands, matching the spec-001 convention.

- [ ] **Live `boundless::logging::emit()` sink + the member-issuance I10 scrubber fixture.** The product's
      hottest PII write path (name + address + phone + the per-Group key). The deployable scrubbed `emit()`
      sink + the no-raw-`tracing` lint are the shared T07-shell-B track (spec 001); spec 008 adds a
      **member-issuance** red-team fixture (a synthetic issuance log line carrying name/address/phone →
      assert zero PII survives the scrubber) once that sink exists.
  - **WHEN:** **spec 001 T07-shell-B** (the live `emit()` sink) + spec 008 T09 (Worker issuance logging).

- [ ] **KEK re-wrap rotation tooling + the Group-key re-encrypt Workflow (ADR-0025).** The
      **runbook-documented** procedures (`docs/runbooks/key-management.md`, authored at T01) are
      **unbuilt**: KEK rotation is cheap (re-wrap `delegated_keys.wrapped_key`, bump `kek_version`);
      Group-key rotation is the expensive re-encrypt-every-PII-row maintenance Workflow. No rotation
      *trigger* ships in spec 008 — a long-lived key relies on KEK protection + the nonce/zeroize discipline.
  - **WHEN:** a key-rotation maintenance spec (or on suspected compromise — run the runbook procedure).

- [ ] **The I12 `forget_member` sweep must cover the new PII surfaces.** When `core::deletion` is built
      (the spec 001-deferred deletion flow), `forget_member` must redact/remove `members.name_encrypted`,
      `members.address_encrypted`, and reconcile the **`audit_log` retention tension** (T03 security
      review F2): `audit_log` carries `FOREIGN KEY (member_id, group_id) REFERENCES members ON DELETE
      CASCADE` (provisional — noted in the `0011` header), so deleting a member's row would **delete**
      their audit rows — but I12 says audit logs are **kept** (legal requirement) with PII redacted. The
      deletion spec must change this (e.g. `member_id` nullable + `ON DELETE SET NULL` + an
      `Anonymous_NNNN` ref, or sever the FK) so the trail survives, and extend the I12 forgetting
      property test to assert the forgotten member's audit rows **still exist** but carry no resolvable
      PII pointer (distinguishing "redacted/retained" from "deleted"). Member deletion is **out of
      scope** for spec 008 (no soft-deactivate in v1).
  - **WHEN:** the `core::deletion` spec (extends the I12 forgetting property test to the spec-008 columns
    + the audit_log retention reconciliation).

- [ ] **`PgDeviceStore` device-token at-rest encryption is now *unblocked*** by spec 008 T02's secretbox
      primitive — the push spec (007) can encrypt the device token under the per-Group `GroupKey`
      (`encrypt_field`). (Cross-linked from the spec-001 `DeviceStore` Postgres-impl item above.)
  - **WHEN:** push spec **007**.

- [ ] **Geocoding the address → coordinates / the ETA-matrix Workflow** — architecture flow D's geocode
      trigger is owned by the **matching spec**, not issuance (the architecture doc is amended to say so).
      Spec 008 persists only the encrypted address. Other spec-008 out-of-scope items (role-swap *workflow*,
      remote-only/"join from home" mode, the O5 device-versions panel, the O7 phone-list export, bulk/CSV
      import) live in their own later specs (`spec.md` → Out of scope).
  - **WHEN:** the matching spec (004+) / the named sibling specs.

- [ ] **Carry-forwards from the T01 review (reviewer S1 / security-auditor F4):**
      (a) **`admin.member.address_invalid`** was registered in `docs/error-codes.md` at T01 *beyond* the
      spec's i18n table (which enumerated only `phone_invalid`/`duplicate_phone`/`edit_stale`) — a
      defensible addition (spec.md edge-cases names "invalid / unparseable phone **or address**"), but the
      English copy must be **confirmed with the owner** when T10 authors it (matches the spec-001
      added-keys-review convention). (b) The **`#[require_audit]` compile gate (T06, I5) must cover the
      duplicate-phone response path** specifically: `ADMIN_MEMBER_DUPLICATE_PHONE` returns an *existing*
      member's identity (a PII read), so it is an audited read, not just a not-found code — T06 must not
      let that path skip the audit obligation, and T08's contract test must assert no `/api/auth/*` shape
      can ever return `ADMIN_MEMBER_DUPLICATE_PHONE` or a member-identity field (no existence-leak
      regression).
  - **WHEN:** (a) **T10** (catalog copy); (b) **T06** (`#[require_audit]` gate) + **T08** (contract test).

- [ ] **Carry-forwards from the T02 review (security-auditor + platform-parity):**
      (a) **Decrypted-PII zeroization (MED).** `core/crypto::decrypt_field` returns a plain `Vec<u8>`;
      wiping only that transient buffer would not close the DO-memory-snapshot threat because the
      boundary re-wraps the bytes into a tainted `Address`/`MemberName` whose inner `String` is not
      zeroized (true of *every* `tainted_secret!` type today). Decide at the consuming task whether the
      tainted types should own a zeroizing buffer (so decrypted PII is wiped on drop, P3/I2) — a
      cross-cutting change to the `tainted_secret!` macro. The non-`Zeroizing` return is documented as
      intentional in `secretbox.rs`. (b) **Production-RNG guard (LOW).** `SecretSource::fresh_nonce`'s
      "no deterministic nonce" rule is enforced by docs + the convention that `RngSecretSource` is the
      only non-test impl, not by the type system. When the issuance path is wired, add an integration
      assertion that a field-encrypting endpoint constructs `RngSecretSource` (not a placeholder/seq
      impl) — mirroring the `PlaceholderSecrets::fresh_nonce` → `unreachable!` fail-closed guard.
      (c) **Wire-projection parity watch (LOW).** Keep `Address`/`MemberName` off the UniFFI/wasm
      bindings at T05; build the wire `MemberSummary`/`MemberDetail` DTOs with a fixture-vs-`api/openapi.yaml`
      contract test (the same core↔wire seam that bit the spec-001 T10 `ManifestPointer`).
  - **WHEN:** (a) **T05/T09** (the decrypt boundary) — decision recorded now; (b) **T09** (deployed
    field-encrypting endpoint); (c) **T05/T08** (the projection + contract test).

- [ ] **Carry-forward from the T04 review (reviewer L1): preserve the key-load failure *reason* on
      the operator path.** `core/server::load_group_key` collapses both failure modes — `wrapped ==
      None` (Group never bootstrapped) and an `unwrap_group_key` `Err` (wrong KEK / corrupt or
      tampered blob) — into one opaque `GroupKeyMissing` → `ADMIN_GROUP_KEY_MISSING`. That is the
      correct **wire/client** posture (revealing which is a needless signal; it denies an attacker an
      oracle at zero cost — security-auditor confirmed). But the two have **different runbook
      remedies** (`docs/runbooks/key-management.md`: "run the bootstrap" vs "fix the KEK binding"), so
      the deployable Worker that loads the KEK and calls `load_group_key` should log the underlying
      `SecretboxError` variant (`Malformed`/`Decrypt` — PII/key-free by construction, `secretbox.rs`)
      via the scrubbed `boundless::logging::emit()` path, so an operator can distinguish "never
      provisioned" from "wrong KEK / corrupt blob". The collapse must stay on the response; only the
      *log* gains the variant.
  - **WHEN:** **T09** (the Worker KEK-load + GroupKey-cache path) — rides the same `emit()` sink the
    spec-008 issuance I10 fixture needs.

## Server / core — `MemberService` (spec 008 T05 — out-of-scope register)

> T05 shipped the pure `core/server` `MemberService` decision layer (`core/server/src/member.rs`):
> issuance (validate → in-core `normalize_phone` → two-fold phone + secretbox name/address → mint
> Onboarding Code → atomic insert), edit (re-encrypt + recompute phone hash + optimistic concurrency
> *decision*), regenerate, the audited detail read + audit-log read (I5), the AC8 `MemberSummary`
> projection + the two-type `MemberDetail`/`MemberDetailView` split, `OnboardingStatus`/`AuditField`/
> `AuditEntry`, the `IssuableRole` admin-unrepresentable reject (`ADMIN_MEMBER_ROLE_FORBIDDEN`, the
> 6th issuance code), and the duplicate-phone surface-and-link (audited). All behind 3 new ports
> (`MemberStore`/`AuditStore`/`DelegatedKeyStore`) with in-memory doubles; **no new deps**;
> wasm32-clean; `.bindings.lock` regenerated (80 inputs). **Module-placement note** (like T04's
> bootstrap module): the 3 member ports live in `member.rs` (co-located with the member types), not
> `ports.rs` — `ports.rs` gained only `SecretSource::fresh_onboarding_code`. Everything below was
> deliberately left out; each carries a WHEN.

- [ ] **Decrypted-PII zeroization (T02 review carry-forward (a)) — decision recorded, change deferred.**
      `decrypt_field` returns a plain `Vec<u8>`; the boundary re-wraps it into tainted
      `MemberName`/`PhoneNumber`/`Address` (inner `String` not zeroized) and `MemberDetail::to_wire`
      makes another plain-`String` copy in `MemberDetailView`. So a detail read scatters 3+ unzeroized
      plaintext copies — the DO-memory-snapshot residual on **PII** (vs keys, which T02 zeroizes).
      Whether to give the `tainted_secret!` types a zeroizing buffer is a cross-cutting decision.
  - **WHEN:** a privacy-hardening pass / when the `tainted_secret!` macro gains a zeroizing buffer.
- [ ] **DB-level atomicity + RLS proofs (T07).** The in-memory double *models* the port atomicity
      contracts (member+code insert in one txn; audit INSERT atomic with the detail SELECT, R5;
      regenerate supersede-then-insert; optimistic `UPDATE … WHERE updated_at = $expected`). The real
      `PgMemberStore`/`PgAuditStore`/`PgDelegatedKeyStore` over real PG18 prove them (the partial-unique
      "one live code" index, the `(group_id, phone_lookup)` unique → `DuplicatePhone`, RLS isolation).
      Also the **`list_members` admin-exclusion** is an in-memory `roles.contains(Admin)` filter here;
      the SQL is `WHERE 'admin' != ALL(roles)` at T07.
  - **WHEN:** **T07** (the Postgres adapters).
- [ ] **`edit_member` has no duplicate-phone conflict outcome (T05 review, reviewer+sec LOW).** Issuance
      models a phone collision as the first-class `IssueMemberOutcome::DuplicatePhone` (audited
      surface-and-link); `EditMemberOutcome` has no analogue. An edit that moves a member's phone onto a
      number already enrolled recomputes the lookup hash and trusts the store — in Postgres the
      `(group_id, phone_lookup_hash)` partial-unique index rejects it as an opaque `StoreError` (a
      500-class failure, not the calm `admin.member.duplicate_phone` copy), and the in-memory `edit_member`
      double *silently overwrites* the `phone_index` (single-tenant, models the happy path only). At T07/T09
      decide the edit-collision contract: map the unique-violation to a clean
      `EditMemberOutcome`-conflict result (audit any existing-member disclosure as issuance does) **or**
      document it as a calm `ADMIN_MEMBER_DUPLICATE_PHONE` reject with no partial write; add a
      `pg_member_store_edit_into_duplicate_phone_*` integration test so the double's silent-overwrite
      cannot mask a regression.
  - **WHEN:** **T07** (DB conflict) / **T09** (the user-facing/audited mapping).
- [ ] **Duplicate-phone cross-surface contract (T01 review (b)).** The `DuplicatePhone` outcome +
      `ADMIN_MEMBER_DUPLICATE_PHONE` are **admin-surface-only** (R9). A T08 contract test must assert no
      `/api/auth/*` shape can return that code or a member-identity field (the no-existence-leak
      discipline holds on the member-facing endpoints). The T05 return types keep the duplicate arm in
      the admin-only `IssueMemberOutcome` (never shared with an auth shape).
  - **WHEN:** **T08** (OpenAPI freeze + contract test).
- [ ] **OpenAPI single-source pins (T08).** `OnboardingStatus` + `AuditField` wire **casing**
      (`snake_case`, pinned by T05 serde tests) mirrored verbatim into the OpenAPI enums; the
      `AuditEntry.timestamp` wire form = **epoch-seconds integer** (core `UnixSeconds(i64)`, no chrono);
      `member_id` as `{type:string, format:uuid}` (`$serde(transparent)`); `roles` reuse `Role` by
      `$ref`; and the `IssueMemberResponse` **two-arm `oneOf`** (Issued-with-code vs duplicate-with-
      summary) — the existing contract test does NOT descend `oneOf`, so T08 needs an explicit per-arm
      assertion (the `ManifestPointer`-miss class).
  - **WHEN:** **T08**.
- [ ] **Worker projection + R10 + request_id (T09).** The Worker must (a) serialize `MemberDetailView`
      via serde (`Response::from_json(&view)`), **not** a hand-rolled `json!` — `server/src/runtime/**`
      is **not** in `.bindings.lock`, so a re-typed projection is unguarded (the `ManifestPointer`
      seam); (b) keep the inbound raw `name`/`address`/`phone` off the log path + out of error responses
      (R10 — the core maps to value-free codes); (c) mint `request_id` as a **server-minted opaque id**
      (never client-echoed), so the persisted, admin-readable `audit_log` row has no PII/secret
      injection point. Plus: a **field-decrypt failure** collapses to `ADMIN_GROUP_KEY_MISSING` on the
      wire (no oracle); the Worker logs the underlying `SecretboxError` variant via `emit()` to
      distinguish "no key" from "corrupt field/blob" (extends the T04 L1 carry-forward to field decrypts).
  - **WHEN:** **T09** (the deployable Worker + `emit()` sink).
- [ ] **Server-side member search/filter (`?search=&role=&status=`).** `list_members` is param-less in
      T05 (lists all non-Admin members); the search/filter (SQL `WHERE` + the query params) is a T07/T08/
      T10 concern. The `MemberStore::list_members` signature may gain a filter param then.
  - **WHEN:** **T07/T10**.
- [ ] **`PATCH` returns the updated detail?** T05's `edit_member` returns only `{Updated, Stale,
      Rejected}` (not the detail), so editing is **not** an audited read — the UI re-fetches via the
      audited `read_detail` if it wants to show the result. Whether the wire `PATCH` returns the detail
      (and so audits) is a T08 wire-shape decision.
  - **WHEN:** **T08**.
- [ ] **Keep the new admin types off the UniFFI/wasm FFI mirror crates.** `MemberSummary`/`MemberDetail`/
      `MemberDetailView`/`OnboardingStatus`/`AuditEntry`/`AuditField`/`IssuableRole` are admin-web/TS-only
      (no proto, no UniFFI) — never add them to `core/ffi-swift`/`core/ffi-kotlin` (the parity gate would
      catch a one-sided addition, but they should simply never be mirrored).
  - **WHEN:** ongoing (a no-op to maintain; note for a future reader).

## Server / core — I5 audited-response compile gate (sealed `AuditedResponse` / `PiiDisclosure`) (spec 008 T06 — out-of-scope register)

> T06 shipped the **compile-time** I5 require-audit gate (`core/server/src/audited.rs`): the
> un-forgeable `PiiDisclosure<T>` audited carrier (`pub(crate)` ctor, delegating `Serialize`, no
> `Debug`), the **sealed** `AuditedResponse` bound + the `admin_response_body` send-seam + the
> hand-curated PII-free allowlist (`MemberSummary`/`Vec<MemberSummary>`/`Vec<AuditEntry>`), and the
> hardening of `MemberDetailView` (private fields + `pub(crate) to_wire` so the bare view is
> un-constructible/un-readable outside the core). `DetailRead::Detail` now carries
> `Box<PiiDisclosure<MemberDetailView>>`. Proven by two `trybuild` harnesses (`require_audit` +
> `member_summary_compile`, 3 compile-fail + 1 pass fixture) + the `audit.rs` `assert_*_impl` locks.
> `trybuild` 1.0.116 added (dev-only); `serde_json` promoted to a runtime dep of `core/server`.
> All host-testable; no Worker/DB needed. Everything below was deliberately left out.

- [ ] **The residual `expose_secret()` + hand-rolled-`json!` egress is NOT closed by any pure-Rust
      gate** (the design panel's adversarial finding — `expose_secret` is a deliberate escape hatch and
      `worker::Response::from_json(&impl Serialize)` is universal). T06 closes the *dominant* paths
      (can't forge the carrier; can't construct/read/serialize a bare `MemberDetailView`; can't send a
      non-`AuditedResponse` through the seam), but a future endpoint that re-`decrypt_field`s and builds
      its own body bypasses the type gate. This residual is covered by I5's **own named second layer** —
      **T08**'s `openapi_pii_handlers_all_require_audit` integration test (every OpenAPI PII handler has a
      matching audit) — plus a **T09** clippy/grep lint forbidding `Response::from_json`/`to_string`/
      `json!` on member PII in `server/runtime/**`, and the P2/I10 scrubber. Document the gate's scope in
      the route code so no one mistakes it for airtight.
  - **WHEN:** **T08** (the OpenAPI-coverage second layer) + **T09** (the deployable route lint).

- [ ] **T08 must extend the `AuditedResponse` allowlist for its new admin wire DTOs.** The allowlist in
      `audited.rs` currently blesses only the three PII-free response types that exist at T05
      (`MemberSummary`, `Vec<MemberSummary>`, `Vec<AuditEntry>`). When T08 freezes `MemberList`,
      `IssueMemberResponse`, the regenerate-code response, etc., each PII-free one must get an
      `impl AuditedResponse` (with the matching `impl sealed::Sealed`), and any that carries decrypted
      PII must flow as a `PiiDisclosure<_>` instead — never a bare `impl AuditedResponse`. The sealed
      supertrait keeps that decision in `core/server`.
  - **WHEN:** **T08** (OpenAPI freeze + the new wire DTOs).

- [ ] **The router send-seam (`admin_response_body`) is provided but not yet *consumed* (no router
      exists — that is T09).** T09 must actually serialize every admin response through it (or an
      equivalent `AuditedResponse`-bounded constructor) rather than calling `worker::Response::from_json`
      directly on a member DTO — the gate only bites code that goes through the bound. Pair with the T09
      lint above. The `#[allow(dead_code)]`-free seam compiles today because the `require_audit` pass
      fixture exercises it.
  - **WHEN:** **T09** (the deployable `/api/admin/members/*` routes).

- [ ] **`.stderr` golden re-bless on a toolchain bump.** `trybuild` normalizes `$CARGO`/`$VERSION` and
      `$N others`, so the committed `.stderr` files are stable across serde/dep bumps; but a *rustc*
      diagnostic-wording change (the repo toolchain is pinned, so this is rare) would require
      `TRYBUILD=overwrite cargo test -p boundless-server-core --test require_audit --test
      member_summary_compile` to regenerate them. Note for whoever next bumps `rust-toolchain.toml`.
  - **WHEN:** the next `rust-toolchain.toml` bump (if the compile-fail tests then mismatch).

- [ ] **A literal `#[require_audit]` proc-macro (the plan §7 "stretch goal") was NOT built.** The sealed
      `AuditedResponse` bound + the un-forgeable carrier satisfy I5's *intent* (omission is a compile
      error) without a `core/macros` proc-macro crate; the proc-macro remains an optional future
      hardening if a per-handler attribute is ever wanted.
  - **WHEN:** only if a literal attribute-macro form is later desired (not required).

## Server / store — `PgMemberStore` (spec 008 T07 — out-of-scope register)

> T07 shipped **`PgMemberStore`** (`server/store/src/members.rs`) — the Postgres adapter implementing
> all three T05 member ports (`MemberStore` + `AuditStore` + `DelegatedKeyStore`) on one struct,
> against real PG18 (migrations 0009–0011). 16 new integration tests (13 member — incl. the
> `prop_rls_isolates_random_two_group_configs` proptest + a concurrent-regenerate advisory-lock proof —
> 2 audit, 1 delegated-key) + the harness migration-count fix (8→11) that unblocked the previously-red
> existing store suites (T03 added 0009–0011 but `server/store/tests/common/mod.rs::setup()` still
> asserted 8). All via `begin()` (RLS), unnamed `query_typed*` (ADR-0024), no raw `Client`; the store
> handles only `bytea`/keyed-hashes/ids — no PII reaches it (P2). `proptest` added as a `server/store`
> dev-dep (its `proptest-regressions/` is committed manually — outside `core/**`, so the
> auto-discovery gate doesn't cover it). A 3-lens adversarial review (reviewer + security-auditor +
> platform-parity) returned **0 confirmed findings** (one doc-citation nit, fixed in-slice). Everything
> below was deliberately left out; each carries a WHEN.

- [ ] **The deployable `/api/admin/members/*` Worker endpoints (T09).** T07 is the store layer only;
      composing `MemberService` over `PgMemberStore`, loading the KEK from Secrets Store, caching the
      unwrapped `GroupKey` in the `GroupHub` DO, injecting the live CSPRNG, and the miniflare+PG worker
      tests (`worker_issue_member_round_trip`, …) are **T09**. The `MemberService`-over-`PgMemberStore`
      end-to-end (issue → encrypt → store → read_detail → decrypt over real PG) is effectively proven
      there: T05 proves the orchestration in-memory, T07 proves the store, T09 wires them. The
      `build_service` analog that constructs `MemberService<PgMemberStore, RngSecretSource, …>` with the
      KEK/HMAC `MemberConfig` mirrors `server/src/runtime/pg.rs::PgService`.
  - **WHEN:** **T09**.

- [ ] **`onboarding_status` TTL nuance.** The derived `STATUS_CASE` (`members.rs`) is **TTL-agnostic**
      (consistent with the `onboarding_consume_ignores_ttl` discipline): a live-but-past-TTL Onboarding
      Code still reads `IssuedNotOnboarded`, never `CodeExpiredOrLost`. The precise "expired (past TTL)
      vs lost (superseded/consumed-without-bind)" distinction needs server time, which the
      `list_members` / `read_member_detail_audited` ports don't pass today. Refine when the status UI
      (T10) or a status spec needs it (the derivation would take a `now` then).
  - **WHEN:** **T10** (status UI) / a status spec.

- [ ] **`edit_member` into a duplicate phone is an opaque `StoreError`, not a calm outcome (T05
      carry-forward).** T07's `edit_member` UPDATE trusts the store: moving a member's phone onto a
      number already enrolled in the Group hits the `members_group_phone_lookup_key` unique index → a
      `tokio_postgres` unique-violation → `StoreError::Db` (a 500-class failure), NOT the calm
      `ADMIN_MEMBER_DUPLICATE_PHONE`. (The in-memory `MemMemberStore` instead silently overwrites its
      phone index — the documented Pg/in-memory divergence from the T05 register.) T07 deliberately does
      NOT special-case it (the store is the wrong layer to mint a user-facing code). The clean mapping
      (catch the unique violation → an `EditMemberOutcome` conflict arm, audited like issuance, or a
      documented calm reject) + a `pg_member_store_edit_into_duplicate_phone` regression test is T09's.
  - **WHEN:** **T09** (the user-facing/audited edit-conflict mapping).

## API contracts / admin surface (spec 008 T08 — out-of-scope register)

> T08 froze the `/api/admin/*` HTTP contract: the 6 admin member paths + 12 schemas were added
> **additively** to `api/openapi.yaml` (the `/api/auth/*` freeze + ADR-0023 tests stay green), the
> `adminSharedSecret` scheme + `AdminIdHeader` param model the ADR-0026 trust boundary, PII handlers
> carry `x-requires-audit: true`, and 4 contract tests (`openapi_pii_handlers_all_require_audit`,
> `member_summary_schema_has_no_tainted_field`, `admin_issuance_error_codes_in_registry`,
> `openapi_admin_surface_has_no_admin_creation_path`) gate it. `.bindings.lock` refreshed; no new deps;
> all host-testable. Everything below was deliberately left out; each carries a WHEN.

- [x] **The hand-rolled-but-derived TS client `web/src/lib/server/members.ts` — DONE 2026-06-11 (T10).**
      Built as the pure `MembersClient` port + `WorkerMembersClient` fetch adapter (types hand-derived from
      `api/openapi.yaml`; the wire types live in the client-safe `$lib/members-types`) + a seedable
      in-memory fake + the fail-closed `selectMembersClient`, with its UI consumer (the `(app)` member
      routes) + Playwright e2e + a `members_client_request_shape.test.ts` unit. Plan §6's TS-provenance
      decision stands (hand-rolled-but-derived for v1; openapi-typescript codegen still scaffold). The live
      deployed round-trip is the T10-shell item (see the T10 register below).

- [ ] **The Rust wire response DTOs + the `audited.rs` `AuditedResponse` allowlist extension.** The new
      admin wire response shapes the Worker serializes (`IssueMemberResponseWire`/`MemberIssued`,
      `RegenerateCodeResponseWire`, the `DuplicatePhoneLink` body) live in `core/server` (the two-type split,
      built via `expose_secret` like `MemberDetail::to_wire`) and must be **blessed** on the sealed
      `AuditedResponse` allowlist (T06 register's "T08 extends the allowlist") so `admin_response_body` will
      serialize them. Deliberately kept in **T09** (where the Worker serializes them and the miniflare
      round-trip tests them), not T08, to keep T08 a pure contract slice. Pin their serde keys (mirror
      `member_detail_view_wire_keys_are_pinned`).
  - **WHEN:** **T09** (the Worker projection + miniflare tests).

- [ ] **Live deployed-edge contract-conformance for the admin surface.** The T08 tests check the contract
      *document* (+ the registry parity); replaying real admin responses against the deployed Worker to
      prove runtime conformance is the deploy-hardening pass (with T09's Worker + T11's seeded Groups).
  - **WHEN:** the deploy-hardening pass / **T11**.

## Server / Worker — admin member endpoints (spec 008 T09 — out-of-scope register)

> T09 shipped the deployable `/api/admin/members/*` + `/api/admin/audit-log` routes
> (`server/src/runtime/members.rs`) composing the **real** core `MemberService` over the **real**
> `PgMemberStore` (P4), with the live `GetrandomRng` CSPRNG injected into `RngSecretSource` (ADR-0021),
> the KEK loaded per-request + the Group key unwrapped per-request, the new core wire DTOs
> (`MemberListView`/`MemberIssuedView`/`DuplicatePhoneLinkView`/`RegenerateCodeView`/`AuditLogView`,
> blessed `AuditedResponse` so every admin response goes through the sealed `admin_response_body` seam —
> no hand-rolled member-PII JSON, I5), and the ADR-0026 shared-secret + `X-Admin-Id` fail-closed gate.
> Proven by **6 miniflare tests over real PG18** (`server/test/admin-members.spec.ts`: issue→encrypt→
> store→audited-detail-decrypt round-trip, the I5 audit row, regenerate, duplicate-phone link, no
> submitted-PII in error bodies, the 401-without-secret + admin-role-forbidden gate) + a Rust seed
> example (`server/store/examples/seed_worker_test_pg.rs`) that bootstraps the test Group. The
> wrangler-credential gate (`scripts/check-wrangler-credentials.sh` + meta-test) is wired into the
> `worker` CI job. New worker deps: `rand_core` 0.9.5 (traits) + `getrandom` 0.4.2 (`wasm_js`). Everything
> below was deliberately left out; each carries a WHEN.

- [~] **The real SvelteKit→Worker BFF call (ADR-0026).** T09 built + miniflare-tested the Worker side (the
      `ADMIN_API_SECRET` + `X-Admin-Id` are injected in tests). **T10 (2026-06-11) shipped the BFF
      adapter** (`WorkerMembersClient` presents `Bearer <ADMIN_API_SECRET>` + the verified `X-Admin-Id`;
      request shape unit-tested; selected fail-closed). **Remaining:** the **live deployed round-trip**
      (the SvelteKit Worker actually calling the deployed Rust Worker over the network) — needs the deploy
      + a Cloudflare account; the UI e2e drives the in-memory fake until then.
  - **WHEN:** the deploy-hardening pass / **T11** (see the T10 register below).

- [ ] **KEK from a real Secrets Store binding on the deployed edge (ADR-0025 R3).** `@cloudflare/vitest-
      pool-workers`/miniflare does **not** emulate a Secrets Store binding (verified via docs-researcher,
      2026-06-11), so T09 loads the KEK via `env.var("KEK")` — the same plaintext-binding mechanism
      `HMAC_KEY` already uses (a `wrangler secret` at deploy, injected in tests). Migrating `KEK` (and
      ideally `HMAC_KEY`) to a real `[[secrets_store_secrets]]` binding (`env.secret_store(...)?.get()`) is
      deferred until either the runtime emulates it locally or a deployed-edge-only test path exists. The
      deploy is operator-gated regardless (`docs/runbooks/deploy-worker.md` step 4b).
  - **WHEN:** when miniflare emulates Secrets Store / a deploy-hardening pass.

- [ ] **GroupHub DO GroupKey cache (the task-title "GroupKey cache").** T09 deliberately unwraps the
      Group key **per request** (inside `MemberService::load_group_key`), NOT a long-lived plaintext key
      cached in the `GroupHub` DO. Rationale: R2's threat is a DO memory snapshot, so caching the
      plaintext key in the DO *widens* the exposure window — per-request unwrap-then-drop (the key
      zeroizes on `Drop`) minimizes the plaintext lifetime (zeroize/P3 aligned). The DO cache is a perf
      optimization (saves one `delegated_keys` SELECT + one unwrap per request); revisit only if the
      per-request unwrap is a measured hot spot, and only with a write-through/evict-on-rotate guard.
  - **WHEN:** a perf pass, if the per-request unwrap is measured as a bottleneck.

- [ ] **Defense-in-depth: the Worker verifies the asserted `X-Admin-Id` actually holds `role=admin`
      (ADR-0026).** Today the Worker trusts the BFF-asserted admin id (the shared secret is the trust
      boundary). A cheap follow-up: a `members` lookup asserting the asserted id is a real Admin in the
      Group, so a leaked secret cannot forge an arbitrary actor on the I5 audit trail. Not required for v1.
  - **WHEN:** a security-hardening pass (with the T10 real BFF call, where the admin session is established).

- [ ] **`edit_member` into a duplicate phone returns an opaque 500, not a calm conflict (T07/T05 carry-
      forward).** Moving a member's phone onto a number already enrolled hits the `(group_id,
      phone_lookup_hash)` unique index → `PgMemberStore::edit_member` → `StoreError::Db` → the Worker maps
      it to a generic 500. The clean mapping (a distinguishable `EditMemberOutcome` conflict arm in the
      core, surfaced like issuance's `ADMIN_MEMBER_DUPLICATE_PHONE`, audited) requires a core change (the
      store would need to return a typed conflict, not an opaque `Db` error) — out of scope for the T09
      Worker slice. No data risk (the UPDATE fails atomically, no partial write); only the UX is poor.
  - **WHEN:** a focused edit-conflict slice (core `EditMemberOutcome` + a `pg_member_store_edit_into_duplicate_phone` test).

- [ ] **Server-side member search/filter (`?search=&role=&status=`).** The list route accepts the query
      filters but does **not** apply them (the core `list_members` takes no filter — the T07/T05 register's
      deferral). The list returns all non-Admin members regardless. Wiring the SQL `WHERE` + the params is
      a T07/T10 concern.
  - **WHEN:** **T07/T10** (server-side filtering + the UI that drives it).

- [ ] **Live `emit()` sink + the member-issuance I10 scrubber fixture + the StoreError/PII-off-logs
      wiring.** T09's routes keep the inbound raw `name`/`address`/`phone` off the log path (tainted types,
      value-free error codes — R10) and never echo them (the `worker_error_response_contains_no_submitted_pii`
      test). The deployable scrubbed `boundless::logging::emit()` sink + routing `StoreError` + the
      KEK-load `SecretboxError` variant (the T04 L1 carry-forward) through it + a member-issuance red-team
      fixture are the shared T07-shell-B logging track.
  - **WHEN:** **T07-shell-B** (the live `emit()` sink).

- [ ] **Deployed-edge cross-tenant proof (AC16) as the live `boundless_app` role.** T07's in-process
      `rls_isolates_member_reads_by_tenant` + the 2-group proptest prove the RLS *policy*; the live
      deployed-edge proof (a Group-A admin token cannot read Group-B members) needs ≥2 seeded Groups on the
      deployed Worker — **T11**.
  - **WHEN:** **T11** (operator-gated, with ≥2 issued Groups).

## Admin web / SvelteKit member-management (spec 008 T10 — out-of-scope register)

> T10 shipped the **admin member-management UI slice**: the authenticated `(app)` route group
> (`/admin/members`, `/admin/members/[id]`, `/admin/audit-log`) — the member list (search/filter via the
> frozen `?search=&role=&status=` params + a semantic `<table>`), the **melt-ui** add/edit dialogs +
> per-row actions menu, the audited detail read, regenerate-code, the first-class audit-log view, and the
> 17 i18n keys (+ affordance/status keys). Per P4/ADR-0026 the tier is a **BFF**: `members.ts` (the
> `MembersClient` port + `WorkerMembersClient` fetch adapter carrying the shared secret + `X-Admin-Id` +
> a seedable in-memory fake + fail-closed `selectMembersClient`) — wiring the "real SvelteKit→Worker BFF
> call" T09 handed forward. **Decided:** TanStack NOT adopted (Svelte-5 adapter beta-only — see
> stack-matrix); **melt-ui** `@melt-ui/svelte` 0.86.6 + `@melt-ui/pp` 0.3.2 preprocessor. 8 Playwright
> e2e + 2 vitest units + the catalog-parity extension; full suite **21 e2e + 93 vitest green**;
> typecheck 0/0; build clean; allow-list clean (6 locks). Same functional-core / imperative-shell split
> as T07–T12. Everything below was deliberately left out (the **T10-shell**); each carries a WHEN.

- [ ] **Live deployed BFF→Worker round-trip (the real `WorkerMembersClient` over the network).** T10
      builds + unit-tests the request shape (`members_client_request_shape.test.ts`: it sends `Bearer
      <ADMIN_API_SECRET>` + `X-Admin-Id` + the right method/URL/body, maps each frozen-contract status to
      its typed outcome) and selects it fail-closed (`selectMembersClient` throws in prod with no
      `ADMIN_WORKER_BASE`/`ADMIN_API_SECRET`), but the UI e2e drives the **in-memory fake** (no deployed
      Worker/account this side). The live deployed round-trip — the real SvelteKit Worker calling the real
      Rust Worker with the wrangler-set secret over Hyperdrive→Neon — is the deploy-hardening proof.
      Closes the T09-register "real SvelteKit→Worker BFF call (ADR-0026)" item end-to-end once live.
  - **WHEN:** the deploy-hardening pass (with the deployed Rust Worker) / **T11**.

- [ ] **`ADMIN_WORKER_BASE` + `ADMIN_API_SECRET` web bindings + the persistent admin-session store.** The
      BFF reads both from `$env/dynamic/private` (unset in dev → the fake; the wrangler secrets at deploy).
      Declaring them in `web/wrangler.toml` + `wrangler secret put` + a service binding (or the Worker URL)
      is the deploy slice. Also: the admin **session data** still lives in an in-memory map
      (`src/lib/server/session.ts`, the T15-shell carry-forward) — persist it (KV/Postgres) + add
      expiry/rotation. And remove the dev-only `/api/test/{seed-member,seed-session,reset}` seams once the
      real backend + a proper test-fixture path land (they are `dev`-gated → 404 in any prod build).
  - **WHEN:** **T10-shell** / the deploy slice (rides the T15-shell persistent-session item).

- [ ] **Server-side member search/filter is BFF-passed but Worker-NOOP.** The list `load` forwards
      `?search=&role=&status=` to `WorkerMembersClient.list` (and the **in-memory fake DOES filter**, so
      the e2e exercises the controls), but the real Worker/`core::list_members` does **not** yet apply them
      (the T07/T09-register deferral — `list_members` takes no filter). So against the live Worker the
      filters are currently inert until the core gains a `WHERE` + filter param. Wire the SQL filter + the
      core signature, then the BFF path is already correct.
  - **WHEN:** **T07/T09** core+store filter (the BFF + UI already pass the params).

- [ ] **Real `gsw`/RTL/`zz-ZZ` translations (Weblate + signed KV, ADR-0014).** Only the `en` catalog ships;
      `zz-ZZ` is generated (pseudoize), `ar` renders RTL via source-fallback. The shipping locales arrive
      through the translation pipeline + the signed KV manifest.
  - **WHEN:** the translation pipeline / manifest-service spec.

- [ ] **Manual NVDA/VoiceOver + Lighthouse pass (pre-GA).** The automated axe + keyboard-ceremony +
      400%-reflow + aria-live legs are green in CI; the a11y-bar's manual screen-reader walkthrough +
      Lighthouse ≥95 (advisory) remain a pre-GA persona-acceptance checklist item.
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **~34 catalog keys (17 spec + ~17 added) — product-owner review.** T10 authored the spec's 17
      `admin.members.*`/`admin.member.*` keys plus the affordance/status/onboarding-status/audit/nav keys
      the screens require (e.g. `admin.member.{view,edit,cancel,saving,issued,saved,actions_for,
      code_explainer,status_*}`, `admin.audit.{when,admin,member,fields,request,empty,explainer}`,
      `admin.nav.{skip,brand}`, the `*_invalid`/`roles_required`/`not_found`/`group_key_missing`/
      `error_generic` error copy) — all voice-and-tone-checked, trivially editable pre-release. Mirrors the
      T11/T12/T15 added-keys flag. The `admin.member.address_invalid` copy is the T01-review owner-confirm
      item (now authored).
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [ ] **The `/admin` placeholder home is unchanged (no redirect to `/admin/members`).** T10 left the
      spec-001 `/admin/+page.svelte` ("You're signed in.") as-is (non-breaking — the T15 e2e asserts that
      copy) and added the member surface under `/admin/members`. A future polish: redirect `/admin` →
      `/admin/members` post-sign-in (or make the home a dashboard). Not needed for v1.
  - **WHEN:** a navigation-polish pass (optional).

- [ ] **melt-ui dialogs render client-only (SSR caveat).** The add/edit dialog + menu content is behind
      `{#if $open}` and mounts on the client (melt builders + `use:melt` need the browser); the axe test
      opens the Add dialog and re-runs axe to cover its a11y, but the dialog markup is not in the SSR HTML.
      Fine for modals; noted so a future reader doesn't expect server-rendered dialog content.
  - **WHEN:** N/A (documented behavior).

- [ ] **Carry-forwards from the T10 review (reviewer — H1 fixed in-slice; the rest non-blocking).**
      The H1 finding (`roleKey('admin')` mislabelled a dual-role member's Admin badge as "Rider") was
      **fixed in-slice** (exhaustive `roleKey` + an `admin.member.role_admin` "Admin" key). The reviewer's
      remaining non-blocking findings, deferred with rationale: (M1) **regenerate-code on a missing Group
      key surfaces an opaque 500**, not the calm `admin.member.group_key_missing` copy the other three
      member endpoints surface — the frozen contract documents only `200/404/401` for regenerate (no 503),
      so the BFF throws on a Worker 500; fix is Worker/contract-side (add a 503/`ADMIN_GROUP_KEY_MISSING`
      arm to regenerate + a `group_key_missing` `RegenerateOutcome`). (M2) the `?edit=1` deep-link
      auto-open uses `onMount` → opens only on a **full** navigation (the always-available "Edit" button is
      the non-JS affordance); a `$effect` keyed on `page.url` would also re-open on SPA nav. (M3) the
      melt `$description` is attached to a branch-transient element in the add dialog (melt re-registers on
      remount; low impact) and to the member name in the edit dialog (identifies the subject but isn't a
      "description") — anchor it on a stable element / use a real description key for cleaner a11y wiring.
      (L2) `WorkerMembersClient` parses `res.json()` on 400/409 without a try — a non-JSON error body would
      throw an uncaught error instead of the value-free `fail()`; the Worker always returns JSON `{error_
      code}` so it's latent. (L3) the server-side BFF `fetch` has no `AbortSignal.timeout` — a hung Worker
      stalls the request until the platform timeout.
  - **WHEN:** M1 — a contract+Worker regenerate-503 slice; M2/M3 — an a11y-polish pass; L2/L3 — the
    deploy-hardening pass (the Worker is ours, so both are latent).

## Constitution

- [ ] **Replace `Ratified: TODO`** in `.specify/memory/constitution.md` with a
      real date.
  - **WHEN:** when you formally adopt the constitution.
