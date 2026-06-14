# 009 — Deploy the admin web dashboard — Tasks

> Tasks status: Ready for `/speckit.implement`
> Spec: `specs/009-admin-web-deploy/spec.md` (Clarified, D1–D8) · Plan: `specs/009-admin-web-deploy/plan.md`
> One task = one PR-sized slice. Pick one, fresh session, `/speckit.implement`, `/compact` + end when done.

## How to read this

- **✓ in the AC tracker = functional-core / test-covered locally**, not deployably done. The **(edge)**
  ACs (AC9–AC11, and the live legs of AC5/AC14) close only on the operator's live `wrangler deploy` run
  (T13). Local proxy tests stand in for them until then (per the project convention).
- **Blockers** are hard serialization points. Tasks with no unmet blocker and marked **∥** can run in
  parallel.
- No task introduces behavior absent from `spec.md` (P6). Every task maps to ≥1 AC.

## AC-coverage tracker (updated as tasks land)

| AC | Closed by | Status |
|---|---|---|
| AC1 — member BFF fail-closed selector | T05 | ✓ |
| AC2 — KV session persist / TTL / revoke | T06 | ✓ |
| AC3 — passkey persists across cold start | T02 (store) · T07 (e2e ceremony) | ◐ T02 store leg done |
| AC4a — invite single-use + TTL + atomic consume | T02 (PG) · T04 (worker) | ✓ |
| AC4b — HMAC compare in core, prod store Worker-backed | T02 (core route) · T04 (worker assertion) · T05 (web) | ✓ all legs done |
| AC5 — no reachable `/api/test/*` in prod | T07 (build-artifact) · T13 (edge probe) | ◐ T07 build-artifact leg done (tree-shake proven under hostile NODE_ENV) |
| AC6 — `wrangler.toml` no secret/real-id in `[vars]` | T09 | ✓ |
| AC7 — operator seed (null-PII admin + invite, idempotent) | T10 | ✓ |
| AC8 — web never logs PII/secrets; token off both log paths | T04 (worker) · T08 (web) | ✓ both legs done |
| AC9 — build + deploy reachable | **T13 (edge)** | ☐ |
| AC10 — live full E2E | **T13 (edge)** | ☐ |
| AC11 — RP_ID/origin/Referrer-Policy | T09 (local rp-config) · T12 (Referrer-Policy) · **T13 (edge)** | ◐ T09 local env-driven rp-config leg done |
| AC12 — a11y/i18n unchanged-and-green | T07 (regression run) | ◐ store swap behavior-preserving in dev (152 unit green); axe ×variant e2e is CI-gated |
| AC13 — B1 contract-freeze + I5 negative gate | T03 | ✓ |
| AC14 — new endpoints RLS-scoped (cross-tenant) | T02 (PG) · T04 (miniflare) · **T13 (edge)** | ◐ T02 PG + T04 miniflare legs done |
| AC15 — wrangler-types/binding drift CI | T09 | ✓ |

---

## T01 — ADR: B1 persistence routing
**Does:** Author `docs/adr/00NN-admin-web-b1-persistence-routing.md` recording the decision (WebAuthn
invite/credential persistence behind server-to-server Worker endpoints; web tier = zero Postgres), **why
B1 over B2**, the route naming (NOT `/api/admin/auth/*`; recommend `/api/admin/webauthn/*`), the
pre-session invite-resolve carve-out (shared-secret, no `X-Admin-Id`), and that it refines ADR-0026
(extends the BFF surface) + ADR-0017 (store moves behind the Worker; ceremony+verdict stay edge-TS,
carve-out scope unchanged).
**Touches:** `docs/adr/`, `docs/adr/README` index if present.
**Closes:** (governance — unblocks AC13) · **Tests:** none (doc). · **Blockers:** none. **∥** no.
**Note:** finalize the route prefix here (plan §10 decision 1) — the contract task (T03) keys off it.
- **Status:** ✅ DONE 2026-06-13 — `docs/adr/0027-admin-web-b1-persistence-routing.md` (+ README index entry).
  **Finalized for T03:** route prefix = **`/api/admin/webauthn/*`** (not `/api/admin/auth/*`, the frozen
  browser ceremony); a combined atomic **`register-complete`** txn (consume+revoke-priors+insert, R11);
  the sign-in lookup is **`POST …/credentials/lookup`** keyed by `credential_id` (usernameless assertion),
  pre-session + `adminSharedSecret`-only (no `X-Admin-Id`) — this supersedes the plan §6 `?admin_id=` draft.
  Adversarial verification (4 dimensions: cross-refs · frozen contract · core/store/schema · mandate +
  WebAuthn-flow design) ran read-only against ground truth — **all claims accurate**; 2 wording nits fixed
  (RLS `$1` notation; explicit plan-§6 supersession note).

