# 008 — Admin member-management (issuance): Tasks

> Status: Ready for `/speckit.implement` — 2026-06-10
> Derived from `plan.md` §10 (sequencing) + §11 (AC→test map). The plan is the design; this is the
> contract for what gets built. **Anything not in this file is scope creep (P6).**
>
> **Task numbering is spec-008-local (T01–T11), distinct from spec 001's T01–T16.**
>
> **Convention (per the AC-bookkeeping rule):** do *not* tick the spec.md AC boxes per task. The
> **AC-coverage tracker** below is the live status. `✓` means *functional-core / test-covered*, **not**
> deployably shipped — deployable-shell legs (Worker/edge/e2e) are marked `[shell]` and a task is only
> fully done when its gating tests are green and the post-edit/pre-commit/pre-push hooks pass.
>
> One task = one PR-sized slice (functional-core-first, with its deferred shell recorded). Pick one,
> start a fresh session, `/speckit.implement`, `/compact` + end the session when its gating tests pass.

---

## Dependency graph & parallelism

```
T01 (docs/codes)        ─┐
T02 (core/crypto)       ─┤ independent — may run in parallel
T03 (migrations)        ─┘
        T02 ──> T04 (bootstrap/key-gen)
        T02 ──> T05 (MemberService) ──> T06 (#[require_audit] compile gate)
   T02+T03+T05 ──────────────────────> T07 (PgStores, real PG18)
        T05 ──> T08 (OpenAPI freeze + contract test)        [T06's audit set referenced]
   T07+T08 ──────────────────────────> T09 (Worker endpoints + KEK + live CSPRNG)
        T08 ──> T10 (SvelteKit UI)     [e2e needs T09's Worker]
        T09 ──> T11 (cross-tenant deployed-edge proof)      [operator-gated]
```

