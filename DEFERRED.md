# Boundless — Deferred Work

> **Open work only.** This is the living checklist of things **decided but not yet
> done** — skimmed at the start of every session, and auto-loaded into context, so
> it is kept lean on purpose.
>
> **Full history → `docs/deferred-archive.md`** (NOT auto-loaded): every completed
> item, plus the full rationale / dates / review notes behind the open items below,
> organized under the *same section headings*. When you need the "why" or the "what
> shipped", read the matching section there.
>
> **Conventions:**
> - When a decision can't be acted on now, add it here with a **WHEN** trigger — not
>   as a `// TODO` in code (the hooks reject those), and not left to memory.
> - Keep each entry terse: the **bold title**, one line of essence, the WHEN. The
>   detail lives in the archive.
> - **When an item is done, move its full note to the archive — don't mark `[x]` and
>   leave it inline here.** (That is what made this file balloon before.)
> - At session start: skim this file; if a WHEN trigger has arrived, act on it or
>   surface it.

---

## Licensing

- [ ] **App Store additional-permission exception (AGPLv3 §7)** — add as a `LICENSE-EXCEPTION` file (AGPL/GPL conflicts with Apple's EULA; sole copyright holder grants the exception, Signal's model).
  - **WHEN:** before preparing the first iOS build.

- [ ] **DCO or lightweight CLA for outside contributors** — keep licensing manageable as the project takes contributions.
  - **WHEN:** before accepting the first external pull request.

---

## Apple

- [ ] **Critical Alerts entitlement** — SUBMITTED and pending Apple review. Bundle ID: `app.boundless.rider`.
  - **WHEN:** watch for Apple's email; respond promptly to any follow-up questions.

- [ ] **Register the Driver app Bundle ID:** `app.boundless.driver`.
  - **WHEN:** when starting the Driver app.

- [ ] **Generate APNs `.p8` key** — note the Key ID + Team ID, store in Cloudflare Secrets Store.
  - **WHEN:** before implementing push notifications.

---

## Cloudflare / Infra

- [ ] **Create FCM service account JSON** for Android push; store in Cloudflare Secrets Store.
  - **WHEN:** before implementing Android push.

- [ ] **Store Cloudflare API token in GitHub Actions secrets** for CI deploys.
  - **WHEN:** setting up the deploy workflow.
  - **Note:** Not needed for local MCP — that uses OAuth.

- **Cloudflare API MCP is authorized READ-ONLY by design.** Infra mutations go through Wrangler (human/CI gate), not the agent. If a task needs MCP write access, re-auth with Custom scopes for just that task, then revert. Never grant standing full access.

- [ ] **Fill in the TODO version numbers in `docs/stack-matrix.md`** with the actual installed versions.
  - **WHEN:** as each part of the stack gets initialized.
  - Remaining TODO: Swift, Kotlin, TypeScript, Xcode, Android Studio, pnpm, and Rust deps `uniffi`, `tokio`, `chrono`/`time`, `geo`, `petgraph` — fill as each part is initialized.

- [ ] **Re-pin `dryoc` to 0.9.0 (or the then-latest)** once it is *published* to crates.io (current pin is the latest published release 0.8.0).
  - **WHEN:** when implementing `core::crypto` (T03), check crates.io for a newer published dryoc and bump if available; update `docs/stack-matrix.md` to match the lock.

---

## Auth / Onboarding (spec 001 plan deferrals)

- [ ] **Two remaining new privacy-invariant tests — implement WITH their code** (P9: the implementing test ships in the same PR): (2) extend the I12 forgetting property test to the new auth artifacts (phone hash + ciphertext, device tokens, sessions/refresh, outstanding Onboarding / Recovery codes, admin WebAuthn creds); (3) a named delete-leg device-token invalidation test, distinct from `i4_tokens_invalidated_on_reonboarding` and `…_on_logout`.
  - **WHEN:** implementing `core::deletion` (account-deletion flow out of scope for spec 001 — spec §Out of scope).

- [ ] **Critical Alerts capability-upgrade path** — onboarding currently requests *standard* notifications (interim, spec 001 OQ6); upgrade the rider doorbell path to Critical Alerts once the entitlement lands.
  - **WHEN:** Apple approves the Critical Alerts entitlement (see **Apple** section).

