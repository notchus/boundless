# 008 — Admin member-management (issuance): Technical Plan

> Plan status: Drafted (ready for `/speckit.tasks`) — 2026-06-10
> Author: notch
> Derived from: `specs/008-admin-member-management/spec.md` (Clarified, 16 ACs) + a four-lens
> plan panel (architect → platform-parity + security-auditor + test-strategist).
> Highest authority: `.specify/memory/constitution.md`. This plan obeys it (P6).

---

## 0. How this plan was produced

A `/speckit.plan` panel ran read-only against the clarified spec, the constitution, the ADRs, and
the as-built code:

- **architect** — the constitution-gated backbone: where each piece lives, data model, API surface,
  task sequencing, open decisions.
- **platform-parity** — the shared-contract surface (verdict: admin-web/TS-only; the real hazard is
  core↔wire projection drift, the same seam as the T10 `ManifestPointer` miss).
- **security-auditor** — a 22-finding risk register (8 critical) against I1/I3/I5/I11/P2 and ADR-0025.
- **test-strategist** — the AC→test map, property tests, and the per-slice implement-gate contract.

Where the architect and the security-auditor disagreed (the `#[require_audit]` mechanism), this plan
resolves it in favour of the constitution invariant (I5) — see §7 and §14.

---

## 1. Constitution gate check (P6)

| Principle | How this plan obeys it | Enforcing artifact |
|---|---|---|
| **P1 — Accessibility is the product** | Admin member screens (list, add/edit dialogs, detail, audit-log, menus) meet WCAG 2.2 AA: keyboard-complete, visible focus, `aria-live` on validation/audit states, 400% reflow, dark + RTL. Accessible primitives (melt-ui — §13.8) give focus-trapped dialogs/menus. | `@axe-core/playwright` 4.11.3 zero-violation per route × {default,dark,RTL}; keyboard-ceremony + reflow Playwright tests (mirrors spec-001 T15). **AC14** |
| **P2 — No PII in logs. Ever.** | The product's hottest PII write path (name + address + phone + the per-Group key). All plaintext flows through tainted newtypes (`Address`, `PhoneNumber`, new `MemberName`) with no `Debug`/`Display`/`Serialize`; `GroupKey`/`Kek` are unloggable **and zeroized**; issuance logging routes through the scrubbed `emit()` sink (or ships none). | `static_assertions::assert_not_impl_any!`; `core/logging` red-team replay + a member-issuance fixture; no-substring-of-submitted-PII Worker error-response test. **AC4/AC8; R10** |
| **P4 — Rust core is source of truth** | Validation, `normalize_phone` (exists, `core/server/src/phone.rs`), phone hash/encrypt, name/address secretbox encrypt/decrypt, per-Group key generation, and code minting all live in `core/crypto` + `core/server`. SvelteKit is a thin shell over the Worker, which composes the core. | New `core/server` `MemberService`; new `core/crypto` `secretbox`; reviewer grep that no normalize/hash/encrypt logic appears in `web/`. **§3; R17** |
| **P5 — Spec before code** | Spec exists + passed `/clarify` (2026-06-10). This plan precedes tasks. | Spec link in PR; `tasks.md` is the contract. |
| **P7 — Native UI on every platform** | Admin = SvelteKit (the only client surface this spec touches; no iOS/Android/watch). | `web/` routes + axe CI. **AC14** |
| **P8 — i18n not afterthought** | Every new admin string ships from the catalog (17 keys in the spec table); pseudo-locale renders without truncation. | `web/src/lib/i18n` additions; `i18n-validator`; `zz-ZZ` render test. **AC15** |
| **P9 — Privacy invariants are testable** | I1, I3 (write side), I5 first acquire enforcing code here; each ships with its test in the same PR. | `core/crypto/tests/invariants.rs::i1_addresses_encrypted` (NEW — the file I1 names does not exist yet); the I5 audit-trail integration test (AC7); the cross-tenant proof (AC16). |
| **P11 — Free, open** | melt-ui / Radix Svelte / TanStack Table+Query / zeroize / trybuild are all MIT/OSS; dryoc (MIT) is already a dep. | `scripts/check-network-allowlist.sh` over the grown `web/pnpm-lock.yaml`. |
| **P12 — Operability** | New stable error codes (`ADMIN_MEMBER_DUPLICATE_PHONE`, `ADMIN_MEMBER_EDIT_STALE`, `ADMIN_MEMBER_PHONE_INVALID`, `ADMIN_MEMBER_ADDRESS_INVALID`, `ADMIN_GROUP_KEY_MISSING`) in `docs/error-codes.md`; a **key-management runbook** (`docs/runbooks/key-management.md`) for per-Group key gen / KEK access / rotation (ADR-0025); OTel + PII-free structured logging on issuance transitions. | `docs/error-codes.md` additions (reviewer flags uncoded variants); the runbook. |

**Tensions, and how they resolve (no silent reconciliation):**

