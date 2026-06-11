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
- **Status:** ✅ DONE 2026-06-10 — commit `af2e701` (reviewer + security-auditor: 0 crit/high).
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
- **Status:** ✅ DONE 2026-06-10 — commit `6490876` (reviewer + security-auditor + platform-parity: 0 crit/high).
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
- **Status:** ✅ DONE 2026-06-10 — commit `72a7193` (reviewer + security-auditor: 0 crit/high; live apply/RLS/revert proven on real PG18).
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
- **Status:** ✅ DONE 2026-06-10 — commit `55813e0` (reviewer + security-auditor + platform-parity:
  0 crit/high; security-auditor "ship it", parity clean).
- **What:** The bootstrap *decision* that mints the Group key from the injected CSPRNG, wraps it with the
  KEK, and shapes the `delegated_keys` write; issuance fails closed without a key.
- **Touches:** `core/server/src/bootstrap.rs` (NEW — chose the **`bootstrap` module** option, not
  `member.rs`: `member.rs` is T05's file and T04/T05 are parallel children of T02), `ports.rs`
  (+`SecretSource::fresh_group_key`), `secrets.rs` (`RngSecretSource::fresh_group_key`, zeroized draw),
  `lib.rs` (re-exports), `Cargo.toml` (+`zeroize`), the three existing `SecretSource` impls
  (`tests/common/mod.rs` + `server/store/tests/service_pg.rs` `SeqSecrets`; `server/src/runtime/pg.rs`
  `PlaceholderSecrets` → `unreachable!`), `tests/error_codes.rs`. **Regenerated `api/.bindings.lock`.**
- **Closes:** AC12 (the generation + fail-closed decision).
- **Tests:** `bootstrap_generates_wrapped_key_from_injected_seed` (wrapped blob ≠ plaintext; round-trips
  via KEK; blob wraps the *same* key cached for the DO); `issuance_fails_closed_without_group_key`
  (**renamed** from the plan's `member_service_issuance_fails_closed_without_group_key` — there is no
  `MemberService` in T04; the gate `load_group_key` returns `ADMIN_GROUP_KEY_MISSING` on a missing/
  corrupt/wrong-KEK key with no `unwrap()`, and T05 wires it into `MemberService::issue` as issuance's
  first step so no row is written). Plus `bootstrap_generates_distinct_keys_per_group` (R1 cross-isolate)
  and `group_key_missing_error_code_registered` (P12: ties the emitting type to the registry).
- **Blockers:** T02. **Parallel:** with the early part of T05.

### T05 — `core/server` `MemberService` + ports + projections + audit decision
- **Status:** ✅ DONE 2026-06-10 — commit `7cb2e83` (reviewer + security-auditor + platform-parity:
  0 crit/high; security + parity "ship", reviewer "fix-then-ship" — all findings applied). 4-lens
  design panel + 3-lens review panel (each adversarially verified).
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
  reject), AC12 (fail-closed path reuses T04's `GroupKeyMissing` gate).
- **Decisions (4-lens panel; see `DEFERRED.md` → T05 register):** (A) Admin is **unrepresentable** at
  issuance — `IssuableRole {Rider,Driver}` + `issuable_roles(Vec<Role>)` → `AdminRoleForbidden` →
  **`ADMIN_MEMBER_ROLE_FORBIDDEN`** (6th issuance code). (B) audit WRITE folded into
  `read_member_detail_audited` (one txn, I5/§7); `AuditStore` read-only. (C) `DelegatedKeyStore`
  returns **wrapped bytes**; the service holds the `Kek` + reuses T04 `load_group_key`. (D) phone
  normalized **in-core**. (E) duplicate-phone is a first-class `IssueMemberOutcome::DuplicatePhone`
  (name-only, audited). **Module placement** (like T04): the 3 ports live in `member.rs`, not `ports.rs`.
- **Tests (all green):** the 12 named below + **panel-added**: `member_service_stores_phone_hash_and_ciphertext`
  (AC4), `member_service_rejects_invalid_phone_and_address`, `member_service_rejects_empty_roles` +
  `member_service_edit_to_empty_roles_rejected` (AC13 — the reviewer-found empty-roles gap → new
  **`ADMIN_MEMBER_ROLES_REQUIRED`** code, 7th), `member_service_duplicate_phone_links_existing_and_audits`,
  `member_service_regenerate_supersedes_decision`, `member_service_edit_role_only_needs_no_group_key`,
  `member_service_issuance_fails_closed_without_group_key`, the audit/casing/two-type-split asserts +
  `member_detail_view_wire_keys_are_pinned`. Named: `member_service_issues_rider_and_driver`,
  `member_service_accepts_multi_role_set`, `member_service_edit_reencrypts_and_recomputes_phone_hash`,
  `member_service_stale_edit_rejected`, `member_service_rejects_admin_role_on_issuance` (exercises
  `issuable_roles`), `member_service_mints_one_live_onboarding_code`, `member_summary_holds_no_tainted_type`
  (compile assert), `member_list_emits_no_audit_event`,
  `audit_entry_carries_field_names_ts_admin_member_request`; props `prop_member_summary_never_carries_pii`,
  `prop_every_pii_detail_read_emits_audit`, `prop_phone_change_recomputes_matching_hash`.
- **After:** regenerated `api/.bindings.lock` (80 inputs). **Blockers:** T02 + T04 (both done).
- **Two new codes** beyond T01's five (both required by P12 for their emitting types, both ship this PR):
  `ADMIN_MEMBER_ROLE_FORBIDDEN` (AC10 admin-role reject) + `ADMIN_MEMBER_ROLES_REQUIRED` (AC13 ≥1-role).

### T06 — I5 require-audit compile gate (sealed `AuditedResponse` / `PiiDisclosure`)
- **Status:** ✅ DONE 2026-06-10 — commit `c8d1b79` (reviewer + platform-parity "ship";
  security-auditor "ship-with-fixes" — all findings applied: the no-`Debug` pin on the PII carrier,
  the doc-scope tightening, the register heading). A 3-mechanism design panel + adversarial stress
  picked the plan §7 sealed-trait gate and **hardened** it; a 3-lens review followed (0 crit/high).
- **What:** Make "a function returning a tainted-carrying type cannot be wired without producing an
  `AuditEntry`" a **compile error** (sealed-trait bound on router registration acceptable; literal
  proc-macro is the stretch). Plan §7/§14 — this is dictated by I5; do **not** weaken to test-only.