## T02 — Store: B1 invite-resolve + consume + credential CRUD (real PG18)
**Does:** Extend the existing admin-provisioning store (`server/store/src/lib.rs`, alongside
`AdminProvisioningStore`/`create_pending_admin_with_invitation`/`reissue_admin_invitation`) with the B1
read/CRUD methods: `resolve_invitation_by_token` (hash via `core::admin_invitation_token_matches` inside
the RLS txn; value-free not-found — **no existence oracle**), atomic `consume_invitation` (conditional
`UPDATE … WHERE consumed_at IS NULL RETURNING …`), `list_active_credentials`, `find_active_credential`,
`insert_credential`, `revoke_all_for_admin`, `bump_sign_count` (only-if-greater), and a combined
**`register_complete`** that does consume + revoke-priors + insert in **one transaction** (R11). All over
the unnamed `query_typed*`/`execute_typed` family (ADR-0024), all `set_config('app.current_group_id',$1,
true)`-scoped (R16); reuse the `ensure_least_privilege` path (R17). PII-free wire DTOs (`AdminInviteRecord`,
`AdminCredential`) in `core/server`, keyed-serde pinned.
**Touches:** `server/store/src/lib.rs`, `core/server` (DTOs).
**Closes:** AC4a (atomic+concurrent consume), AC4b (resolve-through-core), AC3 (persist-across-fresh-store),
AC14 (local PG isolation). · **Tests (`server/store/tests/admin_webauthn.rs`):**
`pg_admin_auth_store_consume_is_single_use` (+ concurrent double-consume → exactly one wins, R15);
`pg_admin_auth_store_resolve_routes_through_core_match`; persist-across-fresh-store-instance (AC3 store
leg); `pg_admin_auth_store_isolates_invite_and_credentials_by_tenant` (Group-B token invisible to a
Group-A-scoped store, AC14). Mirrors `server/store/tests/admin_invitations.rs` + `common/` (`app_store`
as non-superuser `boundless_app`). · **Blockers:** none. **∥** yes (independent of T01/T03).
- **Status:** ✅ DONE 2026-06-13. New `AdminWebAuthnStore` port trait + PII-free wire DTOs
  (`AdminInviteRecord`/`AdminCredential`/`NewAdminCredential`/`RegisterCompleteOutcome`,
  `core/server/src/admin_webauthn.rs`, snake_case + base64url-no-pad bytea, **keyed-serde pinned**);
  `impl AdminWebAuthnStore for PgAuthStore` (`server/store/src/lib.rs`, RLS-scoped `query_typed*`,
  ADR-0024) with `resolve`/`consume`/list/find/insert/`revoke_all`/`bump_sign_count` (only-if-greater,
  +`revoked_at IS NULL`) + atomic **`register_complete`** (consume + revoke-priors + insert, one txn,
  admin_id server-derived). `base64`+`uuid` promoted to runtime deps of `core/server` (workspace deps;
  **no new crate version**; stack-matrix updated; wasm no-getrandom gate ✓). **Tests:** 9 real-PG18 in
  `server/store/tests/admin_webauthn.rs` (resolve-through-core AC4b · single-use + concurrent-consume
  R15 · `register_complete` atomic + revokes-priors R11 · persist-across-fresh-store AC3 · sign-count
  only-if-greater R10 · duplicate-credential_id rejected · tenant isolation AC14) + 2 core serde-pin.
  All green; clippy `-D warnings` clean; full core+store suites green. **Design call (flag for T03):**
  AC4b's "match through the core" is realized as `admin_invitation_token_hash` + indexed equality (the
  `classify_refresh` precedent), not the AC-named `_matches` — reconcile AC4b/ADR-0027 wording at T03
  (→ DEFERRED). **Review:** 3 lenses (reviewer · security-auditor · adversarial design) + per-finding
  adversarial verify — **0 crit/high, 0 confirmed findings**; 4 low/nits actioned in-slice (revoke-SQL
  single-sourced · dup-credential test · `revoked_at IS NULL` on bump · public_key decode-asymmetry
  doc), the rest → DEFERRED (T03/T04/T05 carry-forwards).

