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

- [ ] **Re-confirm the network allow-list (AC13/I8) as the web dep tree grows.** At T01
      the web tier IS now scanned: pinning `@simplewebauthn/server` produced a committed
      `web/pnpm-lock.yaml`, and `scripts/check-network-allowlist.sh` scans it (currently
      clean — no trackers). When T15 builds the SvelteKit app and `pnpm install` expands
      that lock with the full dep tree (SvelteKit, Tailwind, Vitest, Playwright, axe-core,
      …), re-run/confirm the allow-list still passes and tighten patterns if needed.
  - **WHEN:** spec 001 **T15** (SvelteKit admin web).

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

- [ ] **Live deployed-edge E2E + the `webauthn-rs`-sidecar fallback (ADR-0017).** A smoke test against the
      deployed SvelteKit Worker (Miniflare/workerd), and — only if `@simplewebauthn`'s "unofficially
      supported" Workers status ever breaks — the documented fallback to a native `webauthn-rs` sidecar.
      Not built now.
  - **WHEN:** the deploy/CI-hardening pass (with T07-shell-B / T15) — or if the edge runtime breaks.

## Constitution

- [ ] **Replace `Ratified: TODO`** in `.specify/memory/constitution.md` with a
      real date.
  - **WHEN:** when you formally adopt the constitution.