- **Shipped (not a `#[require_audit]` attribute macro — the proc-macro stretch was NOT built):** the
  un-forgeable `PiiDisclosure<T>` carrier (`pub(crate)` ctor, delegating `Serialize`, no `Debug`/`Display`)
  + the **sealed** `AuditedResponse` bound + the `admin_response_body` send-seam + a hand-curated PII-free
  allowlist (`MemberSummary`/`Vec<MemberSummary>`/`Vec<AuditEntry>`), and `MemberDetailView` hardened
  (private fields + `pub(crate) to_wire`, keeps `Serialize` only for the disclosure's delegation).
  `DetailRead::Detail` now carries `Box<PiiDisclosure<MemberDetailView>>`. The residual `expose_secret`/
  hand-rolled-`json!` egress is the plan-sanctioned scope boundary → T08 OpenAPI-coverage + T09 lint
  (`DEFERRED.md` → T06). `core/server/src/audited.rs`; `serde_json` promoted dev→runtime; `trybuild` 1.0.116 (dev).
- **Touches:** `core/server/src/{audited.rs (NEW),member.rs,lib.rs}`; `core/server/tests/{audit.rs,no_formatter.rs}`;
  NEW `tests/require_audit.rs` + `tests/member_summary_compile.rs` + `tests/ui/**` (3 compile-fail + 1 pass fixture + `.stderr`).
- **Closes:** AC7 (compile leg).
- **Tests (green):** `require_audit_compile_fail` (forge `PiiDisclosure` → E0624; send a non-`AuditedResponse`
  body → E0277; pass fixture compiles) + `member_summary_rejects_tainted_field` (tainted field in a
  `Serialize` projection → E0277); `no_formatter.rs` pins `PiiDisclosure<MemberDetailView>`/`MemberDetailView`
  as `!Debug`/`!Display`; `audit.rs` trait-membership asserts. `trybuild` pinned from the lock (1.0.116).
- **Blockers:** T05 (the ports/`AuditEntry`/tainted-carrying types exist) — done. **Parallel:** with T07/T08.

### T07 — `PgMemberStore` / `PgAuditStore` / `PgDelegatedKeyStore` (real PG18)
- **Status:** ✅ DONE 2026-06-11 — commit `fd3a4e1` (reviewer + security-auditor + platform-parity:
  **0 confirmed findings**; one doc-citation nit fixed in-slice). One struct `PgMemberStore` implements
  all three ports (`server/store/src/members.rs`). 16 new tests green on real PG18 + the harness
  migration-count fix (8→11) that unblocked the previously-red existing store suites. All via `begin()`
  (RLS), unnamed `query_typed*` (ADR-0024), no raw `Client`; clippy/fmt/wasm32 clean. Plan §10 named
  all 12 tests; **added** `pg_member_store_onboarding_status_derivation`,
  `…_edit_recomputes_phone_lookup`, and `…_concurrent_regenerate_keeps_one_live` (the advisory-lock
  concurrency proof). See `DEFERRED.md` → "Server / store — `PgMemberStore` (spec 008 T07)".
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
- **Status:** ✅ DONE 2026-06-11 — the 6 admin paths (`GET/POST /api/admin/members`, `GET/PATCH
  /api/admin/members/{id}`, `POST …/regenerate-code`, `GET /api/admin/audit-log`) + 11 schemas
  (`MemberSummary`/`MemberList`/`MemberDetail`/`IssueMemberRequest`/`MemberIssued`/`DuplicatePhoneLink`/
  `EditMemberRequest`/`RegenerateCodeResponse`/`AuditEntry`/`AuditLog`/`OnboardingStatus`/`IssuableRole`)
  added **additively** to `api/openapi.yaml` (the `/api/auth/*` freeze + ADR-0023 tests stay green); the
  `adminSharedSecret` scheme + `AdminIdHeader` param model ADR-0026's trust boundary; PII handlers carry
  `x-requires-audit: true`. 4 new contract tests green (`openapi_pii_handlers_all_require_audit`,
  `member_summary_schema_has_no_tainted_field`, `admin_issuance_error_codes_in_registry`,
  `openapi_admin_surface_has_no_admin_creation_path`); `.bindings.lock` refreshed (87 inputs); allow-list
  clean (6 locks); web typecheck 0/0. **No new deps.** The TS client (`web/src/lib/server/members.ts`) +
  the Rust wire response DTOs + the `audited.rs` allowlist extension were deferred to their consumers
  (T10 / T09) — see `DEFERRED.md`.
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
- **Status:** ✅ DONE 2026-06-11 — `server/src/runtime/members.rs` (the 6 routes wired into the `Router`)
  composes the real core `MemberService` over the real `PgMemberStore` (P4), with the live `GetrandomRng`
  CSPRNG injected into `RngSecretSource` (ADR-0021), the KEK + Group key loaded/unwrapped **per request**
  (no DO cache — per-request unwrap minimizes the plaintext-key window, R2; deferred-with-rationale), and
  the **ADR-0026** shared-secret + `X-Admin-Id` fail-closed gate (`admin_guard`, constant-time, runs
  before any DB connect). Every admin response serializes through the sealed `admin_response_body` seam:
  new core wire DTOs (`MemberListView`/`MemberIssuedView`/`DuplicatePhoneLinkView`/`RegenerateCodeView`/
  `AuditLogView`) blessed `AuditedResponse` (T06 allowlist extended; `.stderr` golden re-blessed) — the
  Worker hand-rolls **no** member-PII JSON (I5). Proven by **6 miniflare tests over real PG18**
  (`server/test/admin-members.spec.ts`): issue→encrypt→store→audited-detail-decrypt round-trip, the I5
  audit row (`worker_detail_read_emits_audit`), regenerate, duplicate-phone link, no-submitted-PII-in-
  error-body (R10), and the 401-without-secret + `ADMIN_MEMBER_ROLE_FORBIDDEN` gate. Harness:
  `seed_worker_test_pg` Rust example (bootstraps the test Group's KEK-wrapped `delegated_keys`),
  `setup-worker-test-db.sh` (→ 11 migrations + seed), vitest `KEK`/`ADMIN_API_SECRET` bindings. Gate:
  `check-wrangler-credentials.sh` (+ meta-test) wired into CI. New worker deps `rand_core` 0.9.5 +
  `getrandom` 0.4.2 (`wasm_js`); `ADMIN_MEMBER_NOT_FOUND` registered (the 404 body code). All native +
  wasm + miniflare green. ADR-0026 authored. Deferred shells (real BFF call → T10; Secrets Store KEK;
  edit-into-duplicate clean mapping; live `emit()`; AC16 deployed-edge) in `DEFERRED.md` → T09.
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
| AC1 create member (roles[], created_by, RLS-scoped) | T05 (core), T07 (DB), T09 [shell] | T05 ✓; **T07 ✓ DB** (`…persists_member_with_roles_and_created_by`, RLS-scoped insert); **T09 ✓ shell** (`worker_issue_member_round_trip`: real Worker → encrypt → store → decrypt over real PG18) |
| AC2 address encrypted at rest (`i1_addresses_encrypted`) | T02 (crypto), T03 (column), T07 (DB) | T02·T03 ✓; **T07 ✓ DB** (`…address_encrypted_round_trip`: ciphertext≠plaintext, decrypt via Group key) |
| AC3 name encrypted at rest | T02, T03, T05, T07 | T02·T03 ✓; T05 ✓; **T07 ✓ DB** (name ciphertext round-trips through `name_encrypted bytea`) |
| AC4 phone two-fold (I3) | T05, T07 | T05 ✓; **T07 ✓ DB** (`…phone_two_fold`: lookup hash + ciphertext; `PgAuthStore::find_member_by_phone` then matches — issuance feeds sign-in) |
| AC5 mint one live Onboarding Code | T05 (decision), T07 (DB), T09 [shell] | T05 ✓; **T07 ✓ DB** (`…issue_is_atomic`: one member + one live code, one txn); **T09 ✓ shell** (issuance returns the show-once code over the live Worker) |
| AC6 regenerate atomic supersede-then-insert | T05, T07, T09 [shell] | T05 ✓; **T07 ✓ DB** (`…regenerate_supersede_then_insert_atomic` + `…concurrent_regenerate_keeps_one_live` advisory-lock proof); **T09 ✓ shell** (`worker_regenerate_code`: fresh code supersedes the prior) |
| AC7 PII reads audit-logged + `#[require_audit]` compile + OpenAPI coverage | T06 (compile), T05 (decision), T07 (DB), T08 (coverage), T09 [shell emit] | T05 ✓; T06 ✓ (compile gate); **T07 ✓ DB** (`pg_audit_store_writes_row_on_detail_read`: audit INSERT atomic with the ciphertext SELECT, I5/§7); **T08 ✓ OpenAPI-coverage** (`openapi_pii_handlers_all_require_audit`); **T09 ✓ shell live-emit** (`worker_detail_read_emits_audit`: a real detail read writes the `audit_log` row over real PG, names only) |
| AC8 `MemberSummary` no tainted type | T05 (compile assert), T08 (schema) | T05 ✓ (compile assert + no-PII prop); **T08 ✓ contract** (`member_summary_schema_has_no_tainted_field`: schema has name/roles/status, no phone/address) |
| AC9 read audit log (names not values) | T03 (shape), T05, T07, T08 | T03 ✓; T05 ✓; **T07 ✓ DB** (`pg_audit_store_read_returns_no_pii`: `fields` are names, no PII value persisted/returned); **T08 ✓ contract** (`AuditEntry.fields` enum `[name,phone,address]` — names only) |
| AC10 no admin-creation affordance (I11) | T05 (role reject), T08 (no path), T09 (HTTP reject), T10 [shell UI] | T05 ✓ (Admin unrepresentable at issuance); **T08 ✓ no-path** (`openapi_admin_surface_has_no_admin_creation_path`); **T09 ✓ HTTP** (the issuance route rejects `roles:[admin]` → `ADMIN_MEMBER_ROLE_FORBIDDEN` 400, worker test); T10 shell UI pending |
| AC11 edit re-encrypts + recompute hash + optimistic concurrency | T05 (decision), T07 (DB) | T05 ✓; **T07 ✓ DB** (`…optimistic_concurrency_stale_reject` whole-second token + `…edit_recomputes_phone_lookup`) |
| AC12 Group bootstrap + per-Group key, fail-closed | T02, T03, T04, T07 | T02·T03·T04 ✓; T05 ✓; **T07 ✓ DB** (`pg_delegated_key_store_persists_only_wrapped`: wrapped-only, `None`→fail-closed) |
| AC13 roles[] at issuance, swap out of scope | T05, T07 | T05 ✓; **T07 ✓ DB** (`…roles_array_round_trip`: multi-role set through `member_role[]`) |
| AC14 a11y (WCAG 2.2 AA, axe, dialogs/menus) | T10 [shell] | pending |
| AC15 i18n + pseudo-locale | T10, T01-catalog | pending |
| AC16 cross-tenant deployed-edge proof (F5) | T11 [shell], T07 (host precursor) | **T07 ✓ host precursor** (`rls_isolates_member_reads_by_tenant` + `prop_rls_isolates_random_two_group_configs`); T11 live deployed-edge proof pending |

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