## T03 — OpenAPI freeze + contract test for the B1 surface
**Does:** Add the B1 ops + PII-free schemas to `api/openapi.yaml` under the chosen prefix (T01); add a
dedicated **B1 describe-block** to `web/tests/contract/api-contract.test.ts`.
**Touches:** `api/openapi.yaml`, `web/tests/contract/api-contract.test.ts`, `docs/error-codes.md` (only if
a new code proves necessary — expected: none).
**Closes:** AC13. · **Tests (the new describe-block):** (1) B1 ops exist + frozen (non-vacuity); (2)
`adminSharedSecret` on every B1 op; (3) `AdminIdHeader` **only** on session-bearing ops, **absent** on
invite-resolve/consume (positive AND negative); (4) **no B1 response schema reaches a member-PII schema**
(`MemberDetail`/`MemberSummary`/`DuplicatePhoneLink`/`phone`/`address`) — the I5 negative gate (F2).
· **Blockers:** T01 (route names). **∥** yes (parallel with T02).
- **Status:** ✅ DONE 2026-06-13. Froze the **4 pre-session ops** under `/api/admin/webauthn/*`
  (`invite/resolve` → `AdminInviteRecord`; `register-complete` → `AdminRegisterCompleteResult`, the
  atomic R11 combined op; `credentials/lookup` → `AdminCredential`, usernameless; `credentials/{id}/
  sign-count` → 204) in `api/openapi.yaml` — EXACTLY the ADR-0027 table, all `adminSharedSecret`-only,
  no `X-Admin-Id`. 8 new **PII-free** schemas byte-faithful to the T02 keyed-serde pins (snake_case,
  base64url-no-pad `bytea`, epoch-int timestamps, `consumed_at`/`revoked_at` present-but-nullable 3.1
  `type:[integer,"null"]`, `transports`/`aaguid` omitted-when-absent); token in the **body** (R13).
  No new error code (invite-verdict + BFF-gate codes already registered). **No frozen shape touched**
  (purely additive). **Tests:** 5 `it()`s in the new B1 describe-block of
  `web/tests/contract/api-contract.test.ts` — ops-exist-and-frozen (count pinned to exactly 4, no silent
  growth) · `adminSharedSecret` on every op · **pre-session no-`X-Admin-Id`** (negative + member-ops
  positive control) · **I5 negative gate** (no B1 response reaches `MemberDetail`/`MemberSummary`/
  `DuplicatePhoneLink` or a `phone`/`address` field — schema-ref + recursive property-name walk) ·
  wire-DTO shape freeze (mirrors the Rust serde pins). Web suite 98/98; strict typecheck clean;
  binding-drift lock regenerated + green. **Decision (recorded):** only the 4 pre-session ops are frozen
  (the ADR-0027 freeze target); the session-bearing backup-key ops (`credentials?admin_id=` list +
  standalone `revoke-all`) are **out of scope** (spec.md) → deferred. **Doc reconciliation:** closed the
  T02→T03 carry-forward — AC4b + ADR-0027 now describe the as-built token match
  (`admin_invitation_token_hash` + unique-index equality, timing-safe — the `find_member_by_phone`
  precedent), not a call to `admin_invitation_token_matches`. **Review:** 3-lens adversarial workflow
  (reviewer · security-auditor · platform-parity), per-finding refutation.

