# 009 — Deploy the admin web dashboard — Technical Plan

> Plan status: Ready for `/speckit.tasks`
> Spec: `specs/009-admin-web-deploy/spec.md` (Clarified, decisions D1–D8 pinned)
> Date: 2026-06-13
> Inputs: `architect`, `platform-parity`, `security-auditor`, `test-strategist` subagent passes (read-only).

---

## 0. What this plan delivers

Make the SvelteKit admin dashboard (`web/`) a live, usable Cloudflare deployment wired end-to-end to
the already-deployed Rust admin Worker. Under **Option B1**: the WebAuthn invite + credential
*persistence* moves behind new server-to-server admin endpoints on the Rust Worker (web tier keeps zero
direct Postgres); admin sessions persist in a new `ADMIN_SESSIONS` KV; the first admin is onboarded via
an operator seed script; the WebAuthn RP_ID is env-configurable (`*.workers.dev` for dev/test, a custom
domain for production).

**Key discovery (reuse, don't rebuild):** the core/store already ships
`AdminProvisioningStore::create_pending_admin_with_invitation` and `reissue_admin_invitation`
(`server/store/src/lib.rs` ~582/~618) with proven, atomic supersede-then-insert ordering against the
`admin_invitations_one_live_per_admin` index, and `core/crypto::admin_invitation_token_matches`
(`core/crypto/src/hashing.rs:308`, constant-time, domain-tagged) already exists. B1 **extends** this
surface — it does not build a parallel one. The operator seed drives the existing core methods (P4).

---

## 1. Constitution gate check (P1–P13)

| Principle | Touched? | How this plan obeys it |
|---|---|---|
| **P2** — no PII in logs | **Yes (heavily)** | Web `emit()` scrubber sink + `handleError` hook + no-raw-`console` lint (§G). The invite token must stay off both the web AND the new Worker invite-resolve log paths (R13). No PII transits the new endpoints — invite/credential metadata only. |
| **P4** — Rust core is source of truth | **Yes** | The invite-token HMAC compare stays in the core (`admin_invitation_token_matches`); the new Worker invite-resolve calls it, never edge-TS (AC4b). The operator seed reuses the core `AdminProvisioningStore` methods. No UniFFI/wasm-client surface (admin-web-only). |
| **P5** — spec before code | Yes | Spec 009 clarified; this plan; then `/tasks`. |
| **P6** — plan mode default | Yes | This plan → `/tasks` → `/implement`. |
| **P8** — i18n not afterthought | Light | No new user-visible strings expected; at most one new `en` catalog key (`admin.backend_unavailable`), never hardcoded. The fail-closed selector throws an *operator* string, not user copy. |
| **P11** — free/open | Yes | `ADMIN_SESSIONS` KV is the same free-tier class as `CHALLENGES`; two Workers stay two Workers; no paid dependency. I8 network allow-list covers `web/`'s lock file. |
| **P12** — operability | **Yes** | New `docs/runbooks/deploy-admin-web.md`, `scripts/smoke-deployed-admin-web.sh`, scrubbed logging, stable error codes (reused, no new ones needed). |
| **P1 / a11y** | Re-verify | AC12: the existing T10/T15 axe-core ×variant runs must stay green after the backend wiring (no new screens). |
| **I5** — admin PII reads audit-logged | Preserved | The new endpoints disclose admin credential/invite metadata, **not** member PII → **not** `x-requires-audit` (AC13). The member endpoints' `#[require_audit]` gate is untouched. Two pre-existing, documented audit-*completeness* gaps remain deferred (R7 forge-`X-Admin-Id`; R18 seed `created_by`). |
| **I11** — admins issued only by Developer | Preserved | The operator seed is the Developer's deliberate action; no signup/self-serve path. **The single most important guard is removing the authority-minting dev seams from prod** (R21). |
| **P13 / O1–O8** | No | No rider surface, no client-version gate, no manifest change. |

**No principle is violated.** One ADR-level decision (B1 routing) must be authored — see §11.

---

## 2. Personas

- **Maria / Daniel / Margaret / Tobias:** nothing changes (no rider/driver surface).
- **Sarah (admin):** everything that was "works on a local dev server" becomes "works at a real URL" —
  her passkey + session + the roster survive cold starts and live against Neon; a backend misconfig
  shows a calm error, never a fake roster. Must not regress the existing T10/T15 a11y (AC12).

---

## 3. Architecture: Option B1 (and the new ADR)

The web tier (a Cloudflare Worker via `@sveltejs/adapter-cloudflare`) calls the already-deployed Rust
Worker over HTTPS using the ADR-0026 BFF shared secret `ADMIN_API_SECRET`. Three persistence concerns
move/land:

1. **Member roster** — already code-complete (`WorkerMembersClient`); activated by setting
   `ADMIN_WORKER_BASE` + `ADMIN_API_SECRET` (fail-closed selector already enforced).
2. **WebAuthn invite + credential stores** — the web `InviteStore`/`CredentialStore` ports
   (`web/src/lib/server/webauthn/ports.ts`) get **Worker-backed adapters** that call new Rust Worker
   endpoints; the web tier never touches Postgres. The WebAuthn *ceremony* verification
   (`@simplewebauthn/server`) and the TTL/consumed *verdict* (`evaluateInvite`) stay edge-TS; the invite
   HMAC compare runs in the core.
3. **Admin session** — a new `KvSessionStore` over `ADMIN_SESSIONS` KV replaces the in-memory `Map`.

**New ADR to author during `/tasks` (task 1): `docs/adr/00NN-admin-web-b1-persistence-routing.md`.** It
records: the decision (WebAuthn invite/credential persistence behind server-to-server Worker endpoints,
web tier = zero Postgres); **why B1 over B2** (B2 would put the HMAC compare + credential CRUD on an
edge-TS Postgres path, widening the ADR-0017 P4 carve-out and giving the web tier a Group-scoped DB
credential — a bigger blast radius; B1 keeps one Postgres trust boundary, the `boundless_app` role, and
the HMAC compare in Rust); the route naming (NOT `/api/admin/auth/*`); that it **refines** ADR-0026
(extends the BFF surface) and ADR-0017 (the store moves behind the Worker; the ceremony + verdict stay
edge-TS, so the carve-out's *scope* is unchanged); and that invite-resolve is pre-session (shared-secret
gated, no `X-Admin-Id`).

---

## 4. Where each piece lives (file-by-file)

### Rust Worker — new admin endpoints
- **`server/src/runtime/admin_auth.rs`** (new) — handlers: `invite_resolve`, `invite_consume` (or a
  combined `complete_registration`, see §6/R11), `cred_list`, `cred_insert`, `cred_revoke_all`,
  `cred_bump_sign_count`. Mirrors `members.rs` structure (guard → store → value-free error bodies).
- **`server/src/runtime/mod.rs`** — register the new routes after the `/api/admin/members/*` block.
- **`server/src/runtime/members.rs`** — add a **pre-session `admin_guard` variant** (or a parameter):
  invite-resolve/consume require the shared secret but **not** `X-Admin-Id` (the admin is being
  registered; there is no acting admin id). The current `admin_guard` (members.rs:108–130) hard-requires
  `X-Admin-Id`.
- **`server/src/runtime/pg.rs`** — reuse `connect_pg`, `load_group_id`, `load_hmac_key`,
  `ensure_least_privilege`. Every new endpoint runs `ensure_least_privilege` via the same `build_*`
  assembly (R17) and scopes its txn by `GROUP_ID` (R16).

### Store — extend the existing admin-provisioning surface (don't build a parallel store)
- **`server/store/src/` (the module that holds `AdminProvisioningStore` + the admin-invitations/
  credentials methods, ~`lib.rs`)** — add the B1 read/CRUD methods that don't exist yet:
  `resolve_invitation_by_token` (compute the hash via the core, look up within the RLS-scoped txn,
  return the row or a value-free not-found — **no existence oracle**), atomic `consume_invitation`
  (conditional `UPDATE … WHERE consumed_at IS NULL RETURNING …`), `list_active_credentials`,
  `insert_credential`, `revoke_all_for_admin`, `bump_sign_count` (only-if-greater),
  `find_active_credential`. Reuse `create_pending_admin_with_invitation` / `reissue_admin_invitation`
  for the seed. All over the unnamed `query_typed*`/`execute_typed` family (ADR-0024), all inside a
  `set_config('app.current_group_id', $GROUP_ID, true)` transaction.
- The "complete registration" path (consume-invite + revoke-priors + insert-new) should be **one
  server-side transaction** (R11) — not three independent calls orchestrated from edge-TS, which cannot
  be made atomic there.

### Core
- `core/crypto/src/hashing.rs` — `admin_invitation_token_matches` already exists (constant-time). No
  change. The new Worker invite-resolve composes it; the token bytes never reach a log (P2).
- New small serde wire DTOs (`AdminInviteRecord`, `AdminCredential`, …) in `core/server` — **PII-free**,
  never reusing `MemberDetail`/`MemberSummary`/`DuplicatePhoneLink` (so the I5 negative gate is
  structural). Keyed-serde pinned like the member `*_wire_keys_are_pinned` tests.

### Web
- **`web/src/lib/server/webauthn/worker-stores.ts`** (new) — `WorkerInviteStore` + `WorkerCredentialStore`
  implementing the existing ports; mirror `WorkerMembersClient` (Bearer secret, base from
  `ADMIN_WORKER_BASE`, value-free `fail(res)`). **The invite token goes in the POST body to the Worker,
  not the URL** (R13). `public_key`/`credential_id`/`aaguid` cross as base64url.
- **`selectInviteStore` / `selectCredentialStore`** — fail-closed selectors mirroring
  `selectMembersClient`/`selectChallengeStore` (real adapter when configured; in-memory only in dev;
  else throw).
- **`web/src/lib/server/webauthn-deps.ts`** — swap `MemoryInviteStore`/`MemoryCredentialStore` for the
  selected stores. `register.ts`/`authenticate.ts` are **unchanged** (they depend only on the ports) —
  this preserves the ADR-0017 ceremony (R12).
- **`web/src/lib/server/session.ts`** — replace the in-memory `Map` with `KvSessionStore` over
  `ADMIN_SESSIONS` (pure-core + shell split, mirroring `kv-challenge-store.ts`). `create` mints a
  ≥128-bit opaque id (keep `crypto.randomUUID()`, R1), `put` with `expirationTtl` **and** an `expiresAt`
  in the value; `get` rejects `now >= expiresAt` (R2 — server-side TTL check, not just KV eviction);
  `revoke` deletes (best-effort within KV's window, D5/R3). `SESSION_TTL_SECS` short (ADR-0016). The
  authenticated session id is minted fresh post-assertion and is distinct from the ceremony cookie (R4).
  `createSession`/`getSession`/`requireAdminId` become async — **audit every call site** (grep
  `requireAdminId`/`getSession`/`createSession`).
- **`web/src/lib/server/log.ts`** (new) + **`web/src/hooks.server.ts`** — the `emit()` scrubber sink +
  `handleError` hook; no-raw-`console` lint over `web/src/**`.
- **`web/src/app.d.ts`** — superseded by `wrangler types`-generated `App.Platform` (adds
  `ADMIN_SESSIONS`); CI asserts the generated types match `wrangler.toml` (AC15, D6). **Fix the stale
  `wrangler.toml`/`app.d.ts` comments** that still describe a web-tier Hyperdrive binding for
  invite/credential stores — under B1 that's wrong (platform-parity F5).
- **`web/wrangler.toml`** — real `CHALLENGES` id, add `[[kv_namespaces]] ADMIN_SESSIONS`,
  `ADMIN_WORKER_BASE` + `WEBAUTHN_RP_ID/_ORIGIN/_RP_NAME` in `[vars]` (workers.dev for dev/test),
  **no secret in `[vars]`** (AC6), deploy-target keys.

### Scripts + runbook
- **`scripts/seed-admin-invite.sh`** + **`server/store/examples/seed_admin_invite_pg.rs`** (new) — mirror
  `bootstrap-group.sh` / `bootstrap_group_pg.rs`. Drive the existing
  `create_pending_admin_with_invitation` (member role=admin, NULL PII — schema confirmed permits it) and
  `reissue_admin_invitation` (idempotent re-run, R19). The owner URL via **env, not argv**; the minted
  token emitted **only to stdout** (examples are lint-exempt for output) and never as an argv or a
  structured log line (R20). `created_by` = a documented `"operator-seed"` sentinel, not NULL (R18).
- **`docs/runbooks/deploy-admin-web.md`** (new) — mirror `deploy-worker.md`: KV create ×2, secrets
  (`ADMIN_API_SECRET` byte-identical to the Rust Worker's, can't be read back — R6), the WebAuthn domain
  config (workers.dev + custom-domain, RP_ID-is-permanent warning, D7), the operator first-admin seed,
  `wrangler deploy`, the smoke.
- **`scripts/smoke-deployed-admin-web.sh`** (new) — mirror `smoke-deployed-edge.sh`: seed invite →
  register passkey → sign-in → list/issue → sign-out → revoked-cookie redirect → `Referrer-Policy` +
  RP_ID-not-`localhost` assertions → the opt-in ≥2-Group cross-tenant block.

### Dev seams (§F)
- **Default the two authority-minting seams to delete** (`seed-session`, `seed-invite`) — a reachable
  one in prod is a direct I11 bypass (R21); drive the e2e via the Playwright virtual authenticator +
  the operator seed against a dev backend. `reset`/`seed-member` may be **repointed** to the dev-durable
  backends to keep the T10/T15 suites green (F12) — behind the `dev`-inlined-`false` gate, targeting a
  dev-only namespace, never the production `ADMIN_SESSIONS`/Worker. The build-artifact test (AC5) proves
  all are tree-shaken from prod.

---

## 5. RLS plumbing (D3)

Every new endpoint scopes DB access by the Worker's `GROUP_ID` binding (never a client/token value),
inside `set_config('app.current_group_id', $1, true)` (the transaction-local parameterised form; `SET
LOCAL` can't bind a parameter) — identical to `PgMemberStore`/`PgAuthStore`. **Invite-resolve is the
subtle one:** it runs pre-session, so the group comes from `GROUP_ID`, and the token is matched
constant-time *within* the already-scoped txn — a cross-tenant invite is invisible (AC14). The
`ensure_least_privilege` boot guard (non-superuser/non-`BYPASSRLS`) runs per-request via the shared
`build_*` path (R17).

---

## 6. Data model, migrations & API surface

### Migrations — **none (D4 confirmed)**
Tables `0007_admin_webauthn_credentials` and `0008_admin_invitations` are sufficient (RLS policies,
`one_live_per_admin` partial-unique, `credential_id` unique, the bytea columns). **§D schema check
confirmed satisfied:** `members` permits an `admin`-role row with NULL `name_encrypted`/`address_encrypted`
(`roles`/`phone_lookup_hash` nullable in `0002`; name/address are nullable ADD COLUMNs in `0010`).

### API surface — new B1 endpoints (AC13, D2)
**Naming (resolves the F1 collision):** the new server-to-server persistence routes must NOT reuse
`/api/admin/auth/*` (which already denotes the EDGE browser ceremony in the frozen contract).
**Recommended prefix: `/api/admin/webauthn/*`** (reads as "the WebAuthn store surface," one prefix to
gate; the architect's alternative `/api/admin/{invitations,credentials}/*` is equally acceptable —
finalize in the contract task):

| Method + path | Purpose | Auth | Notes |
|---|---|---|---|
| `POST /api/admin/webauthn/invite/resolve` | Resolve a presented token → pending-admin row (core HMAC compare) | `adminSharedSecret` **only** (pre-session, **no** `AdminIdHeader`) | token in **body**, not URL (R13); value-free 404 on no match (no oracle) |
| `POST /api/admin/webauthn/invite/consume` | Single-use stamp (atomic) | `adminSharedSecret` only | conditional `UPDATE … WHERE consumed_at IS NULL` |
| `POST /api/admin/webauthn/register-complete` *(recommended combined op, R11)* | consume-invite + revoke-priors + insert-credential in **one txn** | `adminSharedSecret` only | avoids edge-TS orchestrating a 3-step invariant it can't make atomic |
| `GET /api/admin/webauthn/credentials?admin_id=` | List active credentials | `adminSharedSecret` + `AdminIdHeader` | sign-in lookup |
| `POST /api/admin/webauthn/credentials/{id}/sign-count` | Bump sign-count (only-if-greater) | `adminSharedSecret` + `AdminIdHeader` | clone-detection backstop (R10) |

> If a combined `register-complete` is used, the standalone `consume` + `cred_insert` + `revoke_all` may
> be internal store methods rather than separate wire ops. Decide in the contract task; the atomicity
> (R11) is the hard requirement, not the op count.

- **Security scheme:** reuse the existing `adminSharedSecret` + `AdminIdHeader` components. **New, PII-free
  schemas** (`AdminInviteRecord {admin_id, group_id, expires_at, consumed_at}`, `AdminCredential
  {credential_id, admin_id, public_key, sign_count, transports?, aaguid?, revoked_at}`) — never reusing
  member schemas.
- **Error codes:** reuse `ADMIN_UNAUTHORIZED` / `ADMIN_BAD_REQUEST`; the invite TTL/consumed verdict stays
  edge-TS (`ADMIN_INVITE_EXPIRED` / `ADMIN_INVITE_CONSUMED`, already registered). **No new error code
  expected** — confirm at the registry task.
- **Proto:** N/A (HTTP BFF routes).
- **Contract-freeze (AC13):** add the ops + schemas to `api/openapi.yaml`; extend
  `web/tests/contract/api-contract.test.ts` with a **dedicated B1 describe-block** — the existing
  `members`-scoped gates do NOT cover a new prefix. Four assertions: (1) the B1 ops exist + are frozen
  (non-vacuity); (2) `adminSharedSecret` on every B1 op; (3) `AdminIdHeader` **only** on the
  session-bearing ops, **absent** on invite-resolve/consume (positive AND negative); (4) **no B1 response
  schema reaches a member-PII schema** (the I5 negative gate, F2).

---

## 7. Security risk register (condensed — full register in the security-auditor pass)

**Must-fix in this spec (design/tests land here):**
- **R2** — server-side session TTL value check (not just KV eviction).
- **R4** — no session fixation: fresh authenticated id post-assertion, distinct from the ceremony cookie.
- **R10** — sign-count bump persists over B1 (only-if-greater); register→assert→count-advanced PG test.
- **R11** — consume-invite + revoke-priors + insert-credential as **one server-side txn** (combined op).
- **R13** — invite token off URL/log/Referer on both paths; **POST the token in the body** on the
  BFF→Worker hop; `Referrer-Policy: no-referrer` on the registration route; I10 fixture.
- **R15** — concurrent-consume atomicity (two concurrent consumes → exactly one wins) PG test.
- **R16** — invite-resolve scoped by `GROUP_ID`, never the token; AC14 cross-tenant probe on the new ops.
- **R17** — every new endpoint runs `ensure_least_privilege` (shared `build_*` path).
- **R19** — seed reuses the core `create_pending_admin_with_invitation`/`reissue_admin_invitation` (proven
  supersede-then-insert); double-run meta-test.
- **R20** — seed token emitted only to stdout; owner URL via env, not argv; no token in any log line.
- **R21** — delete the authority-minting dev seams (`seed-session`/`seed-invite`); tree-shake-prove the
  rest (AC5).
- **R22** — the web `emit()` sink + `handleError` + no-`console` lint + AC8 fixtures (the slice's I10
  deliverable).
- **R1/R8** — keep ≥128-bit opaque session id; no secret in `[vars]` (gate); sink fixture proves the
  fail-closed throw carries no secret substring.

**Acceptable-as-deferred (with WHEN):**
- **R3** — KV revocation best-effort within the propagation window (documented, D5). WHEN: if a
  Worker-native admin session / instant global revoke is wanted (ADR-0026).
- **R5** — no admin-session device-binding (the deliberate ADR-0016 laptop model). WHEN: bulk-revoke need.
- **R7** — the Worker doesn't verify the asserted `X-Admin-Id` is a real admin (a leaked secret could
  forge the I5 audit actor). Pre-existing, tracked (DEFERRED spec 008 T09); the new endpoints inherit the
  same gate, no new exposure. WHEN: the security-hardening pass with the real BFF call.
- **R18** — seed `created_by` is an `"operator-seed"` sentinel, not the Developer identity. WHEN: the
  Developer-minting UI (spec 001 T08-shell).

**Confirmed clean:** R9 (COSE bytea not PII), R12 (UV enforcement stays edge-TS; store move is
adapter-only), R14 (core HMAC compare constant-time).

---

## 8. Test strategy (AC → level; condensed — full matrix in the test-strategist pass)

`(edge)` = operator-gated/live-only; everything else is locally green-able, with a local proxy for each
edge AC.

| AC | Level / file | Notes |
|---|---|---|
| AC1 | Unit — `web/.../members.test.ts` (existing selector block) | re-confirm fail-closed |
| AC2 | Integration miniflare KV — `web/.../kv-admin-session-store.test.ts` (new) | cold start = fresh store instance over the same persisted KV; TTL via injected `Clock`; revoke; selector fail-closed |
| AC3 | Integration real-PG + e2e — `server/store/tests/admin_webauthn.rs` + extend `web/tests/e2e/webauthn.spec.ts` | fresh store instance reads the persisted credential; virtual-authenticator ceremony |
| AC4a/AC4b | Real-PG + miniflare Worker + web unit — `admin_webauthn.rs`, `server/test/admin-webauthn.spec.ts`, `production-invite-store-is-worker-backed.test.ts` | single-use + concurrent consume; resolve routes through `admin_invitation_token_matches`; prod InviteStore is Worker-backed; a `grep` lint forbids `hmac`/`subtle.sign` in `web/src/lib/server/webauthn/**` |
| AC5 | Build-artifact + (edge) probe — `web/tests/build/no-dev-seams.test.ts` | `dev` inlined false; deployed 404 |
| AC6 | CI gate — credential-scan over `web/wrangler.toml` | |
| AC7 | Script meta-test real-PG — `server/store/examples/seed_admin_invite_pg.rs` + meta-test | null-PII admin + invite; idempotent re-run |
| AC8 | Unit scrubber + miniflare — `web/.../web-emit-scrubber.test.ts` + `admin-webauthn.spec.ts` | operator strings + URL-embedded token redacted on both paths |
| **AC9** | **(edge)** | `pnpm build` local; `wrangler deploy` live |
| **AC10** | **(edge)** — `scripts/smoke-deployed-admin-web.sh` | full path proven locally by AC2+AC3+AC4+AC14; smoke is the live integration |
| **AC11** | **(edge)** + unit `rp-config.test.ts` | env-driven RP config local; ≠`localhost`/HTTPS/`Referrer-Policy` live |
| AC12 | e2e+axe (existing) + catalog-parity | the T10/T15 axe runs stay green; no new hardcoded string |
| AC13 | Contract — extend `web/tests/contract/api-contract.test.ts` | the B1 describe-block (§6) |
| AC14 | Real-PG + miniflare + (edge) — `admin_webauthn.rs` + `server/test/admin-webauthn-cross-tenant.spec.ts` | Group-B invite/credential invisible to a Group-A-scoped store/Worker; live ≥2-Group probe in the smoke |
| AC15 | CI gate — `web/tests/build/wrangler-types-match.test.ts` | regen `wrangler types` → diff vs committed |

**F12 regression gate:** run the existing `admin-onboarding.spec.ts` + `admin-members.spec.ts` after the
wiring — they are the AC12 "unchanged-and-still-green" evidence and the F12 proof in one.

New fixtures: `fixtures/admin-auth/invite_{live,expired,consumed}.json`; a Group-B admin invite +
credential in `setup-worker-test-db.sh` (the AC14 seed); the web scrubber log-line fixtures.

---

## 9. Dependency-ordered sequencing (the task spine for `/tasks`)

Locally-testable first; operator/edge-gated last. (✓ = local; **(edge)** = live-only.)

1. **ADR (B1 routing)** — `docs/adr/00NN-…`. Blocks the contract task.
2. **Store B1 methods** — extend the admin-provisioning store: `resolve_invitation_by_token`, atomic
   `consume`, credential CRUD, `register-complete` txn. Real-PG18 tests (consume single-use + concurrent;
   resolve-through-core; persist-across-fresh-store; tenant isolation). (AC4, AC14 local.)
3. **OpenAPI freeze** — add the B1 ops + schemas; extend `api-contract.test.ts` (the B1 describe-block).
   Depends on (1) for names. (AC13.)
4. **Worker B1 routes** — `admin_auth.rs` + `mod.rs` + the pre-session `admin_guard` variant; miniflare+PG
   round-trips + cross-tenant probe + invite-resolve-no-token-in-log. Depends on (2)+(3). (AC4, AC8, AC14.)
5. **Web Worker-backed stores + selectors** — `worker-stores.ts`; unit tests (adapter request shapes;
   prod-store-is-Worker-backed; no-`hmac`-in-edge-TS lint). Depends on (3). (AC1, AC4b.)
6. **Web KV session store + selector** — `session.ts` rewrite (async ripple, call-site audit); miniflare-KV
   tests (cold-start/TTL/revoke/fixation/entropy). Parallel with (5). (AC2, R1/R2/R4.)
7. **`webauthn-deps.ts` swap + dev-seam decision** — delete `seed-session`/`seed-invite`; repoint
   `reset`/`seed-member` to dev-durable; keep T10/T15 green; build-artifact tree-shake test. Depends on
   (5)+(6). (AC5, F12.)
8. **Web scrubbed logging** — `log.ts`, `hooks.server.ts`, no-`console` lint, I10 fixtures. (AC8.) Parallel.
9. **`wrangler.toml` + `wrangler types` drift CI + stale-comment fix.** (AC6, AC15.)
10. **Operator seed** — `seed-admin-invite.sh` + `seed_admin_invite_pg.rs` (reuse core methods); meta-test.
    (AC7.)
11. **Runbook** — `deploy-admin-web.md`. (P12.)
12. **Smoke script** — `smoke-deployed-admin-web.sh` (written local, run live).
13. **(edge)** `pnpm build` + `wrangler deploy` → live smoke + ≥2-Group cross-tenant probe. (AC9–AC11,
    AC14 edge leg.)

---

## 10. Open decisions (recommendations; none blocks `/tasks`)

1. **Route prefix** — `/api/admin/webauthn/*` (recommended) vs `/api/admin/{invitations,credentials}/*`.
   Pick once in the contract task (the freeze gate keys off the prefix).
2. **Combined `register-complete` op vs three CRUD ops** — recommend the combined op for the atomicity
   (R11); the standalone consume/insert/revoke become internal store methods.
3. **Dev-seam delete-vs-repoint** — recommend **delete** the two authority-minting seams (`seed-session`,
   `seed-invite`); repoint `reset`/`seed-member` to dev-durable. (Security-cleaner than repointing the
   authority seams.)
4. **`created_by` sentinel** — recommend a documented `"operator-seed"` reserved UUID over NULL (R18).

All four are technical and aligned with the subagent recommendations; the user trusts the call. Surface
them in `/tasks` for confirmation, but proceed on the recommendations absent objection.

---

## 11. ADRs & DEFERRED updates

- **New ADR** (task 1): `docs/adr/00NN-admin-web-b1-persistence-routing.md` (§3).
- **DEFERRED.md** additions (WHEN triggers): R7 (verify `X-Admin-Id` is a real admin — security-hardening
  pass / the real BFF call); R18 (seed `created_by` = Developer identity — spec 001 T08-shell minting UI);
  R3/R5 (KV revocation window / admin-session device-binding — if a Worker-native session is wanted); the
  custom-domain RP_ID cutover (D7 — a documented operational event); fold the seed's argv-secret class
  into the existing deploy-hardening item.
- **No conflict** with `architecture.md`, the constitution, the glossary, or ADR-0015/0016/0017/0024/0025/
  0026. B1 keeps "web routes hit the same Workers API as the apps" (web → Worker, not Postgres), which is
  exactly what ADR-0026 + the architecture intend.

---

## 12. File index

**New:** `docs/adr/00NN-admin-web-b1-persistence-routing.md` · `server/src/runtime/admin_auth.rs` ·
`web/src/lib/server/webauthn/worker-stores.ts` · `web/src/lib/server/log.ts` ·
`scripts/seed-admin-invite.sh` · `server/store/examples/seed_admin_invite_pg.rs` ·
`docs/runbooks/deploy-admin-web.md` · `scripts/smoke-deployed-admin-web.sh` ·
test files: `web/src/lib/server/kv-admin-session-store.test.ts`,
`web/src/lib/server/webauthn/production-invite-store-is-worker-backed.test.ts`,
`web/src/lib/server/webauthn/rp-config.test.ts`, `web/src/lib/server/web-emit-scrubber.test.ts`,
`web/tests/build/{no-dev-seams,wrangler-types-match}.test.ts`,
`server/store/tests/admin_webauthn.rs`, `server/test/admin-webauthn{,-cross-tenant}.spec.ts` ·
`fixtures/admin-auth/invite_{live,expired,consumed}.json`.

**Edited:** `server/src/runtime/{mod.rs,members.rs,pg.rs}` · `server/store/src/lib.rs` (B1 store methods)
· `core/server` (PII-free wire DTOs) · `api/openapi.yaml` · `web/tests/contract/api-contract.test.ts` ·
`web/src/lib/server/{session.ts,webauthn-deps.ts}` · `web/src/hooks.server.ts` · `web/src/app.d.ts` ·
`web/wrangler.toml` · `web/src/routes/api/test/*` · `scripts/setup-worker-test-db.sh` · `DEFERRED.md` ·
`docs/error-codes.md` (only if a new code proves necessary).

**Unchanged (deliberately):** `web/src/lib/server/webauthn/{register.ts,authenticate.ts}` (the ceremony —
ports-only dependency, ADR-0017 carve-out preserved) · `core/crypto/src/hashing.rs` · the migrations ·
the `/api/admin/members/*` + `/api/auth/*` frozen contract shapes.