- [ ] **Admin WebAuthn verification host** — if the decision lands on a native Rust sidecar (`webauthn-rs` can't run in Workers wasm — `openssl-sys` C-FFI), that adds one always-on service to deploy/monitor; if it lands on edge-TS, no infra is added.
  - **WHEN:** resolved by the in-flight edge-TS verification → ADR-0017.

---

## Crypto / `core::crypto` (spec 001 T03 — out-of-scope register)

- [ ] **Per-Group field-level PII encryption (I1).** dryoc-based at-rest encryption of `Address` (and any field-level PII), governed by ADR-0025 → **secretbox** (symmetric XSalsa20-Poly1305; sealed boxes reserved for I9's live tracker).
  - **WHEN:** **spec 008 T02** (load-bearing slice): `core/crypto/src/secretbox.rs` (`encrypt_field`/`decrypt_field`, `GroupKey`/`Kek` wrap/unwrap, zeroized) + tainted `Address`/`MemberName` + `core/crypto/tests/invariants.rs::i1_addresses_encrypted`/`i1_name_encrypted`. Per-Group key + KEK (Secrets Store) wiring + `Address` persistence land across T03 (columns)/T04 (bootstrap)/T07 (DB)/T09 (Worker KEK binding).

- [ ] **Onboarding/Recovery code generation, TTL, rate-limit, single-use + regenerate-invalidates-prior.** T03 ships only the at-rest hash primitive (`*_code_hash`/`*_code_matches`); the lifecycle/validation logic is separate.
  - **WHEN:** **T04** (`core::auth` code logic, server-time semantics) + **T07** (server enforcement). Tests: `prop_onboarding_code_single_use_ttl_ratelimit`, `ac17_*`.

- [ ] **Phone-number normalization (E.164) before hashing.** The caller must normalize so the same human phone always yields the same `phone_lookup_hash` (`core::crypto` hashes the exact bytes handed in).
  - **WHEN:** **T04** (`core::auth`) / **T07** (server sign-in lookup path).

- [ ] **`HmacKey` provisioning + rotation.** Load the per-instance secret from Cloudflare Secrets Store + any rotation policy (infra/server, not core; no hardcoded secrets).
  - **WHEN:** **T07** (server) + infra (Secrets Store wiring).

- [ ] **Manifest-mint Worker (server-side signing) + signing-key management + bundled public key.** The production Ed25519 signing Worker, the signing key in Secrets Store, quarterly rotation, and embedding the trusted public key in each client binary. The signer MUST canonicalize identically to `canonical_manifest_bytes` (sorted-key compact JSON) AND keep the manifest integer-only — no floats (a float field is a breaking change to the signing contract).
  - **WHEN:** a server/infra task for ADR-0014's manifest pipeline (surface when the manifest service is built).

- [ ] **Key/secret zeroization on drop.** `HmacKey` bytes are not zeroized on drop (deferred as hardening; consider `zeroize`).
  - **WHEN:** a crypto-hardening pass before GA.

- [ ] **Workspace RNG-backend policy.** Decide workspace-wide whether to keep `getrandom`'s `wasm_js` backend (for the server's eventual real RNG) or install a custom erroring backend until then, to keep "no ambient randomness" literally enforced. See ADR-0018.
  - **WHEN:** **T07** (server), when server-side randomness is first genuinely needed.
  - [ ] **Extend the gate's `CRATES` coverage (sec-audit F1, reviewer).** The no-getrandom gate audits `boundless-server-core` + `boundless-crypto`; NOT covered: `boundless-ffi-wasm`, `boundless-sync`, `boundless-logging`, and `server/`'s deployed `boundless-worker` (F2) — all dependency-free today (latent gap). Add each to `CRATES` when it gains a getrandom edge — `ffi-wasm` at T10 is highest-value. Decide per-crate: carry-the-shim → add as-is; truly randomness-free → assert no getrandom at all. For `server/`, ADR-0021 allows the Worker a real CSPRNG, so the invariant there is narrower (core crates compiled into the Worker stay injection-only).
    - **WHEN:** **T10** (ffi-wasm) / when sync·logging·worker gain deps / a CI-hardening pass.

- [ ] **`core/crypto/tests/invariants.rs` enumerating *every* privacy invariant (P9 goal).** T03 covers I3 (+ AC10 manifest tiers); I1/I2/etc. get their named tests when their primitives exist.
  - **WHEN:** as each invariant's primitive lands (I1 → spec 008; I2 → matching, spec 004+).

---

## `core::auth` (spec 001 T04 — out-of-scope register)

- [ ] **Server-time enforcement of the code lifecycle.** Server feeds the core decision server time, persists `onboarding_codes`/`recovery_codes` rows, runs the rate-limit window bookkeeping (5 attempts / 15 min), wires Turnstile, emits the lock admin alert. Carry-forwards from the T04 security review (must land in T07): (a) **atomic consume-on-accept** — mark `consumed`/`superseded` transactionally with the accept, or two concurrent presentations of one live code could both see `Accepted`; (b) **sign-in response-timing/shape parity** for matched-vs-unmatched phone (no existence leak — response must not branch on existence); (c) the production `Clock` impl must supply server time (a device clock re-enters the threat model). T07 should short-circuit via `recovery_available_for` before loading a challenge.
  - **WHEN:** **T07** (member-auth endpoints + DO) and **T06** (the code tables).

- [ ] **N-2 support policy across a *major* version bump.** `minimum_supported(current, n)` floors within the current major, so shipping `2.0.0` drops every `1.x` client below the window — tensions with P13/O1. Make the across-major support policy an explicit ADR decision before the first `2.0.0`.
  - **WHEN:** before shipping a `2.0.0` server (or when the compat harness, AC9, first spans a major bump).

- [ ] **UniFFI export of the `core::auth` surface.** T04's types are UniFFI-shaped but carry no `#[uniffi::export]`/UDL yet — codegen to Swift/Kotlin is the T10 contract-freeze. The injected `Clock` stays server-side (not on the client UniFFI surface).
  - **WHEN:** **T10** (API contracts + generated bindings).

- [ ] **`chrono`-vs-`time` crate decision.** T04 uses a homegrown `UnixSeconds(i64)` + a `Clock` trait. Pick the crate (file ADR if both used) when real wall-clock/formatting/parsing is first needed.
  - **WHEN:** **T07** (server, real server-time) or the first locale-aware time display.

- [ ] **Promote `Clock`/`UnixSeconds` to a shared crate** if `core::sync`/`core::server`/matching need the same time abstraction (today it lives in `core::auth`).
  - **WHEN:** when a second crate needs an injected clock.

---

## `core::auth` (spec 001 T05 — out-of-scope register)

- [ ] **Server-side refresh persistence + lineage classification.** The server owns the Postgres `sessions` rotation lineage chain, the refresh credential's at-rest HMAC hashing, and the DB lookup + constant-time compare that classifies a presented credential as `Current`/`Superseded`/`Unknown`; the replay→kill-family verdict must be persisted atomically with the family revoke. Carry-forwards (must land in T07): (a) rate-limit `/api/auth/refresh` on `Rejected`/`Unknown` per source (mirror R4) with a timing/shape-identical rejected response (no lineage-existence leak, sec-audit F1); (b) atomic rotate-vs-replay must resolve to a revoked family, never a second valid rotation (TOCTOU; integration test `concurrent_rotate_and_replay_resolves_to_revoked`); (c) classification correctness — a credential rotated N times ago classifies as `Superseded` (replay kills), not `Unknown`; (d) family-kill persistence — assert `sessions.revoked_at` written and the legitimate current credential rejected on next refresh (AC18); (e) `AUTH_DEVICE_TOKEN_INVALIDATED` is silent (no catalog key) — assert logged/audited but never surfaced to the client.
  - **WHEN:** **T07** (member-auth endpoints + DO) and **T06** (the `sessions`/`device_tokens` tables).

- [ ] **Multi-device-per-member policy for the `PriorDevice` invalidation scope.** T07 must decide whether a member may hold multiple concurrent device bindings and, if so, enumerate **all** prior bindings on re-onboard/revoke rather than a single `prior` (else a stale token survives). Test (T07): `reonboarding_with_multiple_prior_bindings_invalidates_all` (or assert the documented single-device constraint). (sec-audit F5.)
  - **WHEN:** **T07** (member-auth endpoints + DO).

- [ ] **Access-token issuance/signing + the ~15-min wall-clock TTL.** Minting/signing the token and supplying real server time are server concerns (T05 models only the expiry instant + `needs_refresh`).
  - **WHEN:** **T07** (server, real server-time — ties into the `chrono`-vs-`time` pick).

- [ ] **Actual push device-token registration (APNs/FCM).** Registering/deregistering the real push token with APNs/FCM and persisting it is server + platform work.
  - **WHEN:** the Doorbell push spec (**007**) / **T07**.

- [ ] **UniFFI export of the session/device surface.** Add `#[uniffi::export]`/UDL for `Session`/`RefreshVerdict`/`DeviceBinding`/… — codegen to Swift/Kotlin.
  - **WHEN:** **T10** (API contracts + generated bindings).

- [ ] **`SecureStoreClass` wiring per platform (plan §10-F).** The actual platform secure-store reads/writes of the `RefreshToken` (Keychain / Keystore / httpOnly-Secure-SameSite cookie) are the UI tasks.
  - **WHEN:** **T11–T15** (the five UIs).

---

## Server / migrations (spec 001 T06 — out-of-scope register)

- [ ] **RLS GUC must be set per *request transaction* on the Hyperdrive/Worker connection.** Use `SET LOCAL app.current_group_id = '<group>'` inside each request's transaction (resolver maps unset/empty → NULL → deny, fail-*closed*); the trap is pooled-connection reuse (Hyperdrive pools physical connections, so a value set without `SET LOCAL` or never reset could carry a prior tenant into the next request) — use `SET LOCAL` within the request txn (resets at COMMIT/ROLLBACK) or explicitly reset on checkout. Highest-leverage carry-forward. (reviewer M2 / sec-audit R1)
  - **WHEN:** **T07** (member-auth endpoints + DO connection layer).

- [ ] **The runtime DB role must be non-superuser and non-`BYPASSRLS`.** `FORCE ROW LEVEL SECURITY` covers the table owner, but a superuser / `BYPASSRLS` role bypasses RLS regardless; the Hyperdrive/Worker role must be a plain role. (sec-audit R3)
  - **WHEN:** **T07** / infra (DB role provisioning).

- [~] **Atomic supersede-then-insert for the four partial-unique indexes.** Partial unique indexes enforce "at most one live row" on `onboarding_codes`/`recovery_codes`, `sessions`, `admin_invitations`; a regenerate/rotate that inserts the new row before superseding the prior in the same transaction hits a unique violation — must be ordered supersede-then-insert atomically.
  - **Onboarding-code regenerate (issuance):** the *consume*-on-bind is atomic (T07-core); the *regenerate*-invalidates-prior write is issuance-side. **WHEN:** **spec 008** (admin issuance).

- [ ] **`audit_log` table + admin-PII-read audit (I5).** This slice provides only `created_by` (write-side actor); the `audit_log` table and the `#[require_audit]` read-path obligation must exist before any endpoint returns `phone_encrypted` to an Admin. (sec-audit R9)
  - **WHEN:** **T07** / **spec 008** (admin member-management).

- [ ] **PostGIS / `pgcrypto` extensions + `address_encrypted` + per-Group key/KEK columns (I1).** Onboarding tables have no geometry/address; address persistence, the per-Group encryption key, and the KEK (Secrets Store) land with issuance.
  - **WHEN:** **spec 008** (admin issuance) — adds the `i1_addresses_encrypted` enforcement.

- [~] **Actual row writes** (group/member issuance, sign-in lookup, device-bind, refresh rotation, recovery re-bind, admin invite mint/consume) — the schema defines the columns; the writes are the endpoint slices.
  - **WHEN (remaining):** **spec 008** (group/member *issuance* + phone writes) / **T09** (invite consume).

---

## Server / core (spec 001 T07 — out-of-scope register)

- [~] **T07-shell-B — the deployable workers-rs Worker. SLICE 1 (toolchain bring-up + Worker skeleton) DONE 2026-06-07; the Postgres/deploy legs remain (slices 2+).** Remaining PG/deploy legs (need a local Postgres + a Cloudflare account, or spec-008):
  - **Replace the scaffold store with `PgAuthStore`-over-Hyperdrive.** Transport DONE; remaining = the deploy legs only: the real Hyperdrive `id` (`wrangler hyperdrive create` against the Neon URL) + `wrangler secret put HMAC_KEY` + the real `GROUP_ID` + `wrangler deploy` (human gate; Cloudflare MCP is read-only). Operator work: run `provision-neon.sh` once, then the runbook's `wrangler` commands.
    - **Backstop (i):** `provision-neon.sh`'s `GRANT … ON ALL TABLES` is point-in-time — when migration 0009+ lands, a re-run on an already-migrated DB skips creating/granting the new table. When 0009+ lands, add `ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT … TO boundless_app` or bump the expected-table count (twin of the `mig_count` note in `setup-worker-test-db.sh`).
    - **Backstop (ii):** the `smoke-deployed-edge.sh` no-leak grep catches a full connection-string leak but not a bare secret echo; fold the "assert the `db` field never contains a substring of the known HMAC_KEY/connection-string" check into the deployed-edge `/readyz` hardening item when the smoke has those values.
    - **Follow-up (1):** genericize the committed `wrangler.toml` resource ids back to `REPLACE_AT_DEPLOY_*` placeholders before the repo is made public (account ids, not secrets). WHEN: before open-sourcing.
    - **Follow-up (2):** `preview_urls = false` set in `server/wrangler.toml` (config done) — the operator's next `wrangler deploy` actually applies it; until then the live Worker still serves per-version preview URLs.
  - **Access-token verify path (ADR-0021; mint side done):** add the `access_token_hash bytea` column + the per-request verify lookup (re-reads family status), folded into the request's group-scoped RLS txn or served from `GroupHub` DO in-memory state with write-through/evict on revoke. **WHEN:** with the PgAuthStore slice.
  - **Live deployed-edge cross-tenant isolation smoke (sec-audit F5):** as the real `boundless_app` role, assert a request scoped to Group A cannot read Group B's rows on the deployed Worker. Needs ≥2 seeded Groups. **WHEN:** spec 008 (issuance seeds multi-tenant data), or a one-off manual seed before GA.
  - **Route the sign-in below-min alert dedup through the `GroupHub` DO** (reviewer H1). The §10-E once-per-day dedup (`GroupHubState.should_alert`) lives in `AuthService.hub`, re-created per request, so sign-in dedup doesn't persist (R12 alert-flood). The bind path already persists via the DO; sign-in must too once `AuthService` is hosted in / fronted by the DO. **WHEN:** the DO-fronting (bind-device) slice.
  - **`PgDeviceStore` (device-token persistence) + APNs/FCM registration.** Needs spec-008 device-token at-rest encryption (in-memory `DeviceStore` stands in). **WHEN:** push spec 007 / issuance 008.
  - **Turnstile** (code-guess + refresh throttle) + the **per-source refresh-rejection 429** network enforcement (the `GroupHubState` counter exists; the Worker enforces). **WHEN:** with the PgAuthStore slice.
  - **The `boundless::logging::emit()` sink + no-raw-`tracing` lint + Logpush replay** (P2/I10) — also a T16-shell item; route `StoreError` + the invite-token URL segment through it. Carry-forward (ADR-0023 / sec-audit F1): keep the inbound raw `String` phone (`body.phone` before `normalize_phone` tints it) off the log path and never echo it in an error response (keep generic `"bad phone"`/`"internal"`). Add a Worker integration test asserting sign-in/bind/recovery error responses for a malformed/unmatched phone contain no substring of the submitted phone. **WHEN:** this same shell track.
  - **Real signed-manifest KV serving** (ADR-0014; the skeleton only reads a manifest index key). **WHEN:** the manifest-service spec.
  - **Live deployed-edge contract-conformance E2E:** replay the golden `fixtures/auth/*` against the deployed Worker through the real Hyperdrive pooler (the smoke checks `/healthz` + `/readyz` + one sign-in shape + no-leak, not the full fixture matrix). **WHEN:** the deploy-hardening pass.
  - **Rate-limit / Access-gate the `/readyz` DB probe** (reviewer M1 / sec-audit F2). `/readyz` opens a per-call Hyperdrive connection (cheap pooler-amplification vector). Rate-limit it or put it behind Cloudflare Access, and add a deployed-edge assertion that the `db` field never contains a substring of the connection string. **WHEN:** the deploy-hardening pass.
  - **`PgService` drift gate** (reviewer M2). `server/src/runtime/pg.rs::PgService` is a verbatim copy of `server/store/tests/service_pg.rs::PgService`, guarded only by a comment. When `PgDeviceStore` lands (spec 008) the two diverge by design; until then, if a third consumer appears, extract the shared shape into a `boundless-server-store` module both import. **WHEN:** spec 008 / `PgDeviceStore`.
  - **Worker-layer coverage of the `signin_wire` `MemberMatched` branch** (reviewer L4). The matched JSON shape (`next_step`/`manifest_pointer`) is no longer Worker-integration-tested. Restore by seeding one member (a Rust-computed `phone_lookup_hash` matching the test `HMAC_KEY`) when the bind-device slice adds seeding, or add a wasm unit test of `signin_wire`. **WHEN:** the bind-device slice (which seeds the worker test DB).
  - **`wrangler.toml` committed-credential grep gate** (sec-audit F1, optional). Extend the secret/allow-list scan to assert `wrangler.toml` never gains a real Hyperdrive `id` (UUID) or a non-`localhost` connection string. **WHEN:** a CI-hardening pass / the deploy slice.

- [ ] **Multi-device (phone + watch + iPad) concurrent bindings.** T07-core decided: single active device per member — re-onboarding invalidates all prior device bindings (sec-audit F5; AC4). When watch/Wear/iPad pairing is specced, revisit whether a member may hold multiple concurrent bindings and scope invalidation per-platform instead of all-for-member.
  - **WHEN:** the watch/Wear-pairing spec.

---

## Server / store (spec 001 T07-shell slice A — out-of-scope register)

- [ ] **`DeviceStore` Postgres impl** (`current_device_bindings` / `invalidate_device` / `register_device`). Implement the Postgres `PgDeviceStore` (isolated behind the `DeviceStore` port, ADR-0020); `register_device` must write `token_encrypted bytea`, encrypting the DeviceToken (PII) under the per-Group `GroupKey` via the secretbox `encrypt_field` primitive (spec 008 T02). **Guard-rail (sec-audit F3):** assert at the SQL layer that the token column is `bytea`, and add a `static_assertions` check that any persisted device-token wrapper exposes no `Serialize`/`Display` (so the in-memory test double holding a raw `DeviceToken` can never be mistaken for the production shape).
  - **WHEN:** push spec **007** / issuance spec **008** (whichever brings device-token encryption).

- [ ] **Real server-time `now`.** Have the Worker supply real server time to `PgAuthStore` (which takes `now: UnixSeconds`, no `SystemTime::now` in the lib); ties into the still-deferred `chrono`-vs-`time` pick.
  - **WHEN:** **T07-shell-B**.

- [~] **Connection lifecycle + non-superuser role provisioning (sec-audit W2 — highest-impact).** The boot-guard `ensure_least_privilege` and the `provision-neon.sh` scripting/testing are done. **Remaining:** the operator runs `scripts/provision-neon.sh` against real Neon (mints a dedicated non-superuser / non-`BYPASSRLS` / non-table-owner app role — Neon's default `neondb_owner` is `neon_superuser` with `BYPASSRLS`; explicitly `GRANT USAGE` + table/sequence privileges since PG15+ removed implicit `PUBLIC CREATE` on `public`), then add a **live deployed-edge cross-tenant smoke** that connects as the real app role and asserts `ensure_least_privilege` passes AND a cross-tenant read returns zero rows (sec-audit F2; the live analog of `rls_isolates_reads_by_tenant`). Keep `FORCE ROW LEVEL SECURITY` on every PII table (the `least_privilege` test cannot prove "not the table owner").
  - **WHEN:** the operator's deploy (`docs/runbooks/deploy-worker.md`).

- [ ] **Route `StoreError` through the scrubbed log path (sec-audit W4).** The Worker must log `StoreError` only via `boundless::logging::emit()` (P2/I10) — never `{e}`/`{:?}` of a `Db` error raw (its `Display`/`Debug` echoes the SQL + Postgres message, including a unique-violation's conflicting `bytea` keyed-hash value). Add an I10 scrubber-suite fixture with a synthetic unique-violation `DETAIL` carrying a `\x…` hex blob, asserting the emitter drops it.
  - **WHEN:** **T07-shell-B** (logging wiring) + the I10 scrubber suite.

---

## Server / admin-provisioning (spec 001 T08 — out-of-scope register)

- [ ] **The deployable `/api/dev/admins` Worker endpoint + the HTTP-level AC1(a) integration test.** Build the `#[event]`/Router route that classifies the request into a `DevCaller`, calls `authorize_developer`, then `create_admin`; add the AC1(a) integration test (real unauth + admin-auth HTTP requests to `/api/dev/admins` are both rejected — the core test `ac1_admin_creation_rejects_unauth_and_admin` proves only the decision).
  - **WHEN:** **T08-shell** (the deployable Worker; alongside T07-shell-B — needs the same workers-rs/Hyperdrive wiring + `docs-researcher` for workers-rs + Email Workers).

- [ ] **Developer hardware-key WebAuthn verification (constructs the `DeveloperAuthority`).** I11 requires Developer auth be a hardware-key-backed WebAuthn credential; build the dev-WebAuthn registration + assertion verification (likely `@simplewebauthn` on the edge, like admin WebAuthn T09/ADR-0017, but a separate developer credential store) so a caller can legitimately become `DevCaller::Developer`.
  - **WHEN:** **T08-shell** / a dev-auth task (relate to ADR-0017's WebAuthn pattern).

- [ ] **Email Workers delivery + the email-body-no-PII/credential wire assertion (R9 / ADR-0015).** Build the registration URL, send it via Email Workers, and add the test asserting the email body carries only the opaque token (no PII, no credential material — ADR-0015's 6 constraints).
  - **WHEN:** **T08-shell** (Email Workers binding).

- [~] **Invite consume on first WebAuthn registration (single-use, AC16 consume leg) → T09.** Remaining: the real DB consume — the Worker/Postgres `InviteStore` that hashes the presented token with the per-instance HMAC and compares against `admin_invitations.token_hash` (P4: routed through the core's `admin_invitation_token_matches`, NOT in edge-TS, per ADR-0017's WebAuthn carve-out), plus the atomic `consumed_at` stamp (the T06 supersede-then-insert twin).
  - **WHEN:** **T09-shell / T15** (deployable SvelteKit routes + KV/Postgres bindings).

- [ ] **`created_by` = the Developer's identity (write-side audit, I5).** Populate `created_by` (currently NULL/system on the pending-admin member + invitation rows) with the verified Developer id once dev-auth lands, so the audit trail names who minted each Admin.
  - **WHEN:** **T08-shell** (with the dev-WebAuthn verification).

- [ ] **`fresh_admin_invitation` human-facing format (if any).** Token is a 256-bit opaque hex draw (rides in a URL, ADR-0015 requires only opaque + no-PII); add a human-typable form only if the registration UX wants it. No change expected.
  - **WHEN:** revisit only if the registration UX (T15) wants a typable token.

---

## Server / admin-WebAuthn (spec 001 T09 — out-of-scope register)

- [ ] **Additive backup-key enrollment (the second half of AC20 / ADR-0016 D4 "register a backup key").** Authenticated add-credential flow (admin already signed in, no invite) to enroll an additional key without revoking the first; needs the post-assertion session (§10-F). `CredentialStore` already supports >1 active cred; only the entry point + UI are missing. Add the second Playwright test **`ac20_register_passkey_and_backup_key`** (plan §7, line ~158).
  - **WHEN:** **T15** (admin onboarding/settings UI — needs the authenticated admin session).

- [ ] **The deployable SvelteKit `+server.ts` routes.** `/api/admin/auth/{invite,register,signin}` wiring the verification functions to HTTP, setting the post-assertion session, and mapping `WebAuthnError.code` → catalog copy + `routesTo`. Needs the scaffolded SvelteKit app.
  - **WHEN:** **T15** (admin onboarding UI) / **T09-shell**.

- [ ] **Real Postgres `InviteStore` + `CredentialStore` via the Worker.** Reads/writes of `admin_invitations` (load + atomic `consumed_at` stamp) and `admin_webauthn_credentials` (list active / insert / revoke-all-for-admin / bump sign_count) through the deployable Worker. Includes the invite-token HMAC compare routed through the core (`admin_invitation_token_matches`, ADR-0017 P4 carve-out — server-side, not edge-TS). `public_key`/`credential_id` are `bytea`.
  - **WHEN:** **T15 / T09-shell** (Hyperdrive/Postgres binding) — pairs with the T07-shell-B Worker runtime.

- [ ] **Post-assertion session establishment (plan §10-F).** The httpOnly + Secure + SameSite=Strict server-side session cookie minted after a successful WebAuthn assertion (admin session; separate + shorter-lived than member sessions, ADR-0016).
  - **WHEN:** **T15 / T09-shell**.

- [ ] **AC11b — admin-web a11y (axe-core + keyboard ceremony).** Zero axe violations on each admin onboarding route, keyboard-operable WebAuthn ceremony, `aria-live` on invite-expired/error, 200%/400% reflow, RTL/dark. Playwright+axe lives with the UI.
  - **WHEN:** **T15** (admin onboarding UI).

- [ ] **Invite token rides in the URL path — harden the shell against log/referrer leakage** (sec-audit F1). `GET /api/admin/auth/invite/{token}` carries the single-use token in the URL path. The shell must: (a) never emit the `{token}` segment to the structured log path — route through `boundless::logging::emit()` and add an **I10 scrubber fixture** for a URL-embedded opaque token (assert redacted); (b) set **`Referrer-Policy: no-referrer`** on the registration page; (c) keep the single-use consume atomic (already the T09 design).
  - **WHEN:** **T15 / T09-shell** (the deployable invite route) + the I10 scrubber suite.

- [ ] **Live deployed-edge E2E + the `webauthn-rs`-sidecar fallback (ADR-0017).** A smoke test against the deployed SvelteKit Worker (Miniflare/workerd), and — only if `@simplewebauthn`'s Workers status breaks — the documented fallback to a native `webauthn-rs` sidecar.
  - **WHEN:** the deploy/CI-hardening pass (with T07-shell-B / T15) — or if the edge runtime breaks.

---

## API contracts / codegen (spec 001 T10 — out-of-scope register)

- [ ] **Real per-target codegen + the `generate-bindings.sh` "real generators" block.** Wire each generator into `scripts/generate-bindings.sh` (replace the scaffold-mode hash-only step), commit the produced `api/generated/<lang>/` trees + the refreshed drift lock:
      - **Swift:** the **OpenAPI Swift HTTP client** (`swift-openapi-generator` + `protoc-gen-swift` → `api/generated/swift/`) — the network layer the Rider UI drives. **WHEN: with T11** (needs the SwiftPM build-tool plugin; pre-pinned swift-openapi-generator 1.12.2 / runtime 1.6.0 / urlsession 1.4.0 — confirm at use).
      - **Kotlin:** `openapi-generator` (kotlin) + `protoc-gen-kotlin` → `api/generated/kotlin/` + the UniFFI **`core-bridge` AAR**. **WHEN: with/before T13–T14** (needs Android Studio/Gradle + `openapi-generator` + `uniffi-bindgen`).
      - **TypeScript:** `openapi-typescript` + `ts-proto` → `api/generated/typescript/` + `web/src/lib/api/generated/` (`ts-proto` needs `protoc`/`buf`, so the full TS set lands together). **WHEN: with T15**.
      - Each landing needs `docs-researcher` to pin generator versions (lock = ground truth) + fill `docs/stack-matrix.md`. Until then `api/generated/**` stays `.gitkeep` placeholders and the drift gate runs in scaffold mode (hash-only).

- [ ] **Carry T02's platform-parity UniFFI mapping notes into the codegen.** When the UniFFI XCFramework/AAR are generated: `AppVersion` record-vs-string mapping, `MemberId` UniFFI custom-type mapping, and the tainted-type formatter-free binding surface (no `Debug`/`Display` across the FFI — P2/I3). **WHEN:** the Swift/Kotlin codegen (T11–T14) — whenever `AppVersion`/`MemberId`/a tainted type is first exported.

- [ ] **Strict fixture↔OpenAPI conformance test (host-only hardening).** Validate each `fixtures/auth/*.json` against its corresponding frozen OpenAPI schema (de-`$ref`'d `oneOf` member) so any field-name/shape drift between a golden fixture and the contract fails CI — e.g. a small Vitest test using a JSON-Schema validator (`ajv`). **WHEN:** a contract-hardening pass (could ride with **T15**).

- [ ] **Live deployed-edge contract-conformance E2E.** Replay the golden fixtures against the deployed Worker to prove runtime responses actually conform to the frozen OpenAPI (T10 tests only check the contract document). **WHEN:** with the Worker runtime (**T07-shell-B**) / the deploy-hardening pass.

---

## Apple / Rider UI (spec 001 T11 — out-of-scope register)

- [ ] **The deployable iOS app shell (`.xcodeproj` app bundle).** Build the shippable iOS `.app` target (App lifecycle, `Info.plist`, entitlements, bundle id `app.boundless.rider`, launch screen) — the composition root that instantiates `OnboardingViewModel` with the real conformers and hosts `OnboardingRouter`.
  - **WHEN:** when preparing the first iOS build (ties to the Apple licensing/entitlement items above).

- [ ] **The OpenAPI Swift HTTP client (the real `OnboardingNetworking`).** Generate via `swift-openapi-generator` (generator 1.12.2 / runtime 1.6.0 / urlsession 1.4.0 — confirm at use) the `/api/auth/{signin,bind-device}` client feeding real `SignInResult`/`BindResult` into the view model; drops in behind the existing `OnboardingNetworking` protocol/stub.
  - **WHEN:** **T07-shell-B** lands (a live Worker to integration-test against) / first iOS build.

- [ ] **Keychain refresh-token storage (plan §10-F) + APNs registration + signed-manifest fetch/verify.** Build the real conformers: `ManifestProviding` (KV manifest fetch + libsodium verify + cache, ADR-0014, providing `{adminName}`), `NotificationPermissionRequesting` (`UNUserNotificationCenter`; Critical Alerts once the entitlement lands), APNs device-token registration, and the **Keychain** refresh-credential store (never `UserDefaults`/`@AppStorage`).
  - **WHEN:** the iOS app shell / push spec **007** / **T07-shell-B**.

- [ ] **Recorded VoiceOver walkthrough + Accessibility Inspector pass (manual).** Manual recorded VoiceOver/Switch-Control walkthrough + Xcode Accessibility Inspector run (swift-snapshot-testing has no a11y-tree strategy). Optional automation: add CashApp **AccessibilitySnapshot** (a new dep — weigh against the model-level assertion).
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** The 68 baselines (recorded locally, `iPhone13` config, `perceptualPrecision 0.98`) may need a one-time CI re-record if the `macos-15` runner renders fonts/AA outside tolerance; the `boundlessrider` job is GitHub-only / not locally verifiable.
  - **WHEN:** first CI run of the `boundlessrider` job (re-record from the runner if it diverges).

- [ ] **Added copy beyond the spec's 14 screen-copy keys — product-owner review.** Confirm/adjust the 11 added catalog keys (catalog total 25): (a) 7 affordance/settings — `onboarding.action.{continue,try_again}`, `onboarding.permissions.{allow,decline}`, `settings.{title,notifications,help}`; (b) 4 name-less fallback — `onboarding.signin.phone_not_on_file_generic`, `onboarding.binding.{code_prompt,code_invalid}_generic`, `onboarding.permissions.notifications_declined_generic`.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [~] **`auth.signin_again` (Driver re-auth) + the two `admin.onboarding.*` keys** — the two `admin.onboarding.*` keys still have no L10n accessor by design (rendered by the SvelteKit admin UI).
  - **WHEN (remaining):** **T15** (admin web).

---

## Apple / Driver UI (spec 001 T12 — out-of-scope register)

- [ ] **The deployable `.xcodeproj` Driver app bundle.** The shippable iOS `.app` target (App lifecycle, `Info.plist`, entitlements, bundle id **`app.boundless.driver`** — see the Apple "Register the Driver app Bundle ID" item) that instantiates `DriverOnboardingViewModel` with real conformers and hosts `DriverOnboardingRouter`.
  - **WHEN:** when preparing the first Driver iOS build (ties to the Apple licensing/entitlement items).

- [ ] **The real `RecoveryCodeProviding` + the OpenAPI Swift client (incl. `/api/auth/recovery/rebind`).** Real impl reads `fresh_recovery_code` off the `/api/auth/bind-device` (and rebind) response; drops in behind the protocol.
  - **WHEN:** **T07-shell-B** lands (a live Worker) / first Driver iOS build.

- [ ] **The self-serve re-bind ENTRY UI** (phone + Recovery Code on a *new* device → re-bind, old token invalidated, fresh code issued). Needs either a new core recovery-rebind state or a separate flow (P4 — no core state to render today).
  - **WHEN:** a recovery-rebind flow spec (or when the Driver app shell adds a "new phone" entry point).

- [ ] **Keychain refresh-token storage (§10-F) + APNs registration + signed-manifest fetch/verify.** Real conformers for the injected `ManifestProviding` / `NotificationPermissionRequesting` boundaries (KV manifest fetch + libsodium verify + cache; `UNUserNotificationCenter`; the Keychain refresh store). Same as the T11-shell items.
  - **WHEN:** the Driver iOS app shell / push spec **007** / **T07-shell-B**.

- [ ] **Recorded VoiceOver walkthrough + Recovery-Code spell-out a11y (manual / polish).** Recorded VoiceOver/Switch-Control walkthrough (manual); optional polish: read the Recovery Code character-by-character via a per-character `accessibilityLabel`.
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** The 76 Driver baselines (`iPhone13`, `perceptualPrecision 0.98`) may need a one-time CI re-record if the `macos-15` runner's simulator diverges. The `boundlessdriver` job is GitHub-only / not locally verifiable.
  - **WHEN:** first CI run of the `boundlessdriver` job (re-record from the runner if it diverges).

- [ ] **4 added catalog keys beyond the spec's table — product-owner review.** Confirm the 4 Driver `DriverOnboarding.xcstrings` keys: `onboarding.driver.intro` + the Recovery Code capture trio `onboarding.recovery.{title,explanation,saved}`.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [ ] **`DriverShared` reuses `RiderShared` directly (no extracted "OnboardingKit" module).** If a future consumer needs the kit without the "Rider" name, consider extracting a neutral `BoundlessOnboardingKit` SwiftPM module. Not needed now (YAGNI).
  - **WHEN:** if/when a third Apple consumer of the onboarding kit appears.

---

## Android bring-up (spec 001 — DONE 2026-06-06; unblocks T13/T14)

- [ ] **Committed `gradle.lockfile`(s) + fold the Android tree into `check-network-allowlist.sh` (I8/AC13).**
      Commit the `gradle.lockfile` and have `check-network-allowlist.sh` scan it like the other 5 locks (it already globs `gradle.lockfile`). The Android tree is gated in the interim by `scripts/check-android-trackers.sh` (in the `android` job, greps closures against `ci/forbidden-trackers.txt`). Gradle dependency locking across the AGP multi-module build has footguns (`lockAllConfigurations` breaks builds if generation missed a config; use LENIENT mode + a `resolveAndLockAll` task), so the lockfile needs its own focused slice.
  - **WHEN:** with **T13/T14** (when the Android dep tree stabilizes with the real Compose/Hilt/Turbine set) or the next CI-hardening pass.

- [ ] **`sdkmanager`/cmdline-tools writes SDK XML v4; AGP 8.4.2's tooling understands up to v3** — a benign build warning (cmdline-tools `latest` 20.0 is newer than AGP 8.4.2). If it ever becomes more than cosmetic, pin an older cmdline-tools or bump AGP (gated on Paparazzi).
  - **WHEN:** only if it stops being a pure warning (or when AGP is next bumped).

- [ ] **Snapshot-baseline CI-runtime pin.** The Paparazzi sample baseline was recorded locally; it is text-free so should be byte-stable, but the `android` job is GitHub-only / not locally verifiable — first CI run is the real proof; re-record from the runner if it diverges. T13/T14's real screens (with text) face the usual cross-runtime tolerance question.
  - **WHEN:** first CI run of the `android` job; and at T13/T14 for the real screens.

- [ ] **Kotlin OpenAPI/proto codegen → `api/generated/kotlin/`** (`openapi-generator` kotlin + `protoc-gen-kotlin`). Not part of the bring-up (which wired only the UniFFI Kotlin). Re-run the network allow-list against the new `gradle.lockfile`(s) when it lands.
  - **WHEN:** with **T13/T14** (the Compose UIs that consume the generated network client).

- [ ] **Carry the T02 platform-parity UniFFI mapping notes into the Kotlin codegen** (`AppVersion` record/string, `MemberId` custom-type, tainted-type formatter-free surface). The `core/ffi-kotlin` surface deliberately excludes all of these (only the state-machine enums + `Role` + `bool` cross, ADR-0022 scope); actionable when a Kotlin UI task first needs one of those types across the FFI.
  - **WHEN:** **T13/T14**, whenever `AppVersion`/`MemberId`/a tainted type is first exported to Kotlin.

---

## Android / Rider UI (spec 001 T13 — out-of-scope register)

- [ ] **The deployable launcher `MainActivity` (the composition root).** Ship the `MainActivity` that instantiates `OnboardingViewModel` with real conformers, hosts `OnboardingRouter`, and wraps it in `RiderTheme`.
  - **WHEN:** when preparing the first Android build (ties to the FCM/Play-auto-update items below).

- [ ] **The production `AndroidRiderStrings` (R.string resolver) + its wiring.** Build the production impl over Android `Resources` (`getString(R.string.x, *args)`), constructed by MainActivity; add an instrumented/Robolectric smoke test exercising the real resource path (today its `key→R.string` map is compile-checked only).
  - **WHEN:** **T13-shell** (with MainActivity).

- [ ] **The real `OnboardingNetworking` (OpenAPI Kotlin HTTP client).** Build the real impl (`openapi-generator` kotlin → `/api/auth/{signin,bind-device}`) feeding real `SignInResult`/`BindResult` into the view model; re-run the network allow-list against the committed `gradle.lockfile` when the Kotlin codegen (`api/generated/kotlin/`) lands.
  - **WHEN:** **T07-shell-B** (a live Worker) + the T10 Kotlin codegen.

- [ ] **`NotificationManager` permission flow + FCM registration + signed-manifest fetch/verify + Keystore refresh storage (§10-F).** Build the real `NotificationPermissionRequesting` (`POST_NOTIFICATIONS` on API 33+), FCM device-token registration, the `ManifestProviding` impl (KV manifest fetch + libsodium verify + cache, ADR-0014, providing `{adminName}`), and the EncryptedSharedPreferences/Keystore refresh-credential store (never plain `SharedPreferences`).
  - **WHEN:** the Android app shell / push spec **007** / **T07-shell-B**.

- [ ] **Onboarding-Code / phone field input-security (security-auditor M1).** When real input is wired, mark the code field as a one-time code (`Modifier.semantics { contentType = ContentType.SmsOtpCode }`, a Compose 1.8 API) + no-personalized-learning/autocorrect-off, and the phone field `ContentType.PhoneNumber`; iOS twin uses `.textContentType(.oneTimeCode/.telephoneNumber)`. Add a Compose semantics test asserting the content types.
  - **WHEN:** **T13-shell** (MainActivity real input) / a Compose 1.8 bump.

- [ ] **Submit re-entrancy guard (reviewer LOW).** Disable the action (or guard on a "submitting" flag) while a `viewModel.submitPhone/submitCode/decideNotifications` coroutine is in-flight, to prevent double-submit on slow real networks.
  - **WHEN:** **T13-shell** (real `OnboardingNetworking`).

- [ ] **Optional Confirmation/Banner icon parity (platform-parity M1).** If visual parity is wanted, add the iOS twin's `checkmark.circle` / `info.circle` icons to the Android `Confirmation`/`Banner` (weigh the `material-icons-extended` dep, or ship a tiny local vector); a11y bar already met via shape+text.
  - **WHEN:** **T13-shell** (only if visual icon parity is desired).

- [ ] **Recorded TalkBack walkthrough + Accessibility Scanner pass (manual).** Manual recorded TalkBack/Switch-Access walkthrough + Accessibility Scanner run (plan §7); optional automation via a Robolectric + `compose-ui-test` `createComposeRule` reading-order test.
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** First CI run is the real cross-runtime proof of the 68 Paparazzi baselines (the `android` job is GitHub-only / not locally verifiable); re-record from the runner if text rendering diverges.
  - **WHEN:** first CI run of the extended `android` job (`:rider:app:verifyPaparazziDebug` with real screens).

- [ ] **11 added catalog keys beyond the spec's 14 — product-owner review.** Confirm the 11 affordance/settings + `*_generic` fallback keys (the `strings.xml` mirrors the iOS `RiderShared` catalog's 25 keys); adjust copy if the owner prefers different wording, keeping lock-step with the iOS catalog.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

---

## Android / Driver UI (spec 001 T14 — out-of-scope register)

- [ ] **The deployable launcher `MainActivity` (the Driver composition root).** Ship the `MainActivity` that instantiates `DriverOnboardingViewModel` with the real conformers, hosts `DriverOnboardingRouter`, and wraps it in `DriverTheme` — bundle id `app.boundless.driver`.
  - **WHEN:** when preparing the first Driver Android build.

- [ ] **The real `RecoveryCodeProviding` + the OpenAPI Kotlin HTTP client (`OnboardingNetworking`).** Real `RecoveryCodeProviding` reading `fresh_recovery_code` off the `/api/auth/bind-device` (and rebind) response + the real `OnboardingNetworking` (`openapi-generator` kotlin → `/api/auth/{signin,bind-device}`). Carry-forward (security review, low): the Recovery Code is a bare `String?` — when the real provider lands, wrap it in a thin `RecoveryCode` newtype with no `toString`/`Debug` returning the raw value (P2) + add a CI/lint grep asserting no `Log.*`/`print` of the value, coordinated with the iOS twin.
  - **WHEN:** **T07-shell-B** (a live Worker) + the T10 Kotlin OpenAPI codegen.

- [ ] **The self-serve re-bind ENTRY UI** (phone + Recovery Code on a *new* device → re-bind, old token invalidated, fresh code issued). The onboarding state machine has no recovery-rebind state to render; needs either a new core state or a separate flow.
  - **WHEN:** a recovery-rebind flow spec (or when the Driver app shell adds a "new phone" entry point).

- [ ] **`NotificationManager`/FCM + signed-manifest fetch/verify + Keystore refresh storage (§10-F).** Real `NotificationPermissionRequesting` (`POST_NOTIFICATIONS` on API 33+), FCM device-token registration, the `ManifestProviding` impl (KV manifest fetch + libsodium verify + cache, ADR-0014, providing `{adminName}`), and the EncryptedSharedPreferences / Keystore refresh-credential store. Carry-forward (security review, nit): before persisting ANY credential, set `android:allowBackup="false"` (or a tight `dataExtractionRules`/`fullBackupContent` excluding the Keystore-backed store) on both the Driver and Rider app manifests (Android Auto Backup else exfiltrates to Google — P2/I12).
  - **WHEN:** the Driver app shell / push spec **007** / **T07-shell-B**.

- [ ] **Onboarding-Code / phone field input-security + submit re-entrancy guard (same as T13-shell).** Mark the code field as a one-time code (`ContentType.SmsOtpCode`, a Compose 1.8 API) + no-personalized-learning, and the phone field `ContentType.PhoneNumber`; disable the action while a `submitPhone`/`submitCode` coroutine is in-flight (double-tap guard). Carry-forward (security review, nit): while the Recovery-Code capture screen is shown, set `FLAG_SECURE` on the window and keep the code out of autofill/clipboard-history learning.
  - **WHEN:** **T14-shell** (MainActivity real input) / a Compose 1.8 bump.

- [ ] **Recorded TalkBack walkthrough + character-by-character Recovery-Code spell-out (manual / polish).** Recorded TalkBack / Switch-Access walkthrough (manual); optional polish of reading the Recovery Code character-by-character via a per-character semantics label.
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **Snapshot-baseline CI-runtime pin.** First CI run is the real cross-runtime proof for the 76 Driver baselines; re-record from the runner if text rendering diverges. Extend the CI `android` job to run `:driver:app:verifyPaparazziDebug` + `:driver:app:testDebugUnitTest` + `:rider:shared:assembleDebug` alongside the rider ones.
  - **WHEN:** first CI run of the extended `android` job; extend the job's task list with the Driver + `:rider:shared`.

- [ ] **4 added Driver catalog keys beyond the spec's table — product-owner review.** Driver `strings.xml` (4 keys) mirroring `DriverOnboarding.xcstrings`: `onboarding_driver_intro` and `onboarding_recovery_{title,explanation,saved}`. Keep in lock-step with the iOS Driver catalog.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [ ] **Move the shared test resolver/fakes to AGP `testFixtures` once KGP ≥ 2.1.** KGP 2.0.21 doesn't compile Kotlin in `testFixtures` (landed in Kotlin 2.1.0); when the toolchain advances (Paparazzi 2.x stable → Kotlin 2.1+), move the catalog resolver + fakes into `:rider:shared/src/testFixtures` and consume via `testImplementation(testFixtures(project(":rider:shared")))` to drop the resolver out of `main` and de-duplicate the fakes.
  - **WHEN:** the next Android toolchain bump (Paparazzi 2.x / Kotlin 2.1+).

---

## Admin web / SvelteKit onboarding (spec 001 T15 — out-of-scope register)

- [ ] **(B) Genuinely deploy/account-blocked — rides with T07-shell-B.** Build the **Postgres** `InviteStore`/`CredentialStore` over **Hyperdrive** (incl. the invite-token HMAC compare routed through the core's `admin_invitation_token_matches` per ADR-0017's P4 carve-out), the actual **`wrangler deploy`** (needs a Cloudflare account), and the live deployed-edge E2E. **Also fold into this deploy slice:**
      - (i)(F1, I10 — web logging backstop) Route uncaught endpoint throws (e.g. the `selectChallengeStore` prod-no-KV throw) through the web scrubbed `emit()` sink + a `handleError` hook + the no-raw-`console` lint, and add an I10 scrubber fixture for that operator string (assert zero redactions). **WHEN:** the web logging slice.
      - (i)(F2, optional hardening) Add a build-artifact test asserting `dev` is inlined to `false` at the `selectChallengeStore` call site (the automated form of the manual built-chunk grep). **WHEN:** the deploy slice.
      - (ii) **Generate `App.Platform` via `wrangler types`** instead of the hand-typed `app.d.ts` once the Hyperdrive binding lands, so a `wrangler.toml` binding rename is type-caught.
      - (iii) **Set `send_metrics = false` in `server/wrangler.toml`** too (web/wrangler.toml already has it).
  - **WHEN:** **T07-shell-B** (the deployable Worker + Hyperdrive) / **T15-shell** deploy.
- [ ] **Persistent server-side session store behind the §10-F cookie.** Persist the admin-session data (currently an in-memory map in `src/lib/server/session.ts`) to KV/Postgres + add expiry/rotation (separate + shorter-lived than member sessions, ADR-0016).
  - **WHEN:** **T15-shell**.
- [ ] **Dev-only `/api/test/{seed-invite,reset}` seams** — remove these `dev`-gated Playwright seams when the real KV/Postgres backend + a proper test-fixture path land.
  - **WHEN:** **T15-shell** (with the real backend).
- [ ] **Authenticated additive backup-key enrollment + `ac20_register_passkey_and_backup_key`.** Build the authenticated add-credential flow (signed-in admin session, no invite, no revoke) + its Playwright test; `CredentialStore` already supports >1 active cred.
  - **WHEN:** **T15-shell** / admin settings UI.
- [ ] **Invite token in the URL path — log/`Referer` hardening at the deployable route** (T09-register sec-audit F1): never emit the `{token}` segment to the structured log path (route through `boundless::logging::emit()`; add an I10 scrubber fixture for a URL-embedded opaque token) and set `Referrer-Policy: no-referrer` on the registration page.
  - **WHEN:** **T15-shell** (deployable invite route) + the I10 scrubber suite.
- [ ] **AC11b live screen-reader pass + Lighthouse (manual/advisory).** Manual NVDA/VoiceOver walkthrough + Lighthouse ≥95 (advisory).
  - **WHEN:** the persona-acceptance / a11y review pass before GA.
- [ ] **13 catalog keys (2 spec + 11 added) — product-owner review.** Confirm the copy in `src/lib/i18n/catalog.ts`: the spec's `admin.onboarding.register_credential`/`admin.onboarding.invite_expired` plus the 11 added `admin.onboarding.{register_explainer,register_action,registering,registered,go_to_signin}`, `admin.signin.{title,explainer,action,signing_in,failed}`, `admin.home.signed_in`.
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.
- [ ] **Real translations + `melt-ui`.** Only `en` ships; `gsw`/RTL (`ar`/`he`)/`zz-ZZ` arrive via Weblate + signed KV manifest (ADR-0014); `melt-ui`/Radix primitives deferred to spec 008.
  - **WHEN:** **T16** (pseudo-locale) / translation pipeline / **spec 008** (primitives).
- [ ] **Snapshot/visual baselines + CI runtime.** The extended `web` CI job (`pnpm build` + T15 e2e) is GitHub-only / not locally gated.
  - **WHEN:** first CI run of the extended `web` job.

---

## Cross-cutting verification (spec 001 T16 — out-of-scope register)

- [ ] **`boundless::logging::emit()` sink + no-raw-`tracing` lint + Logpush/latest-run CI replay.** Build the deployable sink routing every Worker log line through `detect_pii` before persistence, the lint forbidding direct `tracing::*` (I10), and the CI step replaying the latest real run's logs. **Carry-forward (T07/T09/T15):** route `StoreError` and the invite-token URL segment through `emit()`; add I10 scrubber fixtures for a unique-violation `\x…` DETAIL blob + a URL-embedded opaque token.
  - **WHEN:** **T07-shell-B** (the deployable Worker + logging wiring).

- [ ] **Native (iOS / Android) `zz-ZZ` pixel-truncation snapshot variant.** Add a `zz-ZZ` variant to each native ×4 a11y snapshot matrix (T11–T14) via swift-snapshot / Paparazzi.
  - **WHEN:** each platform's **-shell** (the app bundle / MainActivity task), or a pre-GA a11y pass.

- [ ] **Scrubber hardening — residual detector gaps** (latent until the live `emit()` sink ships at T07-shell-B). Remaining detector gaps:
  - **Sub-40-char token/secret blobs.** `TOKEN_BLOB_MIN = 40` misses a 16-byte token (~22–32 chars) or a dashed grouped code; needs an entropy / mixed-charset heuristic (flag a ≥~22-char run with mixed case+digits; leave all-caps codes and all-lower event names alone), not just a smaller threshold.
  - **Lowercase-only street addresses** (`47 willow lane`) — `find_street_addresses` requires a capitalized word; relax case when issuance/logging lands (spec 008).
  - **Bare-dash digit-glue run-merge** — a phone glued onto a UUID via a bare `-` (`5551234567-0000-…`) merges into one >15-digit run and escapes the phone window; fix when phone detection is next reworked (re-scan sub-windows on an over-long run).
  - **General:** Unicode-confusable / homoglyph phone & email; name+DOB heuristics; threshold tuning against real Logpush samples; a `scrub()` that redacts (not just detects) for the `emit()` path.
  - **WHEN:** a privacy-hardening pass with the **T07-shell-B** `emit()` sink, extending `scrub_redteam.rs` with each closed gap's positive case.

- [ ] **Real `gsw` / RTL / `zz-ZZ` translations via Weblate + signed-KV (ADR-0014).** Ship real locales (Swiss German, Arabic/Hebrew RTL) via the translation pipeline + signed KV manifest, replacing the generated pseudo `zz-ZZ`.
  - **WHEN:** the translation pipeline / manifest service spec.

---

## Admin member-management (spec 008 — deferred shells, recorded at T01)

- [ ] **Live `boundless::logging::emit()` sink + the member-issuance I10 scrubber fixture.** The hottest PII write path (name + address + phone + per-Group key); the deployable scrubbed `emit()` sink + no-raw-`tracing` lint are the shared T07-shell-B track — spec 008 adds a member-issuance red-team fixture (a synthetic issuance log line carrying name/address/phone → assert zero PII survives the scrubber) once that sink exists.
  - **WHEN:** **spec 001 T07-shell-B** (the live `emit()` sink) + spec 008 T09 (Worker issuance logging).

- [ ] **KEK re-wrap rotation tooling + the Group-key re-encrypt Workflow (ADR-0025).** The runbook-documented procedures (`docs/runbooks/key-management.md`) are unbuilt: KEK rotation re-wraps `delegated_keys.wrapped_key` + bumps `kek_version`; Group-key rotation is the expensive re-encrypt-every-PII-row maintenance Workflow. No rotation trigger ships in spec 008.
  - **WHEN:** a key-rotation maintenance spec (or on suspected compromise — run the runbook procedure).

- [ ] **The I12 `forget_member` sweep must cover the new PII surfaces.** When `core::deletion` is built, `forget_member` must redact/remove `members.name_encrypted` + `members.address_encrypted` and reconcile the `audit_log` retention tension: `audit_log` carries `FOREIGN KEY (member_id, group_id) REFERENCES members ON DELETE CASCADE` (provisional, noted in the `0011` header), so deleting a member would delete their audit rows — but I12 keeps audit logs (PII redacted). Change this (e.g. `member_id` nullable + `ON DELETE SET NULL` + an `Anonymous_NNNN` ref, or sever the FK), and extend the I12 forgetting property test to assert the forgotten member's audit rows still exist but carry no resolvable PII pointer.
  - **WHEN:** the `core::deletion` spec (extends the I12 forgetting property test to the spec-008 columns + the audit_log retention reconciliation).

- [ ] **`PgDeviceStore` device-token at-rest encryption is now *unblocked*** by spec 008 T02's secretbox primitive — the push spec can encrypt the device token under the per-Group `GroupKey` (`encrypt_field`).
  - **WHEN:** push spec **007**.

- [ ] **Geocoding the address → coordinates / the ETA-matrix Workflow** — the geocode trigger is owned by the matching spec, not issuance; spec 008 persists only the encrypted address. Other spec-008 out-of-scope items (role-swap workflow, remote-only/"join from home" mode, the O5 device-versions panel, the O7 phone-list export, bulk/CSV import) live in their own later specs.
  - **WHEN:** the matching spec (004+) / the named sibling specs.

- [ ] **Carry-forwards from the T01 review (reviewer S1 / security-auditor F4):** (a) **`admin.member.address_invalid`** was registered in `docs/error-codes.md` beyond the spec's i18n table — confirm the English copy with the owner when T10 authors it. (b) The **`#[require_audit]` compile gate (T06, I5) must cover the duplicate-phone response path** — `ADMIN_MEMBER_DUPLICATE_PHONE` returns an existing member's identity (an audited PII read, not just a not-found code); T06 must not let that path skip the audit obligation, and T08's contract test must assert no `/api/auth/*` shape can ever return `ADMIN_MEMBER_DUPLICATE_PHONE` or a member-identity field (no existence-leak regression).
  - **WHEN:** (a) **T10** (catalog copy); (b) **T06** (`#[require_audit]` gate) + **T08** (contract test).

- [ ] **Carry-forwards from the T02 review (security-auditor + platform-parity):** (a) **Decrypted-PII zeroization (MED).** `core/crypto::decrypt_field` returns a plain `Vec<u8>`; the boundary re-wraps it into tainted `Address`/`MemberName` whose inner `String` is not zeroized (true of every `tainted_secret!` type). Decide whether the tainted types should own a zeroizing buffer (decrypted PII wiped on drop, P3/I2) — a cross-cutting change to the `tainted_secret!` macro. (b) **Production-RNG guard (LOW).** When the issuance path is wired, add an integration assertion that a field-encrypting endpoint constructs `RngSecretSource` (not a placeholder/seq impl) — mirroring the `PlaceholderSecrets::fresh_nonce` → `unreachable!` fail-closed guard. (c) **Wire-projection parity watch (LOW).** Keep `Address`/`MemberName` off the UniFFI/wasm bindings at T05; build the wire `MemberSummary`/`MemberDetail` DTOs with a fixture-vs-`api/openapi.yaml` contract test (the core↔wire seam that bit the T10 `ManifestPointer`).
  - **WHEN:** (a) **T05/T09** (the decrypt boundary); (b) **T09** (deployed field-encrypting endpoint); (c) **T05/T08** (the projection + contract test).

- [ ] **Carry-forward from the T04 review (reviewer L1): preserve the key-load failure *reason* on the operator path.** `core/server::load_group_key` collapses both failure modes — `wrapped == None` (Group never bootstrapped) and an `unwrap_group_key` `Err` (wrong KEK / corrupt or tampered blob) — into one opaque `GroupKeyMissing` → `ADMIN_GROUP_KEY_MISSING`. Keep that collapse on the wire/client response; the deployable Worker that loads the KEK and calls `load_group_key` should log the underlying `SecretboxError` variant (`Malformed`/`Decrypt`, PII/key-free per `secretbox.rs`) via the scrubbed `emit()` path so an operator can distinguish "never provisioned" (run the bootstrap) from "wrong KEK / corrupt blob" (fix the KEK binding).
  - **WHEN:** **T09** (the Worker KEK-load + GroupKey-cache path) — rides the same `emit()` sink the spec-008 issuance I10 fixture needs.

---

## Server / core — `MemberService` (spec 008 T05 — out-of-scope register)

- [ ] **Decrypted-PII zeroization (T02 review carry-forward (a)) — decision recorded, change deferred.** Decide whether `tainted_secret!` types (`MemberName`/`PhoneNumber`/`Address`) get a zeroizing buffer — `decrypt_field` returns a plain `Vec<u8>` re-wrapped into tainted types whose inner `String` isn't zeroized, and `MemberDetail::to_wire` copies again into `MemberDetailView`, scattering 3+ unzeroized plaintext copies (DO-memory-snapshot residual on PII).
  - **WHEN:** a privacy-hardening pass / when the `tainted_secret!` macro gains a zeroizing buffer.
- [ ] **DB-level atomicity + RLS proofs (T07).** Real `PgMemberStore`/`PgAuditStore`/`PgDelegatedKeyStore` over PG18 must prove the port atomicity contracts the in-memory double only models (member+code insert in one txn; audit INSERT atomic with detail SELECT, R5; regenerate supersede-then-insert; optimistic `UPDATE … WHERE updated_at = $expected`; the partial-unique "one live code" index; `(group_id, phone_lookup)` unique → `DuplicatePhone`; RLS isolation). The `list_members` admin-exclusion in-memory `roles.contains(Admin)` filter becomes SQL `WHERE 'admin' != ALL(roles)`.
  - **WHEN:** **T07** (the Postgres adapters).
- [ ] **`edit_member` has no duplicate-phone conflict outcome (T05 review, reviewer+sec LOW).** Issuance has first-class `IssueMemberOutcome::DuplicatePhone`; `EditMemberOutcome` has no analogue, so an edit onto an enrolled number hits the `(group_id, phone_lookup_hash)` partial-unique index as an opaque `StoreError` (500-class) and the in-memory double silently overwrites. Decide the contract: map the unique-violation to a clean `EditMemberOutcome`-conflict (audit any existing-member disclosure as issuance does) OR a calm `ADMIN_MEMBER_DUPLICATE_PHONE` reject with no partial write; add a `pg_member_store_edit_into_duplicate_phone_*` integration test.
  - **WHEN:** **T07** (DB conflict) / **T09** (the user-facing/audited mapping).
- [ ] **Duplicate-phone cross-surface contract (T01 review (b)).** A T08 contract test must assert no `/api/auth/*` shape can return `ADMIN_MEMBER_DUPLICATE_PHONE` or a member-identity field (no-existence-leak; the outcome is admin-surface-only, R9).
  - **WHEN:** **T08** (OpenAPI freeze + contract test).
- [ ] **OpenAPI single-source pins (T08).** Mirror verbatim into OpenAPI: `OnboardingStatus`+`AuditField` `snake_case` casing (pinned by T05 serde tests); `AuditEntry.timestamp` = epoch-seconds integer (core `UnixSeconds(i64)`, no chrono); `member_id` as `{type:string, format:uuid}` (`$serde(transparent)`); `roles` `$ref` to `Role`; and the `IssueMemberResponse` two-arm `oneOf` (Issued-with-code vs duplicate-with-summary) — the existing contract test does NOT descend `oneOf`, so T08 needs an explicit per-arm assertion.
  - **WHEN:** **T08**.
- [ ] **Worker projection + R10 + request_id (T09).** The Worker must (a) serialize `MemberDetailView` via serde (`Response::from_json(&view)`), NOT a hand-rolled `json!` (`server/src/runtime/**` is not in `.bindings.lock`); (b) keep inbound raw `name`/`address`/`phone` off the log path and out of error responses (R10 — value-free codes); (c) mint `request_id` as a server-minted opaque id (never client-echoed). Plus: a field-decrypt failure collapses to `ADMIN_GROUP_KEY_MISSING` on the wire (no oracle), with the underlying `SecretboxError` variant logged via `emit()` to distinguish "no key" from "corrupt field/blob".
  - **WHEN:** **T09** (the deployable Worker + `emit()` sink).
- [ ] **Server-side member search/filter (`?search=&role=&status=`).** `list_members` is param-less in T05 (lists all non-Admin); add the SQL `WHERE` + query params (`MemberStore::list_members` may gain a filter param).
  - **WHEN:** **T07/T10**.
- [ ] **`PATCH` returns the updated detail?** `edit_member` returns only `{Updated, Stale, Rejected}` (not an audited read); decide whether the wire `PATCH` returns the detail (and so audits).
  - **WHEN:** **T08**.
- [ ] **Keep the new admin types off the UniFFI/wasm FFI mirror crates.** `MemberSummary`/`MemberDetail`/`MemberDetailView`/`OnboardingStatus`/`AuditEntry`/`AuditField`/`IssuableRole` are admin-web/TS-only — never add them to `core/ffi-swift`/`core/ffi-kotlin`.
  - **WHEN:** ongoing (a no-op to maintain; note for a future reader).

---

## Server / core — I5 audited-response compile gate (sealed `AuditedResponse` / `PiiDisclosure`) (spec 008 T06 — out-of-scope register)

- [ ] **The residual `expose_secret()` + hand-rolled-`json!` egress is NOT closed by any pure-Rust gate** — a future endpoint that re-`decrypt_field`s and builds its own body bypasses the type gate; close via I5's named second layer: T08's `openapi_pii_handlers_all_require_audit` integration test, a T09 clippy/grep lint forbidding `Response::from_json`/`to_string`/`json!` on member PII in `server/runtime/**`, and the P2/I10 scrubber. Document the gate's scope in the route code so no one mistakes it for airtight.
  - **WHEN:** **T08** (the OpenAPI-coverage second layer) + **T09** (the deployable route lint).

- [ ] **T08 must extend the `AuditedResponse` allowlist for its new admin wire DTOs.** When T08 freezes `MemberList`, `IssueMemberResponse`, the regenerate-code response, etc., each PII-free one must get an `impl AuditedResponse` (with the matching `impl sealed::Sealed`), and any carrying decrypted PII must flow as a `PiiDisclosure<_>`, never a bare `impl AuditedResponse`.
  - **WHEN:** **T08** (OpenAPI freeze + the new wire DTOs).

- [ ] **The router send-seam (`admin_response_body`) is provided but not yet *consumed* (no router exists — that is T09).** T09 must serialize every admin response through it (or an equivalent `AuditedResponse`-bounded constructor) rather than calling `worker::Response::from_json` directly on a member DTO; pair with the T09 lint above.
  - **WHEN:** **T09** (the deployable `/api/admin/members/*` routes).

- [ ] **`.stderr` golden re-bless on a toolchain bump.** A rustc diagnostic-wording change requires `TRYBUILD=overwrite cargo test -p boundless-server-core --test require_audit --test member_summary_compile` to regenerate the committed `.stderr` files.
  - **WHEN:** the next `rust-toolchain.toml` bump (if the compile-fail tests then mismatch).

- [ ] **A literal `#[require_audit]` proc-macro (the plan §7 "stretch goal") was NOT built.** The sealed `AuditedResponse` bound + un-forgeable carrier satisfy I5's intent without a `core/macros` proc-macro crate; remains optional future hardening if a per-handler attribute is ever wanted.
  - **WHEN:** only if a literal attribute-macro form is later desired (not required).

---

## Server / store — `PgMemberStore` (spec 008 T07 — out-of-scope register)

- [ ] **The deployable `/api/admin/members/*` Worker endpoints (T09).** Compose `MemberService` over `PgMemberStore`, load the KEK from Secrets Store, cache the unwrapped `GroupKey` in the `GroupHub` DO, inject the live CSPRNG, and add the miniflare+PG worker tests (`worker_issue_member_round_trip`, …). The `build_service` analog constructs `MemberService<PgMemberStore, RngSecretSource, …>` with the KEK/HMAC `MemberConfig`, mirroring `server/src/runtime/pg.rs::PgService`.
  - **WHEN:** **T09**.

- [ ] **`onboarding_status` TTL nuance.** The derived `STATUS_CASE` (`members.rs`) is TTL-agnostic; refine the "expired (past TTL) vs lost (superseded/consumed-without-bind)" distinction by passing `now` into the `list_members` / `read_member_detail_audited` ports.
  - **WHEN:** **T10** (status UI) / a status spec.

- [ ] **`edit_member` into a duplicate phone is an opaque `StoreError`, not a calm outcome (T05 carry-forward).** Moving a member's phone onto a number already enrolled hits the `members_group_phone_lookup_key` unique index → `StoreError::Db` (500-class), not the calm `ADMIN_MEMBER_DUPLICATE_PHONE`. Catch the unique violation → an `EditMemberOutcome` conflict arm (audited like issuance, or a documented calm reject) + a `pg_member_store_edit_into_duplicate_phone` regression test.
  - **WHEN:** **T09** (the user-facing/audited edit-conflict mapping).

---

## API contracts / admin surface (spec 008 T08 — out-of-scope register)

- [ ] **The Rust wire response DTOs + the `audited.rs` `AuditedResponse` allowlist extension.** The admin wire response shapes (`IssueMemberResponseWire`/`MemberIssued`, `RegenerateCodeResponseWire`, the `DuplicatePhoneLink` body) live in `core/server` (two-type split, built via `expose_secret`) and must be blessed on the sealed `AuditedResponse` allowlist so `admin_response_body` serializes them; pin their serde keys (mirror `member_detail_view_wire_keys_are_pinned`).
  - **WHEN:** **T09** (the Worker projection + miniflare tests).

- [ ] **Live deployed-edge contract-conformance for the admin surface.** Replay real admin responses against the deployed Worker to prove runtime conformance (T08 tests only check the contract document + registry parity).
  - **WHEN:** the deploy-hardening pass / **T11**.

---

## Server / Worker — admin member endpoints (spec 008 T09 — out-of-scope register)

- [~] **The real SvelteKit→Worker BFF call (ADR-0026).** Remaining: the live deployed round-trip (the SvelteKit Worker actually calling the deployed Rust Worker over the network) — needs the deploy + a Cloudflare account; the UI e2e drives the in-memory fake until then.
  - **WHEN:** the deploy-hardening pass / **T11**.

- [ ] **KEK from a real Secrets Store binding on the deployed edge (ADR-0025 R3).** Migrate `KEK` (and ideally `HMAC_KEY`) from the `env.var("KEK")` plaintext binding to a real `[[secrets_store_secrets]]` binding (`env.secret_store(...)?.get()`); the deploy is operator-gated (`docs/runbooks/deploy-worker.md` step 4b).
  - **WHEN:** when miniflare emulates Secrets Store / a deploy-hardening pass.

- [ ] **GroupHub DO GroupKey cache (the task-title "GroupKey cache").** Optionally cache the unwrapped Group key in the `GroupHub` DO (saves one `delegated_keys` SELECT + unwrap per request) instead of the per-request unwrap-then-drop — only with a write-through/evict-on-rotate guard.
  - **WHEN:** a perf pass, if the per-request unwrap is measured as a bottleneck.

- [ ] **Defense-in-depth: the Worker verifies the asserted `X-Admin-Id` actually holds `role=admin` (ADR-0026).** Add a `members` lookup asserting the BFF-asserted admin id is a real Admin in the Group, so a leaked secret cannot forge an arbitrary actor on the I5 audit trail.
  - **WHEN:** a security-hardening pass (with the T10 real BFF call).

- [ ] **`edit_member` into a duplicate phone returns an opaque 500, not a calm conflict (T07/T05 carry-forward).** Map the `(group_id, phone_lookup_hash)` unique-violation to a distinguishable `EditMemberOutcome` conflict arm in the core (surfaced like issuance's `ADMIN_MEMBER_DUPLICATE_PHONE`, audited) — the store must return a typed conflict, not an opaque `StoreError::Db`.
  - **WHEN:** a focused edit-conflict slice (core `EditMemberOutcome` + a `pg_member_store_edit_into_duplicate_phone` test).

- [ ] **Server-side member search/filter (`?search=&role=&status=`).** The list route accepts the filters but does not apply them; wire the SQL `WHERE` + the params (core `list_members` takes no filter today).
  - **WHEN:** **T07/T10** (server-side filtering + the UI that drives it).

- [ ] **Live `emit()` sink + the member-issuance I10 scrubber fixture + the StoreError/PII-off-logs wiring.** Add the deployable scrubbed `boundless::logging::emit()` sink + route `StoreError` + the KEK-load `SecretboxError` variant (T04 L1 carry-forward) through it + a member-issuance red-team fixture.
  - **WHEN:** **T07-shell-B** (the live `emit()` sink).

- [ ] **Deployed-edge cross-tenant proof (AC16) as the live `boundless_app` role.** Prove a Group-A admin token cannot read Group-B members on the deployed Worker — needs ≥2 seeded Groups.
  - **WHEN:** **T11** (operator-gated, with ≥2 issued Groups).

---

## Admin web / SvelteKit member-management (spec 008 T10 — out-of-scope register)

- [ ] **Live deployed BFF→Worker round-trip (the real `WorkerMembersClient` over the network).** Run the real SvelteKit Worker calling the real Rust Worker with the wrangler-set secret over Hyperdrive→Neon (the UI e2e drives the in-memory fake this side); request shape unit-tested in `members_client_request_shape.test.ts`. Closes the T09-register "real SvelteKit→Worker BFF call (ADR-0026)" end-to-end.
  - **WHEN:** the deploy-hardening pass (with the deployed Rust Worker) / **T11**.

- [ ] **`ADMIN_WORKER_BASE` + `ADMIN_API_SECRET` web bindings + the persistent admin-session store.** Declare both in `web/wrangler.toml` + `wrangler secret put` + a service binding (or Worker URL); persist admin **session data** out of the in-memory map (`src/lib/server/session.ts`) into KV/Postgres + add expiry/rotation; remove the dev-only `/api/test/{seed-member,seed-session,reset}` seams once the real backend lands.
  - **WHEN:** **T10-shell** / the deploy slice (rides the T15-shell persistent-session item).

- [ ] **Server-side member search/filter is BFF-passed but Worker-NOOP.** The list `load` forwards `?search=&role=&status=` but the real Worker/`core::list_members` ignores them (no filter param); wire the SQL `WHERE` + the core signature, then the BFF path is already correct.
  - **WHEN:** **T07/T09** core+store filter (the BFF + UI already pass the params).

- [ ] **Real `gsw`/RTL/`zz-ZZ` translations (Weblate + signed KV, ADR-0014).** Only the `en` catalog ships; shipping locales arrive through the translation pipeline + the signed KV manifest.
  - **WHEN:** the translation pipeline / manifest-service spec.

- [ ] **Manual NVDA/VoiceOver + Lighthouse pass (pre-GA).** Run the a11y-bar's manual screen-reader walkthrough + Lighthouse ≥95 (advisory).
  - **WHEN:** the persona-acceptance / a11y review pass before GA.

- [ ] **~34 catalog keys (17 spec + ~17 added) — product-owner review.** Confirm the spec's 17 `admin.members.*`/`admin.member.*` keys + the ~17 added affordance/status/onboarding-status/audit/nav keys, including the `admin.member.address_invalid` copy (the T01-review owner-confirm item).
  - **WHEN:** surface for confirmation; adjust copy if the owner prefers different wording.

- [ ] **The `/admin` placeholder home is unchanged (no redirect to `/admin/members`).** Future polish: redirect `/admin` → `/admin/members` post-sign-in (or make the home a dashboard).
  - **WHEN:** a navigation-polish pass (optional).

- [ ] **melt-ui dialogs render client-only (SSR caveat).** Add/edit dialog + menu content mounts on the client (`{#if $open}` + `use:melt`), so dialog markup is not in the SSR HTML.
  - **WHEN:** N/A (documented behavior).

- [ ] **Carry-forwards from the T10 review (reviewer — H1 fixed in-slice; the rest non-blocking).** Remaining: (M1) regenerate-code on a missing Group key surfaces an opaque 500 not the calm `admin.member.group_key_missing` — fix Worker/contract-side (add a 503/`ADMIN_GROUP_KEY_MISSING` arm to regenerate + a `group_key_missing` `RegenerateOutcome`); (M2) the `?edit=1` deep-link auto-open uses `onMount` (full-nav only) — a `$effect` keyed on `page.url` would re-open on SPA nav; (M3) the melt `$description` is attached to a branch-transient element in the add dialog and to the member name in the edit dialog — anchor it on a stable element / use a real description key; (L2) `WorkerMembersClient` parses `res.json()` on 400/409 without a try (latent — Worker always returns JSON `{error_code}`); (L3) the server-side BFF `fetch` has no `AbortSignal.timeout`.
  - **WHEN:** M1 — a contract+Worker regenerate-503 slice; M2/M3 — an a11y-polish pass; L2/L3 — the deploy-hardening pass.

---

## Server / Worker — cross-tenant deployed-edge proof (spec 008 T11 — out-of-scope register)

- [ ] **Optional: a Rust seed helper for the operator's Group-B seed against Neon.** The runbook's step 8 carries the seed SQL inline (run as `neondb_owner`, DIRECT endpoint); a dedicated script (the Neon analog of the `setup-worker-test-db.sh` Group-B block) only if the operator finds the inline SQL inconvenient.
  - **WHEN:** only if the operator asks.

- [ ] **Carry-forwards from the T11 review (reviewer M1/M2 fixed in-slice; the rest non-blocking).** Deferred items: (L1) the gate is a single `it()` running all four isolation channels (read/list/edit/regenerate) in sequence — splitting into one `it()` per channel gives cleaner per-channel CI signal (kept single to preserve the canonical `worker_cross_tenant_admin_cannot_read_other_group` name the tracker references). (security-auditor F2) the smoke passes `ADMIN_API_SECRET` as a `curl` argv `-H "authorization: Bearer …"` (visible in `ps`/`/proc/<pid>/cmdline`) — pass it via `curl -K -` (config on stdin) instead; folds into the deployed-edge `/readyz` hardening (T07-shell-B register). (L3) the `EXIT` trap is registered inside the opt-in block — hoist it if a second trap is ever added above.
  - **WHEN:** the deploy-hardening pass (F2 with the `/readyz` hardening; L1/L3 optional polish).

---

## Deploy tooling — member-management provisioning (spec 008 — DONE 2026-06-11)

- [ ] **Runbook UX hardening from the live run (2026-06-12).** Add runbook notes pre-empting the live-run snags: (a) re-running `provision-neon.sh` without pinning `BOUNDLESS_APP_DB_PASSWORD` rotates the app password → must `wrangler hyperdrive update` (the printed password is a credential); lead step-0's "Re-running…" note with "pin `BOUNDLESS_APP_DB_PASSWORD` to your saved value on every re-run, or you must update Hyperdrive." (b) the bootstrap example's `NoTls → NoTlsError` against Neon (already fixed). (c) long-connection-string paste wraps/inserts a space and the `OWNER_URL=` variable doesn't persist across command invocations — recommend grabbing the full string from the Neon dashboard (pooling OFF) and running each step as a single self-contained command (no cross-command shell variables).
  - **WHEN:** a runbook-polish pass.

- [ ] **Review carry-forwards (reviewer S2; both reviews otherwise clean — 0 crit/high/warning, sec-auditor "ship it").** `bootstrap-group.sh` takes the owner URL (with its password) as `$1` — visible in `ps`/shell history (mirrors `provision-neon.sh`, same argv-secret class as the T11 review's sec-auditor F2 item); fold an env-based owner-URL option into that deploy-hardening pass.
  - **WHEN:** the deploy-hardening pass.

- [ ] **Future robustness: a `schema_migrations` tracking table** (vs. the per-migration marker map). A tracking table (the standard approach) would remove the per-migration maintenance; deferred because it needs back-filling the existing 0001–0008 rows on already-migrated DBs, and ADR-0019 chose plain out-of-band SQL (no migration framework).
  - **WHEN:** if the marker map's maintenance becomes a burden / a migration-framework decision is revisited.

---

## Admin web deploy — store B1 surface (spec 009 T02 — out-of-scope register)

> T02 (the `AdminWebAuthnStore` Postgres impl + the PII-free wire DTOs) is DONE; these are the
> review carry-forwards (all low/nit — both reviews + the adversarial pass found 0 crit/high/confirmed).

- [ ] **Reconcile the AC4b / ADR-0027 wording with the as-built token match.** `resolve_invitation_by_token`
  / `consume_invitation` / `register_complete` realize "the match routes through the Rust core" (AC4b) as
  `core::admin_invitation_token_hash` + indexed equality on the unique `token_hash` (timing-safe — the
  compared value is a secret-keyed 256-bit HMAC, the `phone_lookup_hash`/`classify_refresh` precedent),
  **not** a call to the AC-named `admin_invitation_token_matches`. The store/ports doc comments explain
  this; lift that rationale into the AC4b text + ADR-0027 (≈ lines 59/134) so a future reader doesn't flag
  the missing `_matches` call as a regression.
  - **WHEN:** **T03** (when the contract is frozen / AC4b is ticked).

- [ ] **Worker re-checks the consumed invitation's admin still holds `role=admin` (defense-in-depth).**
  `register_complete` derives `admin_id` from the consumed invitation row (never web-supplied) and relies
  on the mint-time invariant that an `admin_invitations` row only ever FKs a `role=admin` member. When T04
  wires the route, add a defense-in-depth assertion (or document the reliance) that the derived `admin_id`
  still resolves to a `role=admin` member before binding the credential — mirrors the existing ADR-0026
  "Worker verifies asserted `X-Admin-Id` actually holds `role=admin`" item.
  - **WHEN:** **T04** (the deployable Worker route).

- [ ] **`StoreError` from the B1 admin-auth surface must reach the log only via the scrubbed `emit()`
  sink (P2/I10).** `StoreError::Db`'s `Display` can echo a unique-violation `DETAIL` containing the
  conflicting `bytea` (a `credential_id` or `token_hash` `\x…` blob). Add this admin-auth surface to the
  existing "Route `StoreError` through the scrubbed `emit()` path (sec-audit W4)" item, and extend the I10
  scrubber fixture's unique-violation `\x…` `DETAIL` case to cover a `credential_id`/`token_hash` conflict;
  T04 must log `StoreError` via `emit()`, never `{e}`/`{:?}`.
  - **WHEN:** **T04** (Worker logging) + the I10 scrubber suite (T08 of spec 001 / T07-shell-B).

- [ ] **`AdminWebAuthnStore` standalone primitives have no wire consumer yet.** `revoke_all_for_admin` (and
  `consume_invitation` standalone) ship as part of the full sanctioned port surface but are exercised only
  by tests / via `register_complete` until the **additive backup-key enrollment** flow lands (spec 001 T15
  register / a settings UI). Intentional (ADR-0027) — noted so they are not mistaken for unused methods.
  - **WHEN:** the backup-key enrollment flow.

---

## Admin web deploy — B1 contract freeze (spec 009 T03 — out-of-scope register)

> T03 (the `/api/admin/webauthn/*` OpenAPI freeze + the B1 contract describe-block, AC13) is DONE; these
> are the deferrals it recorded. The T02→T03 doc-reconciliation carry-forward was **closed in-slice**
> (AC4b + ADR-0027 now describe `admin_invitation_token_hash` + unique-index equality, not `_matches`).

- [ ] **Session-bearing backup-key B1 ops are intentionally NOT frozen yet.** T03 froze only the 4
  pre-session ops (the ADR-0027 freeze target). The authenticated additive backup-key enrollment flow
  needs a `GET /api/admin/webauthn/credentials?admin_id=` *list* + a standalone `revoke-all` wire op —
  both **session-bearing** (they WOULD carry `X-Admin-Id`, unlike the four). Freezing them now would be
  contract for an out-of-scope feature (spec.md "Out of scope"). When that flow is specced: add the ops,
  give each the `AdminIdHeader` param, and **update the contract test's `FROZEN_B1_OPS` list + the
  "exactly 4" count pin + the pre-session negative assertion** (those session-bearing ops must be excluded
  from the no-`X-Admin-Id` negative set).
  - **WHEN:** the additive backup-key enrollment spec (spec 001 T15 register).

- [ ] **Strict fixture/Rust ↔ OpenAPI conformance cross-check (hardening).** T03 pins the B1 wire shapes
  on BOTH sides independently — the Rust keyed-serde tests (`core/server/src/admin_webauthn.rs`) and the
  OpenAPI-side `b1_wire_dtos_are_pii_free_and_shaped` field-set assertion — but nothing programmatically
  proves the two agree (a reviewer did, by eye). Fold the B1 DTOs into the deferred host-only
  "fixture↔OpenAPI conformance" check (spec 001 T10 register): serialize an `AdminInviteRecord`/
  `AdminCredential` and validate it against the de-`$ref`'d OpenAPI schema, so a one-sided drift fails CI.
  - **WHEN:** the contract-hardening pass (rides the spec-001 T10 fixture↔OpenAPI item).

- [ ] **Live deployed-edge contract-conformance for the B1 surface.** T03 checks only the contract
  *document*; replaying real B1 responses (resolve/register-complete/lookup) against the deployed Worker
  to prove runtime conformance rides the deploy-hardening pass.
  - **WHEN:** **T13** (edge) / the deploy-hardening pass.

---

## Admin web deploy — B1 Worker endpoints (spec 009 T04 — out-of-scope register)

> T04 (the `/api/admin/webauthn/*` Worker handlers + the pre-session `admin_secret_guard`) is DONE.
> **Closed in-slice:** the T02-register "bless the B1 wire DTOs" item and the T03-register
> "AuditedResponse blessing list" item — T04 added `AdminRegisterCompleteResult` (`core/server`) and
> blessed all three bare B1 DTOs `AuditedResponse` (no envelope wrappers exist). The I5 trybuild
> `.stderr` golden was re-blessed (the new types appear in rustc's "other types implement the trait"
> hint — the same mechanism as the T06-register toolchain-bump re-bless; the E0277 compile-fail is
> unchanged). The two new value-free 404 codes (`ADMIN_INVITE_NOT_FOUND`/`ADMIN_CREDENTIAL_NOT_FOUND`)
> are registered in `docs/error-codes.md`.

- [ ] **The web Worker-backed adapters (`WorkerInviteStore`/`WorkerCredentialStore`) consume these
  endpoints — built at T05.** T04 ships only the Worker side; the SvelteKit adapters that POST the token
  in the body + map the value-free 404 → `null` (and the `selectInviteStore`/`selectCredentialStore`
  fail-closed selectors) are T05.
  - **WHEN:** **T05** (web Worker-backed stores + selectors).

- [ ] **Route the B1 `StoreError` through the scrubbed `emit()` sink + an I10 fixture (P2/I10).** T04's
  handlers map any `StoreError` to a generic value-free 500 and never touch the token on a log path (the
  presented token is tainted `AdminInvitationToken` + arrives in the body, never logged — R13-clean). But
  the deployable scrubbed `boundless::logging::emit()` sink doesn't exist yet, so `StoreError` isn't
  routed through `detect_pii`. When the sink lands, route the B1 `StoreError` through it and extend the
  unique-violation `\x…`-`DETAIL` scrubber fixture to a `credential_id` conflict (folds into the
  spec-009 T02-register "StoreError via emit()" item).
  - **WHEN:** **T07-shell-B** (the live `emit()` sink) + the I10 scrubber suite.

- [ ] **`register-complete` on a duplicate `credential_id` surfaces an opaque 500, not a clean code.** A
  ceremony that (near-impossibly) produced a `credential_id` already in the table makes the
  `register_complete` insert hit the global unique index → `Err(StoreError::Db)` → generic 500 (the
  OpenAPI froze only 200/400/401 for register-complete). Acceptable (a fresh-ceremony collision is
  effectively impossible); revisit only if a clean conflict code is ever wanted.
  - **WHEN:** only if a `register-complete` duplicate-credential conflict needs a distinct code.

- [ ] **R7 (verify the asserted `X-Admin-Id` is a real admin) does NOT apply to the B1 ops.** The
  pre-session B1 endpoints carry **no** `X-Admin-Id` (the admin is being registered/authenticated), so
  there is nothing to verify — noted so a future reader doesn't flag the pre-existing member-surface R7
  item as missing here. (The `register-complete` admin id is derived from the consumed invitation row,
  never web-supplied.)
  - **WHEN:** N/A for B1 (the member-surface R7 item stands on its own — DEFERRED spec 008 T09).

- [ ] **The register-invite POOL (24 tokens) is a test-harness convenience.** `seed_worker_test_b1_pg.rs`
  seeds `boundless-test-invite-register-{0..24}` so the single-use register-complete round-trip survives
  re-runs without a re-seed; the test claims the first still-live one. These are TEST invites only — the
  operator seed (T10) mints real invites via `create_pending_admin_with_invitation`. If a dev exhausts
  the pool (24+ `pnpm test` runs without re-`setup-worker-test-db.sh`), the test fails with a re-run
  hint; bump `REGISTER_POOL` (kept in lock-step in the example + the spec) if that ever bites.
  - **WHEN:** N/A (documented harness behavior) / bump the pool only if exhaustion bites.

- [ ] **Live deployed-edge B1 round-trips + cross-tenant probe (the AC14 edge leg).** T04 proves the
  miniflare+PG legs (round-trip + `worker_cross_tenant_invite_resolve_isolated`); the deployed-edge
  versions (against Neon, ≥2 Groups) ride the operator-gated smoke (T12/T13).
  - **WHEN:** **T13** (edge) / the deploy-hardening pass.

---

## Constitution

- [ ] **Replace `Ratified: TODO`** in `.specify/memory/constitution.md` with a real date.
  - **WHEN:** when you formally adopt the constitution.