## T04 — Worker: B1 admin endpoints + pre-session guard
**Does:** `server/src/runtime/admin_auth.rs` (handlers composing the T02 store methods) + register routes
in `server/src/runtime/mod.rs`; add a **pre-session `admin_guard` variant** in `members.rs` (shared
secret required, `X-Admin-Id` not) for invite-resolve/consume. The invite token arrives in the **POST
body**, never the URL (R13); the resolve/consume error paths emit value-free codes only, no token (R13/R20).
**Touches:** `server/src/runtime/{admin_auth.rs,mod.rs,members.rs,pg.rs}`.
**Closes:** AC4a (worker round-trip), AC8 (worker invite-resolve no-token-in-log), AC14 (miniflare
cross-tenant). · **Tests:** `server/test/admin-webauthn.spec.ts` (resolve/consume/register-complete
round-trips; `invite_resolve_error_body_has_no_token`); `server/test/admin-webauthn-cross-tenant.spec.ts`
(`worker_cross_tenant_invite_resolve_isolated`) — mirror `server/test/cross-tenant.spec.ts`; seed a
Group-B invite/credential via `setup-worker-test-db.sh`. · **Blockers:** T02, T03. **∥** no.
- **Status:** ✅ DONE 2026-06-13. New `server/src/runtime/admin_auth.rs` — 4 pre-session handlers
  (`invite_resolve`/`register_complete`/`credential_lookup`/`bump_sign_count`) composing the T02
  `AdminWebAuthnStore` over `PgAuthStore`, RLS-scoped via `build_admin_store` (connect + W2
  `ensure_least_privilege` + `GROUP_ID`, R16/R17); token in the POST body → tainted
  `AdminInvitationToken` on parse, value-free codes only (R13). Added `pub(crate) admin_secret_guard`
  to `members.rs` (shared-secret only, NO `X-Admin-Id`) — `admin_guard` now layers the X-Admin-Id leg
  on it (single-sourced constant-time compare); `err_code`/`audited_body` made `pub(crate)`. Routes
  registered in `mod.rs`. New core wire DTO `AdminRegisterCompleteResult` (`core/server`,
  keyed-serde pinned) + the 3 B1 DTOs blessed `AuditedResponse` (the Worker emits every B1 200 through
  `admin_response_body`). `base64` added to the Worker's wasm deps (inbound base64url decode — existing
  workspace version, no new crate). 2 new value-free 404 codes registered:
  `ADMIN_INVITE_NOT_FOUND`/`ADMIN_CREDENTIAL_NOT_FOUND` (no client surface, no existence oracle).
  Seed: new `server/store/examples/seed_worker_test_b1_pg.rs` (P4 — token hashes computed in the core)
  + `setup-worker-test-db.sh` seeds a Group-A resolve invite + a register-invite POOL (re-run-safe) +
  a Group-B cross-tenant invite & credential. **Tests:** `server/test/admin-webauthn.spec.ts` (5:
  resolve round-trip + value-free 404 · register-complete→lookup→only-if-greater bump→single-use
  reject R10/R11/AC4a · pre-session no-`X-Admin-Id` · fail-closed 401 · `invite_resolve_error_body_has_no_token`
  AC8) + `server/test/admin-webauthn-cross-tenant.spec.ts` (`worker_cross_tenant_invite_resolve_isolated`:
  Group-A Worker can't resolve/consume a Group-B invite nor look up a Group-B credential, non-vacuous,
  AC14). Full server miniflare suite 19/19 green (verified re-run-safe across 2 runs); core 9/9
  admin_webauthn + full core suite green; wasm + native clippy `-D warnings` clean; binding-drift lock
  regenerated (core/** input); the I5 trybuild golden re-blessed (the new types appear in rustc's
  "other types implement `AuditedResponse`" hint — cosmetic, the E0277 compile-fail is unchanged).

## T05 — Web: Worker-backed invite/credential stores + fail-closed selectors
**Does:** `web/src/lib/server/webauthn/worker-stores.ts` — `WorkerInviteStore`/`WorkerCredentialStore`
implementing the existing ports (mirror `WorkerMembersClient`: Bearer secret, base from
`ADMIN_WORKER_BASE`, value-free `fail`); token in POST body. `selectInviteStore`/`selectCredentialStore`
fail-closed selectors (real when configured; in-memory only in dev; else throw). Confirm the member
selector (`selectMembersClient`) still fails closed.
**Touches:** `web/src/lib/server/webauthn/worker-stores.ts` (new), `web/src/lib/server/members-deps.ts`
(re-confirm).
**Closes:** AC1 (member + new selectors fail-closed), AC4b (prod InviteStore is Worker-backed).
· **Tests:** `production-invite-store-is-worker-backed.test.ts` (prod selector → Worker adapter, never
`MemoryInviteStore`); adapter request-shape tests (mock `fetch`, mirror `members.test.ts`); a CI lint /
`grep` asserting no `hmac`/`createHmac`/`subtle.sign` in `web/src/lib/server/webauthn/**` (AC4b structural
backstop). · **Blockers:** T03 (contract). **∥** yes (parallel with T06, T08).
- **Status:** ✅ DONE 2026-06-13. New `web/src/lib/server/webauthn/worker-stores.ts` — `WorkerInviteStore`
  + `WorkerCredentialStore` implementing the existing `InviteStore`/`CredentialStore` ports against the 4
  frozen B1 ops (`invite/resolve`, `register-complete`, `credentials/lookup`, `credentials/{id}/sign-count`),
  mirroring `WorkerMembersClient` (Bearer shared secret, **pre-session — NO `X-Admin-Id`**, base from
  `ADMIN_WORKER_BASE`, value-free `fail(res)`); token in the POST **body** (R13). The three-call
  registration tail (`markConsumed`→`revokeAllForAdmin`→`insert`) coalesces into the single atomic
  `register-complete` via a shared per-request **`WorkerRegistrationHandshake`** (R11) so `register.ts`
  stays unchanged (R12): `markConsumed` stashes the token (no network), `insert` fires register-complete,
  `revokeAllForAdmin` is a no-op (revoke is server-side in that txn). base64url asymmetry honored
  (`public_key` decode↔encode; `credential_id` string; **`aaguid` converted dashed-hex UUID ↔ base64url-
  of-16-bytes** in both directions, malformed omitted). `listActiveByAdmin`→`[]` (no frozen pre-session
  list op; revoke-and-replace makes excludeCredentials moot). Fail-closed `selectInviteStore`/
  `selectCredentialStore` (real when base+secret; in-memory only in dev; else throw). `members-deps.ts`
  cross-ref comment (member selector still fails closed — AC1). **Tests:** `worker-stores.test.ts` (17:
  request shapes for every op, register-complete coalescing fires exactly one call, handshake single-use,
  400→`WebAuthnError(ADMIN_INVITE_CONSUMED)`, aaguid UUID↔base64url both directions + malformed-omit,
  value-free throws, both selectors fail-closed) + `production-invite-store-is-worker-backed.test.ts`
  (AC4b: prod selector → `WorkerInviteStore` never `MemoryInviteStore`; the no-`hmac`/`subtle`-in-edge-TS
  structural lint, comment-stripped). Full web suite 118/118; strict typecheck clean. No `core/**`/`api/**`
  touched → no binding-drift regen. **Review:** 3-lens adversarial workflow (reviewer · security-auditor ·
  platform-parity) + per-finding refutation — 3 confirmed findings, all the SAME real defect (the aaguid
  encode asymmetry: register.ts feeds @simplewebauthn's dashed-hex UUID into a base64url-bytea wire field,
  which the Worker would silently decode to garbage) **fixed in-slice** with a symmetric UUID↔bytea
  converter + tests (verified oracle); the 1 refuted "untested aaguid path" dissolved by those tests.

## T06 — Web: KV-backed admin session store
**Does:** Replace the in-memory `Map` in `web/src/lib/server/session.ts` with `KvSessionStore` over
`ADMIN_SESSIONS` (pure-core + shell, mirror `kv-challenge-store.ts`): ≥128-bit opaque id (keep
`crypto.randomUUID()`, R1); `put` with `expirationTtl` **and** an in-value `expiresAt`; `get` rejects
`now >= expiresAt` (server-side TTL, R2); `revoke` deletes (best-effort window documented, D5/R3). Fresh
authenticated id minted post-assertion, distinct from the ceremony cookie (R4). `selectSessionStore`
fail-closed. Make `createSession`/`getSession`/`requireAdminId` async and **audit every call site**.
**Touches:** `web/src/lib/server/session.ts`, `web/src/routes/api/admin/auth/signin/+server.ts`, every
`(app)` load/action calling `requireAdminId`.
**Closes:** AC2. · **Tests (`web/src/lib/server/kv-admin-session-store.test.ts`, miniflare via
`getPlatformProxy`):** round-trip; **cold-start** = fresh store instance over the same KV resolves the id;
TTL via injected `Clock` (expired id → `null` → redirect); revoke; selector fail-closed branches; entropy
(two mints differ, not `adminId`-derived); ceremony-cookie ≠ session-cookie (R4). · **Blockers:** none.
**∥** yes (parallel with T05, T08).
- **Status:** ✅ DONE 2026-06-13. New pure-core `web/src/lib/server/kv-admin-session-store.ts`
  (`SessionStore` port; `KvSessionStore`/`MemorySessionStore`; fail-closed `selectSessionStore`; the
  `ADMIN_SESSION_COOKIE`/`SESSION_COOKIE_OPTIONS` constants) — no `$app`/`$env` virtuals, so bare-Vitest
  testable (mirrors `kv-challenge-store.ts`). **Two-layer TTL:** KV `expirationTtl` (eviction backstop,
  floored to KV's 60s min) **plus** an in-value `expiresAt` checked server-side on every `get` — the
  authoritative logical expiry (R2). `revoke` = `kv.delete` (best-effort, D5/R3). Rewrote the shell
  `session.ts`: imports `dev`; `SESSION_TTL_SECS = 12h` (ADR-0016 shorter-lived; tunable); real `clock`;
  `MemorySessionStore` fallback singleton; `sessionStore(platform)` = `selectSessionStore(platform?.env
  ?.ADMIN_SESSIONS, clock, TTL, fallback, dev)`; async `createSession`/`getSession`/`revokeSession`/
  `requireAdminId`; `resetSessions()` resets the fallback; re-exports the cookie constants. Threaded
  `platform` + `await` through the 7 call sites (`admin/+page`, `(app)/+layout` — both now async,
  `audit-log`, `members`, `members/[id]`×3, `signin`, `seed-session`); `reset/+server.ts` unchanged
  (`resetSessions()` stays sync). Added the `ADMIN_SESSIONS` binding to `app.d.ts` (`App.Platform.env`)
  + `web/wrangler.toml` (placeholder id; real id + the `wrangler types`-generated `App.Platform` are T09).
  **Deleted** `session.test.ts` (its assertions migrated — it can no longer load under bare Vitest once
  `session.ts` imports `$app/environment`). **Tests:** `kv-admin-session-store.test.ts` (18: §10-F cookie
  policy; pure selector fail-closed/dev branches; `MemorySessionStore` round-trip/unknown/undefined/TTL/
  revoke/entropy; real-Miniflare-KV `KvSessionStore` round-trip, **cold-start** (fresh store over the same
  KV resolves the id — AC2), **server-side TTL** via injected movable clock (R2), revoke, absent/undefined,
  R1 entropy + R4 no-fixation). Full web suite 131/131; strict typecheck clean. No `core/**`/`api/**`
  touched → no binding-drift regen. **Review:** 3-lens adversarial workflow (reviewer · security-auditor ·
  correctness/parity) + per-finding refutation — **0 crit/high/med/low**; 1 confirmed nit (the
  `getSession(undefined)` + `resetSessions()` isolated-unit assertions lost in the migration) **actioned
  in-slice** by lifting the `undefined`-id guard into the pure-core `get()` contract + 2 isolated tests
  (the `resetSessions()` singleton-swap stays e2e-covered — genuinely shell-only); the other 4 findings
  refuted/out-of-scope. AC1's session-selector fail-closed leg is also re-confirmed here.

## T07 — Web: swap deps, remove authority dev-seams, keep e2e green
**Does:** `webauthn-deps.ts` swaps the memory stores for the selected Worker-backed ones (ceremony files
`register.ts`/`authenticate.ts` unchanged — R12). **Delete** the authority-minting dev seams
(`/api/test/seed-session`, `/api/test/seed-invite`); **repoint** `reset`/`seed-member` to the dev-durable
backends (behind the `dev`-inlined-`false` gate, a dev-only namespace) so the T10/T15 Playwright suites
stay green (F12/R21). Drive the e2e onboarding via the Playwright CDP virtual authenticator + the operator
seed against a dev backend.
**Touches:** `web/src/lib/server/webauthn-deps.ts`, `web/src/routes/api/test/*`,
`web/tests/e2e/{webauthn,admin-onboarding,admin-members}.spec.ts`.
**Closes:** AC5 (build-artifact tree-shake), AC3 (e2e ceremony leg), AC12 (regression run). · **Tests:**
`web/tests/build/no-dev-seams.test.ts` (handlers tree-shaken, `dev===false` inlined);
`ac3_credential_persists_across_cold_start` (virtual authenticator → credential survives a fresh store);
re-run `admin-onboarding.spec.ts` + `admin-members.spec.ts` as the F12/AC12 regression gate (axe ×variant
stays green). · **Blockers:** T05, T06. **∥** no.
- **Status:** ✅ DONE 2026-06-14. `webauthn-deps.ts`: `getWebAuthnDeps` builds ONE per-request
  `WorkerRegistrationHandshake` shared by `selectInviteStore` + `selectCredentialStore` (the R11
  register-complete coalescing); the in-memory stores become the dev fallbacks; `resolveInviteStatus`
  resolves through the selected store; `register.ts`/`authenticate.ts` UNCHANGED (R12, verified). **Dev-seam
  decision = Option A (owner-chosen):** KEEP all four `/api/test/*` seams (dev-gated), relying on AC5
  tree-shaking as the prod I11 guarantee — a documented deviation from this task's literal "delete
  seed-session/seed-invite," which spec §F permits ("repoint-to-durable-store-in-dev"); the keep-and-tree-shake
  rationale + the literal-delete future option are recorded in DEFERRED.md. **AC5:**
  `tests/build-gates/no-dev-seams.test.ts` runs the REAL `pnpm build` under a hostile `NODE_ENV=test` and
  asserts every seam handler's first statement is an unconditional `error(404)`. **Hardening from the review
  (HIGH):** SvelteKit inlines `dev` from NODE_ENV (NOT Vite `--mode` — both verified), so a `NODE_ENV=test`
  build shipped the LIVE session-minting seam — **pinned `NODE_ENV=production` in the `package.json` build
  script**; the AC5 test now proves that pin is load-bearing (builds under the hostile env). Web suite **152**
  green; typecheck clean; `pnpm build` green. **AC12:** the store swap is behavior-preserving in dev (selectors
  fall back to the same in-memory stores when `ADMIN_WORKER_BASE` is unset), so the unit suite stands in
  locally; the axe ×variant Playwright regression is CI-gated. **Review:** 4-lens adversarial workflow
  (reviewer · security-auditor · platform-parity · correctness) + per-finding verification — 7 confirmed: 2
  HIGH/MED (the NODE_ENV pin + a `delete env.VITEST` typecheck break) **fixed in-slice**; 4 LOW (scrub gaps,
  no-console strip edge) → DEFERRED; 1 LOW (this Option-A doc divergence) closed here + in DEFERRED.

## T08 — Web: scrubbed logging (P2/I10)
**Does:** `web/src/lib/server/log.ts` (`emit()` scrubber sink) + a `handleError` hook in
`web/src/hooks.server.ts`; a no-raw-`console` lint over `web/src/**`.
**Touches:** `web/src/lib/server/log.ts` (new), `web/src/hooks.server.ts`, lint config.
**Closes:** AC8 (web leg). · **Tests (`web/src/lib/server/web-emit-scrubber.test.ts`):** redacts the
no-KV / no-backend operator strings (assert they carry no secret substring) and a **URL-embedded opaque
invite token** (assert redacted, R13); `handleError` routes uncaught throws through the sink. · **Blockers:**
none. **∥** yes (parallel with T05, T06).
- **Status:** ✅ DONE 2026-06-14. New pure `web/src/lib/server/log.ts` — `scrub()` redacts Bearer secrets /
  `postgres://` conn-strings / ≥32-char high-entropy blobs (incl. the 64-hex/UUID/base64url invite token,
  R13); `emit()` is the SOLE sanctioned `console` sink; `logServerError()`. `hooks.server.ts` `handleError`
  routes uncaught throws through it, logging `route.id` (the pattern, never `url.pathname`) and returning
  void (SvelteKit's default client shape → no new user-visible copy, P8). **Tests:** `web-emit-scrubber.test.ts`
  (10) + `web/tests/cross-platform/web-no-raw-console.test.ts` (grep-as-test: no raw `console.*` in `web/src`
  except `log.ts`). Web suite green; typecheck clean. **Review:** the scrub residual gaps (phone/name/address
  PII; all-digit / base64-`+/=` / sub-32 blobs; the shared `stripComments` `//`-in-string edge) are LOW and
  mirror the Rust scrubber's already-deferred gaps → DEFERRED (the web tier's primary PII defense is
  structural: route-id logging + wire-DTOs only; PII stays in the Worker).

## T09 — wrangler config + bindings + type-drift CI
**Does:** `web/wrangler.toml` — real `CHALLENGES` id, add `[[kv_namespaces]] ADMIN_SESSIONS`,
`ADMIN_WORKER_BASE` + `WEBAUTHN_RP_ID/_ORIGIN/_RP_NAME` in `[vars]` (workers.dev for dev/test), no secret
in `[vars]`, deploy-target keys, `send_metrics=false` (already set). Move `App.Platform` to `wrangler
types`-generated; **fix the stale `app.d.ts`/`wrangler.toml` comments** that describe a web-tier Hyperdrive
binding (platform-parity F5).
**Touches:** `web/wrangler.toml`, `web/src/app.d.ts` (→ generated), CI config.
**Closes:** AC6, AC15, AC11 (local `rp-config` env-driven leg). · **Tests:** credential-scan gate over
`web/wrangler.toml` (AC6); `web/tests/build/wrangler-types-match.test.ts` (regen `wrangler types` → diff
vs committed, AC15); `web/src/lib/server/webauthn/rp-config.test.ts` (RP config fully env-driven, no
hard-coded host). · **Blockers:** T06 (the `ADMIN_SESSIONS` binding exists). **∥** yes (after T06).
- **Status:** ✅ DONE 2026-06-14. `wrangler.toml`: added `[vars]` (`ADMIN_WORKER_BASE` +
  `WEBAUTHN_RP_ID`/`_ORIGIN`/`_RP_NAME`, `REPLACE_AT_DEPLOY` placeholders, **no secret** — `ADMIN_API_SECRET`
  stays a `wrangler secret`); removed the stale `[[hyperdrive]]` block + comments (B1 = zero web Postgres,
  platform-parity F5). Verified (docs-researcher + sveltejs/kit#15736): `[vars]` reach the deployed Worker via
  `$env/dynamic/private` but NOT `vite dev` (only `platform.env`), so the dev/e2e fallback path is unchanged.
  New pure `webauthn/rp-config.ts` (`resolveRpConfig`: env wins; dev → request URL; outside dev with
  RP_ID/ORIGIN unset → **throws**, never trusts the request Host — ADR-0017); `webauthn-deps.ts::rpConfig`
  now calls it. **AC15 is a drift test, NOT the literal D6 generated `App.Platform`:** `wrangler types`' full
  output bundles a 14k-line workerd runtime whose global `KVNamespace` collides with the pinned
  `@cloudflare/workers-types` the project uses via `import type` (and the env-only output needs a global
  `KVNamespace` the project deliberately avoids) → kept the hand-typed `app.d.ts` +
  `tests/build-gates/wrangler-types-match.test.ts` (runs `wrangler types --include-runtime=false`, asserts the
  BINDINGS — identifier-typed; `[vars]` are string-literal-typed and excluded — match `App.Platform.env`).
  **AC6:** the `web` CI job now runs `check-wrangler-credentials.sh web/wrangler.toml`. **Tests:**
  `rp-config.test.ts` (5) + `wrangler-types-match.test.ts` (1). Web suite green; typecheck clean.

## T10 — Operator first-admin seed
**Does:** `scripts/seed-admin-invite.sh` + `server/store/examples/seed_admin_invite_pg.rs` — drive the
existing `create_pending_admin_with_invitation` (member role=admin, NULL PII — schema confirms it) +
`reissue_admin_invitation` (idempotent re-run, R19). Owner URL via **env, not argv**; the minted token
emitted **only to stdout** (never argv, never a structured log line — R20); `created_by` = a documented
`"operator-seed"` sentinel (R18). Mirror `bootstrap-group.sh` / `bootstrap_group_pg.rs`.
**Touches:** `scripts/seed-admin-invite.sh` (new), `server/store/examples/seed_admin_invite_pg.rs` (new).
**Closes:** AC7. · **Tests:** `pg_seed_admin_creates_pending_admin_and_invitation` meta-test (1 null-PII
admin + 1 invite; no PII; idempotent re-run respects `one_live_per_admin`); a fixture/assert that the
seed's own log lines never carry the token. · **Blockers:** T02 (the store methods it reuses). **∥** yes.
- **Status:** ✅ DONE 2026-06-14. New `server/store/examples/seed_admin_invite_pg.rs` (operator example,
  never compiled into the lib/Worker) drives the **existing** core methods (P4 — no parallel store, no
  schema change): no admin-id → `create_pending_admin_with_invitation` (null-PII pending Admin + first
  invite); an admin-id → `reissue_admin_invitation` (atomic supersede-then-insert, R19). Token hash
  computed **in the core** (`admin_invitation_token_hash`); native-TLS connection (Neon `sslmode=require`
  / local `disable`, the `bootstrap_group_pg.rs` pattern). **No `println!`** (the lint forbids it in
  non-test Rust): exit codes (0 created / 3 re-invited / 4 no-such-admin) + a file side-channel for the
  non-secret admin id. New `scripts/seed-admin-invite.sh` — owner URL + HMAC key + token via **ENV, never
  argv** (R20); token via `openssl rand -hex 32`; **token printed ONLY to stdout** (`admin_id`/`token`/
  `onboard_url`), progress to stderr **redacted** (the owner password never logged, P2). `created_by`
  stays **NULL** — no Developer WebAuthn identity yet, a documented audit-actor gap (R18/F5), NOT a fake
  sentinel (a recorded deviation from plan §10.4). New `scripts/test-seed-admin-invite.sh` (the AC7
  meta-test, mirrors `test-bootstrap-group.sh`): throwaway local DB → proves null-PII admin + 1 live
  invite, **token only on stdout / never stderr** (R20), idempotent re-invite keeps exactly one live
  invitation (`one_live_per_admin`), and a phantom admin-id is a clean error — **PASSES against real local
  PG18**; wired into CI after `test-bootstrap-group.sh`. Example builds + clippy `-D warnings` clean.
  **Review:** 3-lens adversarial workflow (reviewer · security-auditor · correctness) + per-finding
  verification — 3 confirmed, all **fixed in-slice**: a HIGH+MED (the same defect — the meta-test's
  defense-in-depth owner-password self-check greped the bare password, which false-fails deterministically
  in CI where the common password `postgres` is a `postgres://`-scheme substring; passed locally only
  because the local URL is password-less) → rewritten to assert on the whole `user:pass` **userinfo
  segment** (a structured token the redacted log can't contain), verified against the exact CI-collision
  case; a LOW (`SEED_INVITE_TTL_SECS=0` minted an already-expired invite) → added a `>0` guard. The Rust
  example, the runbook, and the CI step were clean.

## T11 — Deploy runbook
**Does:** `docs/runbooks/deploy-admin-web.md`, mirroring `deploy-worker.md`: KV create ×2, secrets
(`ADMIN_API_SECRET` byte-identical to the Rust Worker's, can't be read back — R6), WebAuthn domain config
(workers.dev + custom-domain, **RP_ID-is-permanent** warning — D7), the operator first-admin seed (T10),
`wrangler deploy`, the smoke (T12).
**Touches:** `docs/runbooks/deploy-admin-web.md` (new). **Closes:** P12 (operability). · **Tests:** none
(doc; the steps are exercised by T13). · **Blockers:** T09, T10 (so the steps are accurate). **∥** yes.
- **Status:** ✅ DONE 2026-06-14. New `docs/runbooks/deploy-admin-web.md` (mirrors `deploy-worker.md`):
  prerequisites (the Rust Worker already deployed + the Group bootstrapped) · **the two footguns up
  front** — `ADMIN_API_SECRET` byte-identical to the Rust Worker's + can't-be-read-back (a mismatch →
  opaque `ADMIN_UNAUTHORIZED` 401), and **RP_ID is permanent** (D7, the passkey-reset cutover) · steps:
  create the 2 KV namespaces (`CHALLENGES` + `ADMIN_SESSIONS`) → fill the `[vars]` hosts → `wrangler secret
  put ADMIN_API_SECRET` → **the operator first-admin seed (T10)** → **`pnpm build` (the load-bearing
  `NODE_ENV=production` seam-stripping pin, never a bare `vite build`) + `wrangler deploy`** → the live
  smoke (forward-references `smoke-deployed-admin-web.sh`, T12). The deploy step pins the **docs-verified**
  adapter-cloudflare 7.2.8 Workers keys (`main = ".svelte-kit/cloudflare/_worker.js"` + `[assets]`) and
  flags the AC15 drift-test classifier extension for the new `ASSETS` binding. Custom-domain RP_ID cutover
  + troubleshooting (401 secret-mismatch / 503 group-key-missing / fail-closed) sections. Doc only —
  exercised live in T13.

## T12 — Deployed-edge smoke script
**Does:** `scripts/smoke-deployed-admin-web.sh`, mirroring `smoke-deployed-edge.sh`: seed invite → register
passkey → sign-in → list/issue → sign-out → revoked-cookie returns the `/admin/signin` redirect → assert
`Referrer-Policy: no-referrer` on the invite route + `RP_ID` not `localhost` → the opt-in ≥2-Group
cross-tenant block. Written + lint-clean locally; **run** live in T13.
**Touches:** `scripts/smoke-deployed-admin-web.sh` (new). **Closes:** (provides the AC10/AC11/AC14-edge
harness). · **Tests:** shellcheck / a dry-run against `wrangler dev` if feasible. · **Blockers:** T04, T07
(the flow it drives must exist). **∥** yes.

## T13 — (edge) Deploy + live verification — OPERATOR-GATED
**Does:** The human-gated run (the agent never runs `wrangler deploy`): `pnpm build`, create the 2 KV
namespaces, set `ADMIN_API_SECRET`/`ADMIN_WORKER_BASE`, deploy, run the operator seed, run
`smoke-deployed-admin-web.sh` (incl. the ≥2-Group cross-tenant probe). Tick the **(edge)** ACs on success;
update the AC tracker + move this slice's completed register to `docs/deferred-archive.md`.
**Touches:** Cloudflare account (operator), the AC tracker, `DEFERRED.md`/archive.
**Closes:** AC9, AC10, AC11 (edge), AC5 (edge probe), AC14 (edge ≥2-Group). · **Tests:** the live smoke is
the test. · **Blockers:** T03–T12 all landed. **∥** no (last).

---

## Serialization summary

- **T01 → T03** (ADR fixes the route names before the contract freeze).
- **T02 + T03 → T04** (the Worker composes the store + the frozen contract).
- **T03 → T05** (web adapters target the frozen contract).
- **T05 + T06 → T07** (deps swap + dev-seam removal).
- **T02 → T10** (the seed reuses the store methods).
- **T13 last** (needs T03–T12).
- **Parallelizable:** T02 ∥ T03 (early); then T05 ∥ T06 ∥ T08; T09 (after T06); T10 (after T02); T11/T12
  as prep. Solo dev: the natural order is T01 → T02 → T03 → T04 → T05 → T06 → T07 → T08 → T09 → T10 → T11
  → T12 → T13.

## DEFERRED.md additions to record as tasks land (plan §11)
R7 (verify `X-Admin-Id` is a real admin — security-hardening pass) · R18 (seed `created_by` = Developer
identity — spec 001 T08-shell) · R3/R5 (KV revocation window / admin-session device-binding — if a
Worker-native session is wanted) · the custom-domain RP_ID cutover (D7) · fold the seed's argv-secret
class into the existing deploy-hardening item. Plus each task's own out-of-scope register (per the project
convention).
