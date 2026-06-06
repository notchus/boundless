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
    `petgraph`, …) remain TODO until those parts are initialized.

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

- [ ] **Per-Group sealed-box / secretbox PII encryption (I1).** dryoc-based encryption of
      `Address` (and any field-level PII) at rest. The `core/crypto` doc still names this as
      the crate's eventual scope, but T03 does **not** implement it (addresses are entered at
      admin *issuance*, not on the onboarded device).
  - **WHEN:** **spec 008** (admin member-management / issuance), where `Address` persistence
    and the per-Group key + KEK (Secrets Store) land. Adds the `i1_addresses_encrypted` test.

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
  - **CI gate to add (sec-audit F6, T07-shell-B slice 2).** The "no ambient randomness in `core`"
    invariant is currently enforced only by a *manual* `cargo build --target wasm32-unknown-unknown
    -p boundless-server-core`/`-p boundless-crypto`. Two `rand_core` majors now coexist in the lock
    (`0.9.5` — the slice-2 pin, traits-only/feature-empty on the non-dev path — and `0.10.1`, pulled
    by dryoc's `rand`). The separation is clean *today* (`cargo tree -p boundless-server-core
    -e no-dev -i getrandom@0.3.4` prints nothing), but nothing *gates* it: a future `cargo update` or
    a dep that flips `rand_core/os_rng` could pull `getrandom 0.3.4` onto the production wasm path and
    silently break the invariant. **Add a CI step** that (a) builds `boundless-server-core` +
    `boundless-crypto` for `wasm32-unknown-unknown`, and (b) asserts the only non-dev `getrandom` edge
    is the known dryoc-`0.4.2` (`wasm_js`-shimmed) one. Not added here (GH-Actions-only; not locally
    verifiable — same constraint as the other CI items).
  - **WHEN:** next CI-hardening pass / **T07-shell-B**.

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

- [ ] **P9 process: guarantee `core/<crate>/proptest-regressions/` is tracked in CI.** Noted in
      the T05 review (applies crate-wide, predates T05): proptest writes a failing-seed file only
      *on failure*, into `proptest-regressions/` (currently **not** gitignored — good, so the
      commit-on-failure workflow is sound). But nothing yet *forces* the directory to be tracked,
      so a first CI failure could write a seed to an untracked path and lose it. Add a tiny CI
      check (or a committed `.gitkeep`) so the P9 "reproducible seeds checked into the repo"
      guarantee is enforced rather than conventional.
  - **WHEN:** next CI-hardening pass (not blocking; no property has failed yet).

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

- [ ] **T07-shell-B — the deployable workers-rs Worker (the async-port bridge is now DONE, above).**
      The `#[event]`/ Router entry point, the `GroupHub` Durable Object (persisting `GroupHubState`),
      and the Cloudflare bindings — Queues (admin alerts), KV (manifest + per-Group `{adminName}`),
      Turnstile (code-guess + refresh throttle), Hyperdrive → Postgres. Drives the **already-built,
      now-`AuthStore`-implementing `PgAuthStore`** over a `hyperdrive.connect()` `worker::Socket` —
      which requires (i) resolving the `tokio-postgres` **wasm32 feature flags** and (ii) the
      **pooler-safe `query_raw`** path (the Hyperdrive pooler dislikes tokio-postgres unnamed prepared
      statements; native tests don't hit this). The async-port bridge it needed is **done** (ADR-0020),
      so the Worker composes `PgAuthStore` (`AuthStore`) with a **`PgDeviceStore`** (`DeviceStore`) —
      the latter needs the spec-008 device-token encryption (see the device-token deferral below); until
      then the device half has no Postgres impl. The production **CSPRNG `SecretSource`** is **DONE**
      (`RngSecretSource`, slice 2) — the Worker only **injects** a getrandom-backed RNG into it.
      **APNs/FCM** device-token registration, the `RefreshResponse::server_verdict` → PII-free `emit()`
      logging (never returned to the client), and the **per-source refresh-rejection 429** (the
      `GroupHubState` counter exists; the network enforcement is the shell's).
      - **Access-token verify path (ADR-0021; the mint side is DONE in slice 2):** the access token is an
        **opaque-random bearer** (no JWT, no signing key). It is minted by `RngSecretSource` and its
        at-rest hash primitive (`access_token_hash`/`_matches`) ships in slice 2; the Worker must add the
        **`access_token_hash bytea` column + the per-request verify lookup** (a migration + a `PgAuthStore`
        method) that re-reads the family's mutable status each request. **Guard-rail (ADR-0021):** this
        lookup must NOT be a naive standalone Neon round trip — fold it into the request's existing
        group-scoped RLS txn, or serve `token-hash → family_status` from `GroupHub` DO in-memory state, and
        **on any revoke the DO/Worker cache must write-through/evict** (authoritative-on-revoke, not TTL),
        or it recreates the stale window the opaque format exists to avoid. (The Worker authenticates
        *before* the DO RPC, so "fold into the DO" = fold auth into that RPC, not a separate pre-call.)
      - **W2 boot-guard call site (the guard fn is DONE in slice 2):** the Worker must call
        `ensure_least_privilege(&client)` immediately after `hyperdrive.connect()`, before constructing any
        `PgAuthStore`, and **fail closed** on `Err` (+ a CI smoke test). The reusable function + both-legs
        real-PG test already exist; only the boot-time *invocation* + infra role provisioning remain.
  - **Needs `docs-researcher`:** the workers-rs runtime (`worker`/`worker-build` versions, the
    `#[event]`/Router/DO/`hyperdrive.connect()` Socket API) + the miniflare/workerd test harness —
    then pin `worker`/`worker-build` and fill `docs/stack-matrix.md`. (`tokio-postgres`/`tokio` are
    already pinned by slice A.) Toolchain note (verified 2026-06-05): `wrangler` + `worker-build` are
    **not installed** in the dev env, and the only supported Worker test path is a **JS/TS miniflare**
    harness — both are part of this slice's setup cost.
  - **WHEN:** the T07-shell-B infra task (after slice A).

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
      at-rest encryption does not exist yet** (the I1-adjacent sealed-box crypto is deferred to spec
      008; the push registration itself to spec 007). The DeviceToken is PII (P2) — storing it without
      encryption would violate "encrypt before writing." The Worker (T07-shell-B) and the orchestration
      tests currently compose `PgAuthStore` with an **in-memory** `DeviceStore`; implement the Postgres
      `PgDeviceStore` when the encryption primitive lands. **Guard-rail (sec-audit F3):** when it lands,
      assert at the SQL layer that the token column is `bytea` (the `_encrypted` contract) and add a
      `static_assertions` check that any persisted device-token wrapper exposes no `Serialize`/`Display`
      — so the test doubles' in-memory convenience (which holds the raw `DeviceToken`) can never be
      mistaken for the production storage shape.
  - **WHEN:** push spec **007** / issuance spec **008** (whichever brings device-token encryption).

- [ ] **wasm32 feature flags + pooler-safe `query_raw`.** Slice A uses `tokio-postgres` with default
      (native) features and idiomatic `query`/`execute`/`transaction`. The Worker (T07-shell-B) must
      (i) build `tokio-postgres` for `wasm32` configured to use a `worker::Socket`, and (ii) use the
      **pooler-safe** query path (`query_raw` / simple protocol) — the Hyperdrive pooler dislikes
      tokio-postgres unnamed prepared statements. Native tests don't exercise this; it is an explicit
      slice-B risk (ADR-0019).
  - **WHEN:** **T07-shell-B**.

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
  - **WHEN:** **T07-shell-B** / infra (DB role).

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

- [ ] **Real Cloudflare **KV** `ChallengeStore` impl.** The production one-time-use, 5-min-TTL challenge
      store on KV (ADR-0017 D3). T09 ships the port + the consume-once/TTL semantics (proven against the
      in-memory fake); the KV binding is the shell's.
  - **WHEN:** **T15 / T09-shell** (KV binding).

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

- [ ] **`core/ffi-kotlin` ⇄ `core/ffi-swift` surface parity is a convention, not yet a gate.** The two
      mirror crates MUST stay identical (same enums/variants/fns), enforced today only by the shared core
      they both mirror (a core change breaks both compiles) + the `platform-parity` review. A cheap CI
      guard (e.g. assert the exported fn/enum sets match across the two crates) would make the lock-step
      mechanical rather than reviewer-dependent.
  - **WHEN:** next CI-hardening pass.

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

- [ ] **(A) Locally buildable + testable NOW — no account (the clean next session).** Add `wrangler`
      as a devDep (+ a `wrangler.toml` with a `[[kv_namespaces]]` binding `CHALLENGES`), swap the
      in-memory `ChallengeStore` in `web/src/lib/server/webauthn-deps.ts` for a **real KV
      `ChallengeStore`** — `put(key, json, { expirationTtl: 300 })` (note: `expirationTtl` is in
      **seconds, minimum 60** → 300 = the ADR-0017 D3 five-minute one-time challenge), `get(key,'json')`,
      `delete(key)` to consume-on-first-use — and **test it via `getPlatformProxy()` in Vitest** (real
      Miniflare KV, no account). Also locally doable: swap `@sveltejs/adapter-node` → `@sveltejs/
      adapter-cloudflare` and type `App.Platform` (`platform.env.{CHALLENGES,HYPERDRIVE,…}`) in
      `src/app.d.ts`. After installing `wrangler`, **re-run `scripts/check-network-allowlist.sh`** over
      the expanded `web/pnpm-lock.yaml` (I8/AC13). This closes the T09-register "real KV ChallengeStore"
      item without any edge deploy.
  - **WHEN:** a cleanly-scoped **fully-local next session** (no account, no Worker runtime needed).
- [ ] **(B) Genuinely deploy/account-blocked — rides with T07-shell-B.** The **Postgres**
      `InviteStore`/`CredentialStore` over **Hyperdrive** (`env.HYPERDRIVE.connectionString`/`.connect()`
      → a TCP Postgres driver; incl. the invite-token HMAC compare routed through the core's
      `admin_invitation_token_matches` per ADR-0017's P4 carve-out) couples to the **unbuilt Rust Worker
      runtime** (T07-shell-B) and a real Hyperdrive/Neon, so it cannot be proven this side of that. Plus
      the actual **`wrangler deploy`** (needs a Cloudflare account + OAuth/API-token) and the live
      deployed-edge E2E. `getPlatformProxy` can emulate a KV-backed *interim* `InviteStore`/`Credential
      Store` locally, but the **production** stores are Postgres (the schema + RLS already exist, T06),
      so building a throwaway KV version of them is not worth it — defer the real ones to the Worker.
  - **WHEN:** **T07-shell-B** (the deployable Worker + Hyperdrive) / **T15-shell** deploy.
- [ ] **Recommendation (flagged for the owner).** Do leg **(A)** as its own local session — it is
      self-contained, account-free, and turns the WebAuthn challenge store from a stub into the real KV
      one with a Vitest proof. Hold leg **(B)** until T07-shell-B stands up the Worker/Hyperdrive (so the
      Postgres stores + deploy are tested against something real, not "this should work" code). This
      session delivered the **investigation only** (the user said "investigate"); no T15-shell code was
      written here — bundling a `wrangler` install + KV leg on top of the Android bring-up would blow the
      context budget and mix two slices.
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

## Constitution

- [ ] **Replace `Ratified: TODO`** in `.specify/memory/constitution.md` with a
      real date.
  - **WHEN:** when you formally adopt the constitution.
