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

- [ ] **Atomic supersede-then-insert for the four partial-unique indexes.** The schema *enforces*
      "at most one live row" via partial unique indexes on `onboarding_codes`/`recovery_codes`
      (one live code per member), `sessions` (one current credential per family), and
      `admin_invitations` (one live invite per admin). A regenerate/rotate that inserts the new row
      before superseding the prior in the **same transaction** will hit a unique violation — T07/T08
      must order it supersede-then-insert atomically (the DB twin of T04's "atomic consume-on-accept"
      carry-forward). (reviewer / sec-audit R5-adjacent)
  - **WHEN:** **T07** (code/session rotation) and **T08** (admin invite re-issue).

- [ ] **`audit_log` table + admin-PII-read audit (I5).** This slice provides only `created_by`
      (write-side actor). The `audit_log` table and the `#[require_audit]` read-path obligation must
      exist before any endpoint returns `phone_encrypted` to an Admin. (sec-audit R9)
  - **WHEN:** **T07** / **spec 008** (admin member-management).

- [ ] **sqlx `Migrator` programmatic up/down integration harness + the `sqlx` pin.** T06 is
      zero-dependency on purpose (filenames are already sqlx-reversible-compatible — `NNNN_*.{up,down}.sql`,
      verified via docs-researcher). When T07 pulls in `sqlx` for queries, adopt `sqlx::migrate!`
      and a programmatic up/down test, **pin `sqlx` (latest published = 0.8.6)** in the manifests,
      and **fill `docs/stack-matrix.md`** (the `sqlx` row is named but unversioned). Keep LF line
      endings (CRLF changes sqlx's migration hash).
  - **WHEN:** **T07** (first real Postgres queries).

- [ ] **PostGIS / `pgcrypto` extensions + `address_encrypted` + per-Group key/KEK columns (I1).**
      Onboarding tables have no geometry and no address (crypto is core-owned, §10-H). Address
      persistence, the per-Group encryption key, and the KEK (Secrets Store) land with issuance.
  - **WHEN:** **spec 008** (admin issuance) — adds the `i1_addresses_encrypted` enforcement.

- [ ] **Actual row writes** (group/member issuance, sign-in lookup, device-bind, refresh rotation,
      recovery re-bind, admin invite mint/consume) — the schema defines the columns; the writes are
      the endpoint slices.
  - **WHEN:** **T07** (member auth) / **T08** (dev admin-create + invite) / **spec 008** (issuance).

---

## Constitution

- [ ] **Replace `Ratified: TODO`** in `.specify/memory/constitution.md` with a
      real date.
  - **WHEN:** when you formally adopt the constitution.