**Hard serialization points:** T02 precedes T04/T05/T07; T03 precedes T07; T05 precedes T06/T07/T08;
T07 **and** T08 precede T09; T09 precedes T11.
**Safely parallel:** {T01, T02, T03}; later {T06, T07, T08} can overlap once T05 lands (T06 and T08 both
read T05's types/projections; T07 is the DB leg). T10's UI can be built against the T08 contract while
T09 is in flight, but its e2e needs T09's Worker.

---

## Tasks

### T01 — Docs, error codes, runbook stub, DEFERRED/stack-matrix reconciliation
- **What:** Land the non-code scaffolding the rest of the spec returns typed codes/keys against.
- **Touches:** `docs/error-codes.md` (+5 codes), `docs/runbooks/key-management.md` (NEW stub),
  `docs/stack-matrix.md` (resolve the dryoc "sealed-box/secretbox" hedge → **secretbox** for field-level
  PII at rest; sealed boxes reserved for I9), `DEFERRED.md` (repoint the spec-008 I1 items to ADR-0025;
  record the new deferred shells — live `emit()` issuance fixture, KEK/Group-key rotation Workflow, the
  I12 `forget_member` sweep must cover `name_encrypted`/`address_encrypted`/`audit_log`),
  `core/server/tests/` error-code parity test.
- **Codes (append-only, PII-free):** `ADMIN_MEMBER_DUPLICATE_PHONE`, `ADMIN_MEMBER_EDIT_STALE`,
  `ADMIN_MEMBER_PHONE_INVALID`, `ADMIN_MEMBER_ADDRESS_INVALID`, `ADMIN_GROUP_KEY_MISSING`.
- **Closes (partial):** P12 scaffolding for AC1/AC6/AC11/AC12 error paths.
- **Tests:** error-code-registry parity test extended to the 5 new codes; `docs/runbooks/key-management.md`
  documents per-Group key gen + KEK access + **annual + on-compromise** KEK re-wrap rotation
  (`kek_version`) + the deferred Group-key re-encrypt procedure (ADR-0025).
- **Blockers:** none. **Parallel:** with T02, T03.

### T02 — `core/crypto` secretbox + `GroupKey`/`Kek` (zeroize) + tainted `Address`/`MemberName` + injected nonce
- **What:** The field-encryption primitive (ADR-0025) and the new tainted types. **The load-bearing slice.**
- **Touches:** `core/crypto/src/secretbox.rs` (NEW) + `core/crypto/src/lib.rs`; `core/crypto/Cargo.toml`
  (+`zeroize`); `core/domain/src/tainted.rs` (+`Address`, `MemberName` via the `tainted_secret!` macro);
  `core/server/src/secrets.rs`/`ports.rs` (extend `SecretSource` with `fresh_nonce()`);
  `core/crypto/tests/invariants.rs` (NEW — the file I1's enforcement names);
  `core/crypto/proptest-regressions/.gitkeep` (NEW — keeps the auto-discovery gate green).
- **Design constraints (plan §5):** exactly **one** `encrypt_field(plaintext, &GroupKey, &Nonce)` (no
  nonce-less overload); nonce is a **fresh random draw from the injected CSPRNG** (no ambient randomness —
  ADR-0021); `GroupKey`/`Kek` have no `Debug`/`Display`/`Serialize` **and** `impl Drop` zeroize; dryoc
  `crypto_secretbox` fn names + nonce length **pinned via docs-researcher** against locked dryoc 0.8.0
  first; pin `zeroize` from the lock.
- **Closes:** AC2 (`i1_addresses_encrypted`), AC3 (`i1_name_encrypted`) at the crypto layer.
- **Tests:** `i1_addresses_encrypted`, `i1_name_encrypted` (ciphertext≠plaintext; stored `nonce ‖
  ciphertext`; wrong key ⇒ `Err`; tamper ⇒ `Err`); `assert_not_impl_any!` on
  `Address`/`MemberName`/`GroupKey`/`Kek`; `prop_secretbox_round_trip_and_ciphertext_differs`,
  `prop_secretbox_nonce_unique_across_calls`, `prop_decrypt_wrong_key_fails`,
  `prop_kek_wrap_unwrap_round_trips`; `cargo build --target wasm32 -p boundless-crypto` clean.
- **After:** regenerate `api/.bindings.lock` (core changed). **Blockers:** none. **Parallel:** T01, T03.

### T03 — Migrations `0009_delegated_keys`, `0010_member_pii`, `0011_audit_log`
- **What:** The schema (plan §4). Reversible, FORCE-RLS, group-scoped, append-only audit.
- **Touches:** `server/migrations/0009_*.{up,down}.sql`, `0010_*`, `0011_*`; `server/tests/migrations.rs`
  (`EXPECTED_VERSIONS` → 1..=11).
- **Shapes:** `delegated_keys(group_id PK, wrapped_key bytea, kek_version int, created_at/updated_at,
  created_by)`; `members ADD COLUMN name_encrypted bytea, address_encrypted bytea` (nullable);
  `audit_log(id, group_id, admin_id, member_id, fields text[], request_id, created_at)` — **no
  `_encrypted` column**, **no `updated_at` trigger** (append-only; note the divergence in the header).
  All three ENABLE + **FORCE** RLS with `group_id = current_group_id()`.
- **Closes:** AC12 (structure), AC2/AC3 (columns), AC9 (audit shape) at the schema layer.
- **Tests:** `server/tests/migrations.rs` (bytea columns; `audit_log.fields text[]`; no `_encrypted` on
  audit; FORCE RLS on all three; append-only divergence asserted); `scripts/test-migrations.sh` live
  apply/RLS/revert on real PG18 (`server-migrations` job).
- **Blockers:** none (shapes decided in the plan). **Parallel:** T01, T02.

### T04 — Group bootstrap + per-Group key generation (core decision + injected RNG)
- **What:** The bootstrap *decision* that mints the Group key from the injected CSPRNG, wraps it with the
  KEK, and shapes the `groups` + `delegated_keys` write; issuance fails closed without a key.
- **Touches:** `core/server/src/member.rs` (or a `bootstrap` module) + `ports.rs`.
- **Closes:** AC12 (the generation + fail-closed decision).
- **Tests:** `bootstrap_generates_wrapped_key_from_injected_seed` (wrapped blob ≠ plaintext; round-trips
  via KEK); `member_service_issuance_fails_closed_without_group_key` (returns `ADMIN_GROUP_KEY_MISSING`,
  writes no row; no `unwrap()` on key load).
- **Blockers:** T02. **Parallel:** with the early part of T05.

### T05 — `core/server` `MemberService` + ports + projections + audit decision
- **What:** The pure issuance/edit/regenerate orchestration behind new `MemberStore`/`AuditStore`/
  `DelegatedKeyStore` ports; the `MemberSummary`/two-type-`MemberDetail` projections; the audit decision.
- **Touches:** `core/server/src/member.rs` (NEW), `core/server/src/ports.rs`,
  `core/server/src/lib.rs`; `core/server/tests/` (new `member.rs`/`audit.rs`/`properties.rs`).
- **Design constraints (plan §3/§6/§7):** `MemberSummary = {member_id, name: String, roles: Vec<Role>,
  onboarding_status}` — **no tainted type**. Core `MemberDetail` carries tainted `Address`/`PhoneNumber`
  (cannot derive `Serialize`); the **wire** `MemberDetail` is a *separate* serializable DTO built via
  `expose_secret()` at the Worker boundary (parity R1 — the split lives here). `AuditEntry.fields` is an
  `AuditField` enum / `&'static str` (names, never values). Reject `Role::Admin` at issuance (I11).
  Optimistic-concurrency *decision* on `updated_at`. `SecretSource::fresh_onboarding_code`.
- **Closes:** AC1, AC4, AC8, AC11 (decision), AC13; partial AC5/AC6 (mint decision), AC10 (admin-role
  reject), AC12 (fail-closed path uses T04).
- **Tests:** `member_service_issues_rider_and_driver`, `member_service_accepts_multi_role_set`,
  `member_service_edit_reencrypts_and_recomputes_phone_hash`, `member_service_stale_edit_rejected`,
  `member_service_rejects_admin_role_on_issuance`, `member_service_mints_one_live_onboarding_code`,
  `member_summary_holds_no_tainted_type`, `member_list_emits_no_audit_event`,
  `audit_entry_carries_field_names_ts_admin_member_request`; props
  `prop_member_summary_never_carries_pii`, `prop_every_pii_detail_read_emits_audit`,
  `prop_phone_change_recomputes_matching_hash`.
- **After:** regenerate `api/.bindings.lock`. **Blockers:** T02 (T04 for the fail-closed leg).

### T06 — `#[require_audit]` compile-time gate (I5)
- **What:** Make "a function returning a tainted-carrying type cannot be wired without producing an
  `AuditEntry`" a **compile error** (sealed-trait bound on router registration acceptable; literal
  proc-macro is the stretch). Plan §7/§14 — this is dictated by I5; do **not** weaken to test-only.
- **Touches:** `core/server` (or a new `core/macros` crate); `core/server/tests/compile-fail/`.
- **Closes:** AC7 (compile leg).
- **Tests:** `require_audit_compile_fail` (a PII-returning handler/type lacking the obligation **fails to
  build**); `member_summary_rejects_tainted_field` (trybuild). Pin `trybuild` (dev) from the lock.
- **Blockers:** T05 (the ports/`AuditEntry`/tainted-carrying types exist). **Parallel:** with T07/T08.

### T07 — `PgMemberStore` / `PgAuditStore` / `PgDelegatedKeyStore` (real PG18)
- **What:** The Postgres adapters for the T05 ports. All via `begin()` RLS scoping, `query_typed*`
  (ADR-0024), **no raw `Client` accessor**.
- **Touches:** `server/store/src/` (new store modules), `server/store/tests/` (new `members.rs`,
  `audit.rs`, `delegated_keys.rs`, mirroring `integration.rs`/`admin_invitations.rs`).
- **Design constraints (plan §8):** member create + code mint in **one txn**; regenerate =
  supersede-then-insert in one txn against the `onboarding_codes_one_live_per_member` partial-unique index
  (`pg_advisory_xact_lock` per member if concurrent); optimistic concurrency via `UPDATE … WHERE
  updated_at = $expected` (0 rows ⇒ `ADMIN_MEMBER_EDIT_STALE`); audit INSERT atomic with the detail SELECT
  (R5); `ensure_least_privilege` reused as the boot precondition.
- **Closes:** AC1/AC2/AC4/AC6/AC9/AC11/AC12/AC13 (DB legs).
- **Tests (real PG18, `server-store` job, self-skips without `DATABASE_URL`):**
  `pg_member_store_persists_member_with_roles_and_created_by`, `…_address_encrypted_round_trip`,
  `…_phone_two_fold`, `…_roles_array_round_trip`, `…_regenerate_supersede_then_insert_atomic`,
  `…_optimistic_concurrency_stale_reject`, `…_issue_is_atomic`, `pg_audit_store_writes_row_on_detail_read`,
  `…_read_returns_no_pii`, `pg_delegated_key_store_persists_only_wrapped`,
  `rls_isolates_member_reads_by_tenant`, `prop_rls_isolates_random_two_group_configs`.
- **After:** `cargo build --target wasm32 -p boundless-server-store` clean. **Blockers:** T02, T03, T05.

### T08 — OpenAPI `/api/admin/*` freeze + contract test
- **What:** Add the admin paths/schemas to `api/openapi.yaml` (additive; auth shapes frozen) and the
  contract tests. Plan §6.
- **Touches:** `api/openapi.yaml`, `web/tests/contract/api-contract.test.ts`,
  `api/.bindings.lock` (regenerate), `docs/error-codes.md` (parity), `web/src/lib/server/members.ts`
  (typed client provenance — hand-rolled-but-derived for v1, plan §6).
- **Design constraints:** two-type `MemberDetail` (wire = plain strings); define `OnboardingStatus` once
  (mirror one core enum); audited-field vocabulary single-sourced; reuse `Role` by `$ref`; `member_id`
  `{type:string, format:uuid}` (copy `DeviceBound`).
- **Closes:** AC7 (OpenAPI-coverage leg), AC9, AC10 (no admin-creation path).
- **Tests:** `openapi_pii_handlers_all_require_audit`, `member_summary_schema_has_no_tainted_field`,
  `admin_issuance_error_codes_in_registry`, `openapi_admin_surface_has_no_admin_creation_path`; the
  existing `/api/auth/*` freeze test still green; `scripts/check-binding-drift.sh` lock refreshed.
- **Blockers:** T05 (projection shapes), references T06's audit set. **Parallel:** with T06/T07.

### T09 — Worker endpoints + KEK binding + GroupKey cache + live CSPRNG `[shell]`
- **What:** The deployable `/api/admin/members/*` routes composing `MemberService` over the Pg stores;
  loads the KEK from **Secrets Store**, caches the unwrapped `GroupKey` in the `GroupHub` DO; wires the
  **live injected CSPRNG** (the nonce/key source — R1, wired here, not deferred).
- **Touches:** `server/src/runtime/members.rs` (NEW) + `runtime/mod.rs` (`Router`, `build_service`
  analog); `server/wrangler.toml` (NEW `KEK` Secrets-Store binding; `send_metrics=false`).
- **Design constraints (plan §5/§6):** KEK via the Secrets Store binding API (not `env.var`); boot
  fails closed without the KEK; keep inbound raw `name`/`address`/`phone` off the log path and out of
  error responses (R10); duplicate-phone surface-and-link is admin-only + audited (R9).
- **Closes:** AC1/AC5/AC6/AC10 (HTTP legs); AC7 (live audit emit).
- **Tests (miniflare + local PG18, `worker` job):** `worker_issue_member_round_trip`,
  `worker_detail_read_emits_audit`, `worker_regenerate_code`, `worker_duplicate_phone_links_existing`,
  `worker_error_response_contains_no_submitted_pii` (no substring of submitted name/address/phone in any
  error body).
- **Blockers:** T07, T08. Extend the `wrangler.toml` committed-credential grep gate to forbid a
  `KEK`/`GROUP_KEY` value.

### T10 — SvelteKit admin UI + i18n + a11y `[shell]`
- **What:** The admin screens behind the existing session: member list (search+filter via **TanStack
  Table/Query**), add/edit dialogs + member menu (**melt-ui**), member detail (audited read), audit-log
  view, regenerate-code; the 17 i18n keys.
- **Touches:** `web/src/routes/(admin)/members/*` (NEW), `web/src/lib/server/members.ts`,
  `web/src/lib/i18n/catalog.ts`, `web/tests/e2e/members.spec.ts` (NEW),
  `web/tests/cross-platform/catalog-parity.test.ts` + `web/tests/e2e/pseudo-locale.spec.ts` (extend).
- **Design constraints:** **pin melt-ui + TanStack versions from `web/pnpm-lock.yaml` via docs-researcher
  — never invent.** A11y bar (AC14): axe zero-violations per route × {default,dark,RTL}; keyboard-complete
  dialogs/menus (focus trap, Esc, focus return); `aria-live` on validation + audit states; 400% reflow.
- **Closes:** AC14, AC15; client legs of AC1/AC6/AC9/AC10.
- **Tests:** `members_routes_axe_clean_default_dark_rtl`, `members_add_edit_dialog_keyboard_ceremony`,
  `members_list_reflows_at_400_percent`, `audit_log_validation_aria_live`,
  `members_ui_offers_no_create_admin_action`, `members_pseudo_locale_renders_without_truncation`,
  `admin_members_catalog_parity`; `scripts/check-network-allowlist.sh` clean across the grown web lock;
  `i18n-validator` subagent pass.
- **Blockers:** T08 (contract); e2e needs T09 (a running Worker/dev server).

### T11 — Cross-tenant deployed-edge proof (AC16) `[shell]` — operator-gated
- **What:** With ≥2 seeded Groups, prove a Group-A admin token cannot list/read/edit Group-B members on
  the **deployed edge** as the locked-down `boundless_app` role. Closes the long-open sec-audit **F5**.
- **Touches:** `scripts/smoke-deployed-edge.sh` (extend); `docs/runbooks/deploy-worker.md` (note the
  ≥2-Group seeding + the AC16 check).
- **Closes:** AC16.
- **Tests:** `cross_tenant_admin_cannot_read_other_group` on the live edge; the in-process precursor
  `rls_isolates_member_reads_by_tenant` (T07) already proves the *policy*.
- **Blockers:** T09 deployed + ≥2 Groups issued. **Deployable-shell-only** — gated on the operator's
  deploy (Cloudflare MCP is read-only; the human runs `wrangler deploy`). AC16 = host-precursor-covered
  until this runs live.

---

## AC coverage map (the live tracker — `✓` = test-covered in core/CI, `[shell]` = deployable leg)

| AC | Covered by | Status |
|---|---|---|
| AC1 create member (roles[], created_by, RLS-scoped) | T05 (core), T07 (DB), T09 [shell] | pending |
| AC2 address encrypted at rest (`i1_addresses_encrypted`) | T02 (crypto), T03 (column), T07 (DB) | pending |
| AC3 name encrypted at rest | T02, T03, T05 | pending |
| AC4 phone two-fold (I3) | T05, T07 | pending |
| AC5 mint one live Onboarding Code | T05 (decision), T07 (DB), T09 [shell] | pending |
| AC6 regenerate atomic supersede-then-insert | T05, T07, T09 [shell] | pending |
| AC7 PII reads audit-logged + `#[require_audit]` compile + OpenAPI coverage | T06 (compile), T05 (decision), T07 (DB), T08 (coverage), T09 [shell emit] | pending |
| AC8 `MemberSummary` no tainted type | T05 (compile assert) | pending |
| AC9 read audit log (names not values) | T03 (shape), T05, T07, T08 | pending |
| AC10 no admin-creation affordance (I11) | T05 (role reject), T08 (no path), T10 [shell UI] | pending |
| AC11 edit re-encrypts + recompute hash + optimistic concurrency | T05 (decision), T07 (DB) | pending |
| AC12 Group bootstrap + per-Group key, fail-closed | T02, T03, T04, T07 | pending |
| AC13 roles[] at issuance, swap out of scope | T05, T07 | pending |
| AC14 a11y (WCAG 2.2 AA, axe, dialogs/menus) | T10 [shell] | pending |
| AC15 i18n + pseudo-locale | T10, T01-catalog | pending |
| AC16 cross-tenant deployed-edge proof (F5) | T11 [shell], T07 (host precursor) | pending |

Every task maps to ≥1 AC; no task introduces behavior absent from `spec.md`. Out-of-scope items (geocoding,
deletion/I12, device-token encryption, role-swap workflow, remote-only, O5/O7, matching, bulk import)
stay out per the spec's "Out of scope" section and are tracked in `DEFERRED.md`.

---

## Deferred shells (recorded in DEFERRED.md at T01)

- Live `boundless::logging::emit()` sink + the member-issuance I10 scrubber fixture (T07-shell-B track).
- `PgDeviceStore` device-token at-rest encryption — now *unblocked* by T02's secretbox key (push spec 007).
- KEK re-wrap rotation tooling + the Group-key re-encrypt Workflow (runbook-documented, unbuilt — ADR-0025).
- The I12 `forget_member` sweep must cover `name_encrypted`/`address_encrypted`/`audit_log` (core::deletion spec).
- Geocoding/ETA Workflow (matching spec, architecture flow D).