1. **P4 vs. the admin-web TS surface.** Admin auth verification already runs in edge-TS (ADR-0017,
   a WebAuthn-only carve-out). This spec's PII crypto + validation must **not** widen that carve-out:
   the SvelteKit routes call the **Rust Worker** (`/api/admin/members/*`) which composes the core; they
   own only session/cookie + presentation (architecture §4: "server routes hit the same Workers API as
   the apps — no separate admin API"). **Resolved:** carve-out stays exactly as wide as ADR-0017 made it.
2. **ADR-0025 "server can decrypt" vs. "Unbreachable by Design."** I1 at-rest encryption defends a
   *storage* breach, **not** a compromised server tier — by design (matching must decrypt into a
   `MatchingContext`; admins read PII through audited endpoints). **Resolved/stated:** see §2. The plan
   states this explicitly so a reader never mistakes I1 for E2E (only I9's live tracker is E2E).
3. **`#[require_audit]` mechanism (architect Open-Decision-5 vs. security R4).** I5 literally mandates a
   *compile-time* check. The architect proposed weakening it to a sealed-trait + integration test only;
   the auditor rejected that. **Resolved (no ADR needed):** a compile-time guarantee is **required** (a
   sealed-trait whose omission is a *compile error* satisfies I5's intent; a literal `#[require_audit]`
   proc-macro is the stretch), proven by a `trybuild` compile-fail test, with the OpenAPI-coverage
   integration test as a *second* layer. Dropping to integration-test-only **would** require an ADR to
   weaken I5 — out of scope. See §7 and §14.

---

## 2. Threat model & scope statement (ADR-0025)

State this in `plan.md` and the key-management runbook so it is never silently assumed (security R18):

- **I1 field encryption defends against a storage breach** (stolen DB, backups, a `delegated_keys` row)
  — ciphertext is useless without the KEK→Group key. It does **not** make PII opaque to the server: the
  server holds the KEK (Secrets Store) → unwraps the Group key → can decrypt, **by design**, because
  (a) matching must decrypt addresses into an ephemeral `MatchingContext` and drop them (P3/I2, a later
  spec) and (b) Admins read PII through audited endpoints (I5).
- The "Unbreachable by Design" promise is satisfied by the *composition*: at-rest field encryption (I1)
  + audited reads (I5) + plaintext dropped after matching (P3/I2) + the true-E2E Optional Live Tracker
  (I9, a separate mechanism, out of scope here).
- **Residual risk (recorded, ADR-0025):** Group-key rotation is deferred, so one long-lived key protects
  the whole PII corpus. This is acceptable for v1 **only if** the nonce discipline (§5, R1) and the
  zeroize discipline (§5, R2) are solid — those are the realistic compromise vectors. KEK rotation
  (cheap re-wrap, `kek_version` bookkeeping) ships as a runbook procedure; Group-key rotation
  (re-encrypt Workflow) is documented-but-unbuilt.

---

## 3. Where each piece lives (functional-core / imperative-shell)

| Capability | Crate / module / file | Pure core slice (host-testable) | Deployable shell (real infra) |
|---|---|---|---|
| **secretbox encrypt/decrypt** | `core/crypto/src/secretbox.rs` (NEW) | `encrypt_field(plaintext, &GroupKey, &Nonce) -> Vec<u8>` storing `nonce ‖ ciphertext`; `decrypt_field(bytes, &GroupKey) -> Result`. dryoc `crypto_secretbox` — API/nonce-length pinned via docs-researcher at T02. | — |
| **`GroupKey` / `Kek` + KEK wrap/unwrap** | `core/crypto/src/secretbox.rs` | `wrap_group_key(&GroupKey,&Kek)`, `unwrap_group_key(bytes,&Kek)`; both newtypes have **no** `Debug`/`Display`/`Serialize` **and** `impl Drop` zeroize (R2). | Worker loads KEK from the **Secrets Store** binding (R3); caches the unwrapped `GroupKey` in the `GroupHub` DO memory only. |
| **Per-Group key generation @ bootstrap** | `core/server` decision; randomness injected | The mint→wrap→row-shape *decision*; the 32 random key bytes + the per-field nonce come from the **injected CSPRNG** (extend `SecretSource`/`RngSecretSource`). | Operator-run provisioning (extend `scripts/provision-neon.sh`) writes the `groups` + `delegated_keys` rows (§13.4). |
| **Phone hash + encrypt** | `core/crypto/src/hashing.rs` (hash EXISTS) + `secretbox.rs` (encrypt NEW) | `phone_lookup_hash` (EXISTS); `phone_encrypted = encrypt_field(phone,&GroupKey,nonce)`. `normalize_phone` EXISTS. | — |
| **Name / address encrypt + tainted types** | `core/domain/src/tainted.rs` + `secretbox.rs` | NEW `Address`, `MemberName` tainted newtypes (`tainted_secret!` macro exists); `Address::from_db(bytes,&GroupKey)` is I1's decrypt shape. | — |
| **Member create/edit orchestration** | `core/server/src/member.rs` (NEW) — `MemberService`, analog of `AuthService` | Validate→hash+encrypt→compose the write set + code mint behind new `MemberStore`/`AuditStore`/`DelegatedKeyStore` ports; optimistic-concurrency *decision* on `updated_at`. | `server/store` `PgMemberStore`/`PgAuditStore`/`PgDelegatedKeyStore` + Worker `/api/admin/members`. |
| **`MemberSummary` vs `MemberDetail` projections** | `core/server/src/member.rs` | `MemberSummary = { member_id, name: String, roles: Vec<Role>, onboarding_status }` — **no tainted type** (AC8, compile-asserted). **Core** `MemberDetail` carries tainted `Address`/`PhoneNumber` → cannot derive `Serialize` (P2, by construction). | The **wire** `MemberDetail` is a *separate* serializable DTO of plain `String` fields the Worker builds via `expose_secret()` at the audited boundary (parity R1 — the two-type split). |
| **`#[require_audit]` + audit write** | `core/server` (decision) + `server/store` `PgAuditStore` | The which-fields-are-PII *decision* + `AuditEntry { ts, admin_id, member_id, fields, request_id }` where `fields` is an **enum/`&'static str`** (names, never values — R6). The compile gate (§7). | Worker emits the audit row **in the same txn** as the PII read (R5); `audit_log` migration. |
| **Onboarding-Code mint/regenerate** | `core/auth` lifecycle (`evaluate_onboarding_code` EXISTS) + `core/server` mint + `core/crypto` `onboarding_code_hash` (EXISTS) | Mint *decision* + at-rest hash; regenerate = supersede-then-insert *contract*; code value from injected `SecretSource` (add `fresh_onboarding_code`). | `PgMemberStore` atomic supersede-then-insert vs `onboarding_codes` (table + the `one_live_per_member` partial-unique index EXIST). |
| **Admin web screens** | `web/src/routes/(admin)/members/*` (NEW) + `web/src/lib/server/members.ts` (thin Worker client) | — | List (search+filter, TanStack), add/edit dialogs (melt-ui), detail (audited read), audit-log view, regenerate. Behind the existing admin session. |

---

## 4. Data model & migrations

New reversible `NNNN_*.{up,down}.sql` continuing 0001–0008. LF endings; the runner wraps each file in
its own txn; **ENABLE + FORCE RLS** on every PII table (FORCE closes the table-owner bypass —
`server/store/src/lib.rs:178`); group scoping via the existing `current_group_id()` resolver (unset →
NULL → deny, fail-closed).

### `0009_delegated_keys.{up,down}.sql` (ADR-0025)
```
delegated_keys (
  group_id     uuid PRIMARY KEY REFERENCES groups(id) ON DELETE CASCADE,
  wrapped_key  bytea NOT NULL,            -- Group secretbox key, KEK-wrapped (nonce ‖ ciphertext); NEVER plaintext
  kek_version  integer NOT NULL DEFAULT 1,-- so a KEK re-wrap rotation is traceable
  created_at   timestamptz NOT NULL DEFAULT now(),
  updated_at   timestamptz NOT NULL DEFAULT now(),
  created_by   uuid
)
```
ENABLE + FORCE RLS, `USING/WITH CHECK (group_id = current_group_id())`; `set_updated_at` trigger.

### `0010_member_pii.{up,down}.sql` — **decided: columns on `members`, not a separate table** (Open-Decision-1a)
`ALTER TABLE members ADD COLUMN name_encrypted bytea, ADD COLUMN address_encrypted bytea;` (both
nullable — Admins have no address; existing rows backfill NULL). Each `bytea` holds `nonce ‖ ciphertext`.
Rationale: same RLS surface, phone PII already lives on `members`, I12 sweeps one row (ADR-0006 rejected
separate entities for the same reason). `down` drops the two columns.

### `0011_audit_log.{up,down}.sql` (I5)
```
audit_log (
  id          uuid PRIMARY KEY,     -- T03 resolution: NO column DEFAULT; minted via gen_random_uuid() inline at the audit INSERT (T07), matching every other table's store convention (server/store/src/lib.rs) — keeps the schema uniform + the core randomness-free (ADR-0021)
  group_id    uuid NOT NULL,
  admin_id    uuid NOT NULL,        -- the actor (I5)
  member_id   uuid NOT NULL,        -- whose PII was read
  fields      text[] NOT NULL,      -- FIELD NAMES, never values (AC9) e.g. {'address','phone'}
  request_id  text NOT NULL,
  created_at  timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (member_id, group_id) REFERENCES members(id, group_id) ON DELETE CASCADE
)
CREATE INDEX audit_log_member_idx ON audit_log (member_id);
```
**No `_encrypted` column by design** (nothing here is PII). ENABLE + FORCE RLS, **group-scoped**
(Open-Decision-3a — uniform with every table; AC16's cross-tenant proof then covers this table too).
**Append-only:** no `updated_at`/`set_updated_at` trigger — note this convention divergence in the
migration header.

### Cross-cutting DB discipline (spec-001 carry-forward)
- All Worker-path queries use the unnamed `query_typed*`/`execute_typed` family (ADR-0024).
- Every store method opens `begin()` (RLS `set_config(..., true)`); **no raw-`Client` accessor** on any
  new store (R7 — the pooled-connection trap).
- `ensure_least_privilege` (rejects superuser/`BYPASSRLS`/`REPLICATION`) is called at boot before any
  store is constructed; the new `build_service` analog must invoke it.
- Member create + code mint is **one txn** (R13); regenerate is supersede-then-insert in one txn against
  the `onboarding_codes_one_live_per_member` partial-unique index (the proven `consume_and_rotate_recovery`
  pattern) — `pg_advisory_xact_lock` per member if concurrent regenerate is reachable.
- `members.id` is `(id, group_id)` referenceable (already the case for the audit FK).

---

## 5. Crypto design (ADR-0025) — load-bearing

This is the section the security panel weighted most. Three controls are make-or-break:

**R1 — random, injected, single-source nonce (wired THIS spec).** XSalsa20-Poly1305 (`crypto_secretbox`)
nonce reuse is catastrophic (leaks plaintext XOR + enables forgery), and the Group key is long-lived
(rotation deferred), so the whole PII corpus shares one key — nonce uniqueness is the only guard.
Therefore:
- Exactly **one** `encrypt_field(plaintext, &GroupKey, nonce: &Nonce)` entry point; the caller cannot
  hand-roll or omit the nonce, and there is no nonce-less overload.
- The nonce is a **fresh random draw from the injected CSPRNG** at the Worker boundary (extend
  `RngSecretSource` with `fresh_nonce()` — no ambient randomness in core, ADR-0021). **This CSPRNG must
  be wired in this spec, not deferred** — issuance cannot encrypt without it (today `PlaceholderSecrets`
  is `unreachable!` for every mint; there is no live wasm-reachable CSPRNG yet).
- **Forbid a counter/deterministic nonce** — a pooled, multi-isolate Worker fleet has no shared counter.
- Nonce **length** comes from dryoc's `crypto_secretbox` constant via docs-researcher (XSalsa20 ⇒ 24
  bytes, but pin it, never assume).

**R2 — zeroize the keys.** `GroupKey`/`Kek`: no `Debug`/`Display`/`Serialize` (compile-asserted, like
`HmacKey`) **and** `impl Drop` with `zeroize` (a new dep — pin from lock at T02). Unlike `HmacKey`
(loaded once, process-lifetime), the Group key is unwrapped per-DO-init and the threat is a DO memory
snapshot, so zeroize is load-bearing here, not GA hardening. Evict the cached plaintext key on DO
eviction; never persist it (only the wrapped blob persists in `delegated_keys`).

**R3 — KEK from Secrets Store, never `env.var`.** The existing `load_hmac_key` reads `env.var("HMAC_KEY")`
(a `[vars]`/`secret` plaintext binding) — the KEK must **not** follow that pattern. It uses the Secrets
Store binding API (`env.SECRETS.get("KEK")` / the workers-rs 0.8.3 equivalent — pin via docs-researcher)
because it is the root of the whole PII envelope. Boot fails closed if the KEK is absent. Extend the
`wrangler.toml` committed-credential grep gate to assert no `KEK`/`GROUP_KEY` value ever lands in
`wrangler.toml`/`[vars]`.

**R11/R12 — fail-closed + the named test.** `encrypt_field` requires `&GroupKey` by type, so "encrypt
without a key" is unrepresentable; the orchestration loads+unwraps the key first, returns
`ADMIN_GROUP_KEY_MISSING` on failure, and never reaches the member INSERT (no `unwrap()` on the key load).
`core/crypto/tests/invariants.rs::i1_addresses_encrypted` ships in the same task as `secretbox.rs` (P9):
ciphertext≠plaintext; stored blob is `nonce ‖ ciphertext`; wrong key ⇒ `Err` (Poly1305 tag), not garbage;
tamper a byte ⇒ `Err`.

---

## 6. API surface (OpenAPI 3.1 — additive)

`/api/auth/*` stays **frozen**; spec 008 **adds** `/api/admin/*` (additive — the freeze test checks auth
shapes are unchanged). `api/boundless.proto` is **untouched** (HTTP admin CRUD, no WebSocket concern —
parity confirmed).

New paths (sketch — finalized at T08):
- `GET /api/admin/members` → `MemberList { members: MemberSummary[] }`, `?search=&role=&status=`. **No
  tainted PII → not an audited read.**
- `POST /api/admin/members` → `IssueMemberRequest { name, phone, address, roles[] }` →
  `IssueMemberResponse { member: MemberSummary, onboarding_code, code_expires_at }`. `onboarding_code` is
  a sensitive TLS-only show-once field (modeled like the existing optional-sensitive `access_token`).
  Inbound raw PII (TLS) — the Worker keeps `name`/`address`/`phone` off the log path (R10).
- `GET /api/admin/members/{id}` → **wire** `MemberDetail` (plain-string `address`/`phone`) — carries PII
  → `#[require_audit]` + emits an `AuditEntry`. **AC7**
- `PATCH /api/admin/members/{id}` → `EditMemberRequest { …, expected_updated_at }` (optimistic
  concurrency) → wire `MemberDetail` (audited). Stale ⇒ `ADMIN_MEMBER_EDIT_STALE`. **AC11**
- `POST /api/admin/members/{id}/regenerate-code` → `{ onboarding_code, code_expires_at }`. **AC6**
- `GET /api/admin/audit-log` → `{ entries: AuditEntry[] }`, `fields: string[]` are **names only** (AC9) →
  not a recursive audited read.

**Parity must-fixes baked into T08 (R1–R5 of the parity lens):**
- **Two-type `MemberDetail`** — the core (tainted, no `Serialize`) and the wire (serializable, plain
  strings) are distinct types; the Worker converts via `expose_secret()` at the explicit audited
  boundary. This is the single highest-likelihood spot for a P2 leak or a drift — written into T05/T09.
- **`OnboardingStatus`** is a brand-new enum — define it **once** as a named OpenAPI schema mirroring one
  core enum (values/order/casing pinned in both).
- **Audited-field vocabulary** (`"address"`, `"phone"`, `"name"`) is a single Rust source the audit
  decision uses, so what `MemberDetail` exposes and what `audit_log.fields` records cannot diverge.
- Reuse `Role` by `$ref` (never re-inline `[rider,driver,admin]`); `member_id` is `{type:string,
  format:uuid}` serializing the `#[serde(transparent)]` `MemberId` (copy the proven `DeviceBound`
  convention).
- **Regenerate `api/.bindings.lock`** on every contract/core-touching slice (T02/T03/T05/T08 and any
  `Cargo.lock` change) — the drift gate hashes `api/openapi.yaml` + all `core/**` and fails closed until
  the lock is committed. (The architect's backbone omitted this — it is now an explicit step.)
- **TS client provenance (decide, don't leave "or"):** generate via `openapi-typescript` and record the
  `[outputs]` in the lock, **or** hand-roll a typed `web/src/lib/server/members.ts` *derived from*
  `api/openapi.yaml`. Either is fine for a thin marshalling layer; an independently-authored client that
  drifts is the P4 duplicate failure. **Recommendation:** hand-rolled-but-derived for v1 (TS codegen is
  still scaffold per the T10-shell register), revisit when codegen is wired.

Register the new error codes in `docs/error-codes.md` in the same PR (P12).

---

## 7. Audit enforcement (I5) — the compile gate

I5 is non-negotiable and literal ("compile-time check… or fail the build"). The plan:
- **A function whose return type transitively contains a tainted type cannot be wired into the router
  without producing an `AuditEntry`** — enforced by the **type system** (a sealed `AuditedResponse` bound
  on router registration, or a literal `#[require_audit]` proc-macro), **not** by CI alone. Omitting it is
  a *compile error*. Proven by `trybuild` (`require_audit_compile_fail`). The OpenAPI-PII-handler-coverage
  integration test (`openapi_pii_handlers_all_require_audit`, AC7) is a **second** layer.
- **Atomic with the read (R5):** the `audit_log` INSERT and the member-row SELECT (the decrypt source)
  run in **one** tenant-scoped txn, committed together. Order: open txn → set RLS GUC → SELECT ciphertext
  → INSERT audit row → COMMIT → *then* decrypt in core + serialize. A failed audit INSERT rolls back the
  read (admin gets a 500, never PII-without-audit); a decrypt panic after commit leaves at most a benign
  spurious audit row, never a missing one.
- **`fields` is names-only (R6):** `AuditEntry.fields` is `Vec<&'static str>` / an `AuditField` enum, so a
  field *value* is type-impossible to store; the audit-log read response is compile-asserted to carry no
  tainted type (same as `MemberSummary`).

If the team genuinely cannot achieve a compile gate, that is an **ADR to weaken I5** — not a plan/tasks
decision (§14).

---

## 8. RLS / tenant isolation (R7, AC16)

- Each new store (`PgMemberStore`, `PgAuditStore`, `PgDelegatedKeyStore`) copies the `PgAuthStore`
  discipline **exactly**: private `client`, **no raw accessor**, every method opens `begin()` (RLS
  `set_config(app.current_group_id, $1, true)`), fail-closed.
- `delegated_keys` + `audit_log` get ENABLE + **FORCE** RLS (FORCE matters if the app role owns the table).
- The Group-**bootstrap** write (the first `groups` + `delegated_keys` rows) is the one legitimate
  unscoped path — it is **developer-gated and operator-run** (a `provision-neon.sh` extension, §13.4),
  **not** a method on a tenant-scoped store, and `WITH CHECK`s exactly the install's group.
- `ensure_least_privilege` is reused as the live precondition (already rejects superuser/`BYPASSRLS`/
  `REPLICATION`).
- **AC16 (the F5 closer):** with ≥2 seeded Groups, as the real locked-down `boundless_app` role on the
  **deployed edge**, a Group-A admin token cannot list/read/edit Group-B members. This spec is the first
  that *can* run it (it produces the ≥2 Groups). Host precursor now (`rls_isolates_member_reads_by_tenant`
  + a 2-group proptest on PG18); the **live** proof is deployable-shell-only (gated on the operator's
  deploy).

---

## 9. Per-platform work breakdown

**(a) Rust core** — `core/domain` tainted `Address`/`MemberName` + `assert_not_impl_any!`; `core/crypto`
`secretbox.rs` (`GroupKey`/`Kek`, encrypt/decrypt, wrap/unwrap, zeroize) + `tests/invariants.rs`;
`core/server` `member.rs` `MemberService` + `MemberStore`/`AuditStore`/`DelegatedKeyStore` ports +
projections + audit decision + the bootstrap orchestration + `SecretSource::fresh_onboarding_code`/
`fresh_nonce`; the `#[require_audit]` compile guard.

**(b) server/store + Worker** — `PgMemberStore`/`PgAuditStore`/`PgDelegatedKeyStore` (`query_typed*`, all
via `begin()`); `server/src/runtime/members.rs` route module wired into the `Router`, loads + caches the
unwrapped `GroupKey` (KEK from the Secrets Store binding); new `KEK` Secrets-Store binding in
`server/wrangler.toml`; the bootstrap provisioning extension + the key-management runbook.

**(c) SvelteKit admin UI** (behind the existing admin session) — member list (search+filter via TanStack
Table/Query), add/edit dialogs + member menu (melt-ui), member detail (audited read), audit-log view,
regenerate-code; the 17 i18n keys. **A11y obligations (AC14):** axe zero-violations per route ×
{default,dark,RTL}; keyboard-complete dialogs/menus (focus trap, Esc, focus return); `aria-live` on
validation + audit states; `<label for>`/`aria-describedby`; 400% reflow; visible focus; skip links.
Tooling already pinned (`@axe-core/playwright` 4.11.3, `@playwright/test` 1.60.0). **Web dep versions
(melt-ui/Radix, TanStack) are pinned from `web/pnpm-lock.yaml` via docs-researcher at T10 — never
invented.**

---

## 10. Dependency-ordered task sequencing (T01–T11)

Each slice is functional-core-first with its deferred shell (the spec-001 T07-shell-A/B discipline).
`/speckit.tasks` formalizes this into `tasks.md`.

| # | Slice | Closes | Gating tests (must be green) | Notes |
|---|---|---|---|---|
| **T01** | ADRs/docs/error-codes wiring | P12 scaffolding | `core/server` error-code parity extended; `docs/error-codes.md` + `docs/runbooks/key-management.md` stub; `docs/stack-matrix.md` dryoc→secretbox resolution; DEFERRED.md spec-008 items repointed to ADR-0025 | ADR-0006/0025 already authored. Docs only. |
| **T02** | `core/crypto` secretbox + `GroupKey`/`Kek` (zeroize) + tainted `Address`/`MemberName` + injected `fresh_nonce` | AC2/AC3 foundation; R1/R2/R12 | `i1_addresses_encrypted`, `i1_name_encrypted`; `assert_not_impl_any!` (Address/MemberName/GroupKey/Kek); `prop_secretbox_round_trip_and_ciphertext_differs`, `prop_secretbox_nonce_unique_across_calls`, `prop_decrypt_wrong_key_fails`, `prop_kek_wrap_unwrap_round_trips`; `cargo build --target wasm32 -p boundless-crypto` | dryoc `crypto_secretbox` API + nonce len pinned via docs-researcher first; pin `zeroize` from lock. **Regenerate `.bindings.lock`.** |
| **T03** | `0009_delegated_keys` + `0010_member_pii` + `0011_audit_log` migrations | AC12(structure)/AC2/AC3(columns)/AC9(shape) | `server/tests/migrations.rs` (versions→1..=11; bytea columns; `audit_log.fields text[]`, no `_encrypted`, append-only divergence; all three ENABLE+FORCE RLS); `scripts/test-migrations.sh` live apply/RLS/revert on PG18 | **Regenerate `.bindings.lock` if core touched.** |
| **T04** | Group bootstrap + key generation (core decision + injected RNG) | AC12 | `bootstrap_generates_wrapped_key_from_injected_seed`; `member_service_issuance_fails_closed_without_group_key` (`ADMIN_GROUP_KEY_MISSING`) | |
| **T05** | `core/server` `MemberService` + ports + projections + audit decision | AC1/AC4/AC8/AC11(decision)/AC13; R6/R8/R11/R13/R16 | `member_service_issues_rider_and_driver`, `member_service_accepts_multi_role_set`, `member_service_edit_reencrypts_and_recomputes_phone_hash`, `member_service_stale_edit_rejected`, `member_service_rejects_admin_role_on_issuance`, `member_service_mints_one_live_onboarding_code`, `member_summary_holds_no_tainted_type`, `member_list_emits_no_audit_event`, `audit_entry_carries_field_names_ts_admin_member_request`; `prop_member_summary_never_carries_pii`, `prop_every_pii_detail_read_emits_audit`, `prop_phone_change_recomputes_matching_hash` | Two-type `MemberDetail` split lives here. **Regenerate `.bindings.lock`.** |
| **T06** | `#[require_audit]` compile enforcement | AC7(compile leg); R4 | `require_audit_compile_fail`, `member_summary_rejects_tainted_field` (trybuild) | Pin `trybuild` from lock. Compile gate is mandatory (§7/§14). |
| **T07** | `PgMemberStore`/`PgAuditStore`/`PgDelegatedKeyStore` (real PG18) | AC1/AC2/AC4/AC6/AC9/AC11/AC12/AC13(DB legs); R5/R7/R13/R14 | `pg_member_store_persists_member_with_roles_and_created_by`, `…_address_encrypted_round_trip`, `…_phone_two_fold`, `…_roles_array_round_trip`, `…_regenerate_supersede_then_insert_atomic`, `…_optimistic_concurrency_stale_reject`, `…_issue_is_atomic`, `pg_audit_store_writes_row_on_detail_read`, `…_read_returns_no_pii`, `pg_delegated_key_store_persists_only_wrapped`, `rls_isolates_member_reads_by_tenant`, `prop_rls_isolates_random_two_group_configs` | All via `query_typed*`; all via `begin()`; no raw client. |
| **T08** | OpenAPI `/api/admin/*` freeze + contract test | AC7(coverage leg)/AC9/AC10 | `openapi_pii_handlers_all_require_audit`, `member_summary_schema_has_no_tainted_field`, `admin_issuance_error_codes_in_registry`, `openapi_admin_surface_has_no_admin_creation_path`; `/api/auth/*` freeze still green | Two-type split, `OnboardingStatus`, audited-field vocab, `Role` `$ref`. **Regenerate `.bindings.lock`.** Decide TS provenance. |
| **T09** | Worker endpoints + KEK binding + GroupKey cache | AC1/AC5/AC6/AC10(HTTP legs); R3/R10 | `worker_issue_member_round_trip`, `worker_detail_read_emits_audit`, `worker_regenerate_code`, `worker_duplicate_phone_links_existing`, `worker_error_response_contains_no_submitted_pii` (miniflare + local PG18) | KEK from Secrets Store; live CSPRNG wired here. |
| **T10** | SvelteKit admin UI + i18n + a11y | AC14/AC15 + client legs of AC1/AC6/AC9 | `members_routes_axe_clean_default_dark_rtl`, `members_add_edit_dialog_keyboard_ceremony`, `members_list_reflows_at_400_percent`, `audit_log_validation_aria_live`, `members_ui_offers_no_create_admin_action`, `members_pseudo_locale_renders_without_truncation`, `admin_members_catalog_parity`; allow-list clean over grown `web/pnpm-lock.yaml`; `i18n-validator` | Pin melt-ui/TanStack from lock. |
| **T11** | Cross-tenant deployed-edge proof | AC16 | `cross_tenant_admin_cannot_read_other_group` on the deployed edge as `boundless_app` with ≥2 Groups (`scripts/smoke-deployed-edge.sh` extension) | **Deployable-shell-only** — gated on the operator's deploy; AC16 = host-precursor-covered until live. |

**Deferred shells** (recorded in DEFERRED.md): the live `boundless::logging::emit()` sink + the
member-issuance I10 fixture (T07-shell-B track); `PgDeviceStore` token encryption (now *unblocked* by
T02's secretbox key — push spec 007); the KEK-rotation re-wrap + Group-key re-encrypt Workflow
(runbook-documented, unbuilt — ADR-0025); geocoding/ETA (matching spec, architecture flow D).

---

## 11. Test strategy (the implement-gate contract)

**Levels** (no new CI jobs — extend `rust-core`, `server-store`, `worker`, `web`, `server-migrations`):
property (proptest, committed regression seeds — P9), host unit, `trybuild` compile-fail, real-PG18
integration, miniflare Worker, Vitest contract, Playwright+axe e2e, and the deployed-edge bash smoke.

**AC → test map (all 16; ✓ = functional-core/test-covered, not deployably shipped — spec-001 convention):**

| AC | Named/representative test(s) | Level | Host-now / shell-only |
|---|---|---|---|
| AC1 | `member_service_issues_rider_and_driver`; `pg_member_store_persists_member_with_roles_and_created_by`; `worker_issue_member_round_trip` | unit+integration+worker | host-now / shell(worker) |
| AC2 | **`i1_addresses_encrypted`**; `pg_member_store_address_encrypted_round_trip` | unit+integration | host-now |
| AC3 | `i1_name_encrypted`; `member_summary_name_is_plain_string_not_persisted`; migration bytea assert | unit+migration | host-now |
| AC4 | `member_service_stores_phone_hash_and_ciphertext`; `pg_member_store_phone_two_fold`; reuse `i3_phone_lookup_constant_time` | unit+integration | host-now |
| AC5 | `member_service_mints_one_live_onboarding_code`; `pg_member_store_mints_single_live_code` | unit+integration | host-now |
| AC6 | `member_service_regenerate_supersedes_decision`; **`pg_member_store_regenerate_supersede_then_insert_atomic`**; `worker_regenerate_code` | unit+integration+worker | host-now / shell(worker) |
| AC7 | `require_audit_compile_fail`; `audit_entry_carries_field_names_…`; `pg_audit_store_writes_row_on_detail_read`; **`openapi_pii_handlers_all_require_audit`**; `worker_detail_read_emits_audit` | compile+unit+integration+contract+worker | host-now / shell(worker live emit) |
| AC8 | **`member_summary_holds_no_tainted_type`**; `member_list_emits_no_audit_event`; logging red-team replay | unit+compile | host-now |
| AC9 | `audit_log_stores_field_names_not_values`; `pg_audit_store_read_returns_no_pii`; migration "no `_encrypted`" assert | unit+integration+migration | host-now |
| AC10 | `openapi_admin_surface_has_no_admin_creation_path`; `member_service_rejects_admin_role_on_issuance`; e2e `members_ui_offers_no_create_admin_action` | contract+unit+e2e | host-now / shell(e2e) |
| AC11 | `member_service_edit_reencrypts_and_recomputes_phone_hash`; `member_service_stale_edit_rejected`; **`pg_member_store_optimistic_concurrency_stale_reject`** | unit+integration | host-now |
| AC12 | `bootstrap_generates_wrapped_key_from_injected_seed`; `member_service_issuance_fails_closed_without_group_key`; `pg_delegated_key_store_persists_only_wrapped`; migration bytea assert | unit+integration+migration | host-now |
| AC13 | `member_service_accepts_multi_role_set`; `pg_member_store_roles_array_round_trip`; `member_service_edit_changes_role_set` | unit+integration | host-now |
| AC14 | `members_routes_axe_clean_default_dark_rtl`; `members_add_edit_dialog_keyboard_ceremony`; `members_list_reflows_at_400_percent`; `audit_log_validation_aria_live` | e2e | shell-only |
| AC15 | `admin_members_catalog_parity`; `members_pseudo_locale_renders_without_truncation`; `i18n-validator` | unit+e2e+subagent | host-now / shell(render) |
| AC16 | **`cross_tenant_admin_cannot_read_other_group`** (deployed edge); host precursor `rls_isolates_member_reads_by_tenant` | deployed-edge+integration | **shell-only** (host precursor now) |

**Property tests worth having:** `prop_secretbox_round_trip_and_ciphertext_differs`,
`prop_secretbox_nonce_unique_across_calls` (the footgun), `prop_decrypt_wrong_key_fails`,
`prop_kek_wrap_unwrap_round_trips`, `prop_member_summary_never_carries_pii`,
`prop_every_pii_detail_read_emits_audit`, `prop_phone_change_recomputes_matching_hash`,
`prop_rls_isolates_random_two_group_configs`. New `core/crypto/proptest-regressions/` needs a `.gitkeep`
so the existing `scripts/check-proptest-regressions.sh` auto-discovery stays green.

**Genuinely hard to test (honest mitigation):** AC16 live proof (host precursor now + `ensure_least_privilege`
reuse; live deferred to deploy with ≥2 Groups); KEK-from-Secrets-Store unwrap (host-test pure wrap/unwrap;
binding read miniflare-stubbed; real fetch shell-only); "plaintext key never anywhere" (proven by no-formatter
compile asserts + persists-only-wrapped + logging replay; the live `emit()` sink deferred); a11y beyond axe
(manual NVDA/VoiceOver + Lighthouse = pre-GA persona-acceptance checklist, not CI-gated); the
`#[require_audit]` literalness (trybuild proves whichever compile mechanism lands).

---

## 12. New dependencies (lock-pinned at implement time — never invented)

| Dep | Where | Used for | When pinned |
|---|---|---|---|
| `dryoc` `crypto_secretbox` | `core/crypto` (already a dep) | secretbox field encryption + KEK wrap | API/nonce-len via docs-researcher at T02 (ADR-0025 fixed the *class*, not signatures) |
| `zeroize` | `core/crypto` | `Drop`-zeroize `GroupKey`/`Kek` (R2) | T02, from lock |
| `trybuild` (dev) | `core/server` (or `core/macros`) | the I5 compile-fail gate (R4) | T06, from lock |
| `melt-ui` (recommended) / Radix Svelte | `web` | accessible dialogs/menus | T10, from `web/pnpm-lock.yaml` |
| TanStack Table + TanStack Query | `web` | member data-grid + server state | T10, from lock |

`docs/stack-matrix.md` is updated to match the lock as each lands (the parked "melt-ui/Radix + TanStack
for spec 008" rows get real versions); `scripts/check-network-allowlist.sh` re-runs clean across all
locks (the grown web tree); the wasm `getrandom`-gate CRATES list gains nothing new (the CSPRNG is
injected, core stays randomness-free).

---

## 13. Open decisions (resolved with a recommendation — flag to override)

All are doc-grounded and **do not block `/speckit.tasks`**; reversible at the named slice.

1. **Name/address: columns on `members`** (not a separate `member_pii` table). *Why:* one RLS surface,
   phone PII already there, one-row I12 sweep (ADR-0006 precedent). → adopted in §4.
2. **Decrypt in core, two-type `MemberDetail`** (core tainted + wire serializable via `expose_secret()`).
   *Why:* P4 single-source crypto + P2 (tainted can't `Serialize`). → §3/§6.
3. **`audit_log` group-scoped FORCE RLS.** *Why:* uniformity + AC16 coverage. → §4/§8.
4. **Group bootstrap = operator-run provisioning script + runbook** (not a `/api/dev/bootstrap` endpoint).
   *Why:* simpler, no new authed surface; a bootstrap endpoint would need the still-deferred developer
   hardware-key WebAuthn (R16). **KEK rotation cadence:** recommend the runbook default = annual KEK
   re-wrap + on-suspected-compromise (documented-but-unbuilt; ADR-0025). → §8/§13.4.
5. **`#[require_audit]` stays a compile gate** (sealed-trait acceptable; proc-macro is the stretch),
   proven by `trybuild`; integration test is the second layer. *Why:* I5 mandates it — weakening needs an
   ADR (§14). → §7.
6. **Register issuance error codes at T01** (append-only). → §10.
7. **Duplicate-phone disclosure is an admin-only, I5-audited trust boundary** — reviewer-enforced; never
   reused on `/api/auth/*` or `/api/rider/*` (R9). → §6.
8. **melt-ui** over Radix Svelte (stack-matrix names it first; Svelte-5-runes-native), version pinned from
   the lock at T10. → §9/§12.

**Genuinely your call (recommendations stand if you don't say otherwise):** #4's KEK-rotation cadence and
#8's primitive library are the two with real discretion; #5 is dictated by I5 and flagged in §14.

---

## 14. Conflict surfaced (per P6 — not silently reconciled)

**The `#[require_audit]` mechanism.** The architect's Open-Decision-5 recommended weakening I5's
enforcement to a sealed-trait + integration test, calling the literal proc-macro "heavy." The
security-auditor (R4) holds that I5 literally requires a *compile-time* check and that integration-test-
only is a regression that would let an un-audited PII read ship silently. **This plan sides with I5:** a
compile-time guarantee is required (a sealed-trait whose omission is a compile error satisfies I5's
intent; the literal `#[require_audit]` proc-macro is the stretch goal), proven by `trybuild`, with the
OpenAPI-coverage test as a second layer. **Dropping to integration-test-only would require an ADR to
weaken I5** and is out of scope. No other conflict with the constitution, an ADR, the glossary, or
`architecture.md` was found.

---

## 15. DEFERRED.md / docs updates this plan implies (land at T01 unless noted)

- `docs/stack-matrix.md`: resolve the dryoc "sealed-box/secretbox" hedge → **secretbox for field-level
  PII at rest** (sealed boxes reserved for I9); add the parked web rows' real versions at T10.
- `DEFERRED.md`: repoint the spec-008-tagged I1 items (per-Group key/KEK columns, sealed-box→secretbox) to
  ADR-0025; record the new deferred shells (live `emit()` issuance fixture; KEK/Group-key rotation
  Workflow; the I12 `forget_member` sweep must cover `name_encrypted`/`address_encrypted`/`audit_log`).
- `docs/error-codes.md`: the five new `ADMIN_…` codes.
- `docs/runbooks/key-management.md`: NEW — per-Group key gen, KEK access, KEK re-wrap rotation
  (`kek_version`), deferred Group-key re-encrypt procedure (P12, ADR-0025).
