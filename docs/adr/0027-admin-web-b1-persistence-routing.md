# ADR-0027: Admin-web WebAuthn invite/credential persistence routes behind the Rust Worker (Option B1)

- **Status:** Accepted
- **Date:** 2026-06-13
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0026 (admin→Worker shared-secret BFF gate — **refined/extended here**), ADR-0017 (admin WebAuthn on the SvelteKit edge — **refined here**), ADR-0015 (admin invitation channel), ADR-0024 (Hyperdrive unnamed statements), ADR-0025 (per-Group key lifecycle); I5 (admin PII reads audit-logged), I10/P2 (no PII in logs), I11 (admins issued only by the Developer), P4 (Rust core is the source of truth); spec 009 (deploy the admin web dashboard), decisions D1/D2/D3.

## Context

The Rust admin Worker is live on the edge: `/api/admin/members/*` + `/api/admin/audit-log`, Hyperdrive → Neon, cross-tenant isolation proven (spec 008 T11/AC16). It is reached by the SvelteKit admin tier over the **ADR-0026 shared-secret BFF gate** (`ADMIN_API_SECRET` + the asserted `X-Admin-Id`; group from the single-install `GROUP_ID`).

Its other half — the SvelteKit admin dashboard in `web/` — still runs against **in-memory fakes** for three persistence concerns: the admin **session** (an in-memory `Map`), and the WebAuthn **invite** and **credential** stores (`MemoryInviteStore` / `MemoryCredentialStore`). Spec 009 deploys the dashboard and makes those three durable so a real Admin (Sarah) can register a passkey, sign in, and manage members against live Neon, with her session/passkey/invite surviving Worker cold starts.

The session store is uncontroversial (a new `ADMIN_SESSIONS` KV — spec 009 D5). The **open architectural question** is *where the WebAuthn invite + credential persistence lives*, and it sits on a fault line between two existing ADRs:

- **ADR-0017** put admin WebAuthn registration + assertion *verification* in the **SvelteKit edge** (`@simplewebauthn/server`), against "our own `admin_webauthn_credentials` store" — a documented P4 carve-out (WebAuthn is a web-platform protocol, no wasm-compatible Rust RP verifier exists, the admin surface is web-only). At the time, "our own store" was the web tier's in-memory fake; *how* it would become durable was left to deployment.
- **ADR-0026** then established the **Rust Worker as the single Postgres trust boundary** for the admin surface: the web tier talks to the Worker over the shared secret, never to Postgres directly, so the per-Group crypto + validation stay single-sourced in Rust (P4) and there is exactly one DB credential (`boundless_app`, non-superuser, `FORCE ROW LEVEL SECURITY`).

Making the invite/credential stores durable forces the question those two ADRs left open: **does the web tier get its own Group-scoped Postgres credential to persist invites/credentials in edge-TS (B2), or does that persistence move behind new server-to-server Worker endpoints (B1)?** The invite-token **HMAC compare** (`core::crypto::admin_invitation_token_matches`, constant-time, domain-tagged) is the sharpest version of the question: under B2 it would have to run in edge-TS (re-implemented or FFI'd); under B1 it stays in the Rust core.

Getting this wrong is high-impact: B2 would add a *second* Postgres trust boundary on the admin path and pull a privacy-relevant constant-time compare into a non-Rust runtime — exactly the duplication ADR-0026 and P4 exist to prevent.

## Decision

**Adopt Option B1: the WebAuthn invite + credential persistence moves behind new server-to-server endpoints on the Rust Worker. The web tier keeps zero direct Postgres access.** The new endpoints ride the same ADR-0026 BFF surface (the `ADMIN_API_SECRET` shared secret; group from `GROUP_ID`; RLS-scoped, fail-closed). The invite-token HMAC compare runs in the Rust **core** as part of the Worker's invite-resolve; the WebAuthn **ceremony verification** and the TTL/consumed **verdict** (`evaluateInvite`) stay in edge-TS (ADR-0017's carve-out, scope unchanged).

This ADR **refines, and does not supersede,** ADR-0026 and ADR-0017:

- **Extends ADR-0026's BFF surface** with a new namespace of server-to-server admin endpoints under the same shared-secret trust model.
- **Refines ADR-0017** by locating "our own credential store" *behind the Worker* (Postgres, via the existing `boundless_app` role) rather than in the web tier. The carve-out's **scope is unchanged**: the ceremony + the TTL/consumed verdict remain edge-TS; only the *store* moves. Nothing new enters or leaves `core::auth`.

### Route naming (finalized — T03 keys off this)

The new routes **must not reuse `/api/admin/auth/*`**, which is already frozen in `api/openapi.yaml` as the **browser↔edge ceremony** (`GET /api/admin/auth/invite/{token}`, `POST /api/admin/auth/register`, `POST /api/admin/auth/signin` — edge-TS-verified, browser-facing, **no** shared secret). Those are a different surface (browser ↔ SvelteKit) from the new one (SvelteKit ↔ Rust Worker).

**The new server-to-server persistence prefix is `/api/admin/webauthn/*`.** It reads as "the WebAuthn store surface," is a single prefix to gate, and does not collide with the frozen contract. (The alternative `/api/admin/{invitations,credentials}/*` — two prefixes — was considered and rejected as harder to gate as one unit; see Alternatives.)

### Endpoint shape (the freeze target for T03)

| Method + path | Purpose | Auth |
|---|---|---|
| `POST /api/admin/webauthn/invite/resolve` | Resolve a presented token → pending-admin invite metadata (the **core** HMAC compare runs here). Read-only; edge-TS then applies the TTL/consumed verdict. | `adminSharedSecret` **only** (pre-session) |
| `POST /api/admin/webauthn/register-complete` | **One server-side transaction**: re-resolve + single-use consume + revoke-prior-credentials + insert the new credential (R11). | `adminSharedSecret` **only** (pre-session) |
| `POST /api/admin/webauthn/credentials/lookup` | Resolve a presented `credential_id` → the active credential (admin id, COSE public key, sign-count) for assertion verification at sign-in. | `adminSharedSecret` **only** (pre-session) |
| `POST /api/admin/webauthn/credentials/{id}/sign-count` | Bump the stored sign-count, only-if-greater (clone-detection backstop), during assertion verification. | `adminSharedSecret` **only** (pre-session) |

The sign-in credential lookup is keyed by the presented **`credential_id`**, not an admin id: the admin assertion is usernameless/discoverable (`buildAuthenticationOptions` supplies no `adminId`), so no admin id is known until the credential resolves (the admin id is read *off* the resolved credential). This is why `credentials/lookup` is pre-session and shared-secret-only — it **supersedes** the plan §6 draft's `GET …/credentials?admin_id=` + `AdminIdHeader` row, which assumed an admin id available at sign-in. A `credentials?admin_id=` *list* (with `AdminIdHeader`) is needed only by the authenticated backup-key flow (out of scope here).

Two decisions are pinned in this shape, both because edge-TS cannot enforce them itself:

1. **A combined `register-complete`, not three edge-orchestrated calls.** Consume-invite + revoke-priors + insert-credential is a three-statement invariant that **must be atomic** (R11). Edge-TS cannot make three independent Worker calls atomic; one server-side transaction can. The standalone consume / insert / revoke-all therefore become **internal store methods**, not separate wire ops. (`revoke-all-for-admin` as a standalone wire op, and a `credentials?admin_id=` *list*, are only needed by the **authenticated backup-key enrollment** flow, which is out of scope for spec 009 — deferred, spec 001 T15 register. T03 may freeze them now as session-bearing stubs or defer them; the atomicity of `register-complete` is the hard requirement, not the op count.)

2. **The pre-session carve-out.** Every B1 op in spec 009's scope runs **before** a verified admin session exists — the admin is being registered (`invite/resolve`, `register-complete`) or is being authenticated (`credentials/lookup`, `sign-count` bump happen *during* assertion verification). So they require the shared secret but carry **no `X-Admin-Id`** — there is no verified acting admin to assert. This is the deliberate difference from the `/api/admin/members/*` ops, which run *with* a verified session and therefore carry both the secret **and** `X-Admin-Id` (the I5 audit actor, ADR-0026). Any future *session-bearing* B1 op (authenticated backup-key enrollment) would carry both, like the member ops.

### Invariants the new surface inherits / states

- **RLS by `GROUP_ID`, never the token (D3, R16).** Every endpoint runs inside a `SELECT set_config('app.current_group_id', $1, true)` transaction — the group bound as the SQL positional parameter `$1` from the Worker's single-install `GROUP_ID` binding (the parameterised transaction-local form; `SET LOCAL` cannot bind a parameter). This is exactly how `PgMemberStore`/`PgAuthStore` already scope every query. **Invite-resolve is the subtle case:** it runs pre-session, so the group comes from `GROUP_ID` and the presented token is matched constant-time *within* the already-scoped transaction — a cross-tenant invite is invisible (AC14). The `ensure_least_privilege` boot guard runs per request via the shared assembly path (R17).
- **HMAC compare stays in the core (P4, AC4b).** `admin_invitation_token_matches` (constant-time, domain-tagged) is composed by `invite/resolve`; it is never re-implemented in edge-TS. This is the whole point of B1 over B2.
- **Token off every log path (R13, I10/P2, AC8).** The presented invite token arrives in the **POST body**, never a URL path or query (contrast the *pre-existing* browser route `GET /api/admin/auth/invite/{token}`, whose token-in-URL leak is separately tracked). The resolve/consume error paths emit value-free error codes only — never the token, never an existence oracle (a no-match returns the same value-free not-found shape as a wrong-group token).
- **Not an audited read (I5, D2/AC13).** These endpoints disclose admin credential/invite **metadata** (admin id, expiry, consumed-at, COSE public key, sign-count) — **not member PII**. They are therefore **not** `x-requires-audit`, unlike the member ops. The PII-free wire DTOs (`AdminInviteRecord`, `AdminCredential`) live in `core/server` and never reuse `MemberDetail`/`MemberSummary`/`DuplicatePhoneLink`, so the contract-freeze test can assert the I5 negative gate structurally (no B1 response schema reaches a member-PII schema).
- **No new migration (D4).** Tables `0007_admin_webauthn_credentials` and `0008_admin_invitations` already carry the columns, the `admin_invitations_one_live_per_admin` partial-unique index, the `credential_id` unique constraint, the `bytea` columns, and the RLS policies. B1 reuses the existing `AdminProvisioningStore` methods (`create_pending_admin_with_invitation`, `reissue_admin_invitation`) and adds read/CRUD methods alongside them — it does not build a parallel store.

## Considered alternatives

### Option A (chosen) — B1: persistence behind new Rust Worker endpoints; web tier = zero Postgres

**Pros:**
- One Postgres trust boundary on the admin path (the `boundless_app` role), consistent with ADR-0026 and "web routes hit the same Workers API as the apps."
- The invite-token HMAC compare stays in Rust (P4); no privacy-relevant constant-time compare in a non-Rust runtime.
- Per-Group crypto + RLS stay single-sourced in the Worker; the web tier holds no DB credential and no `GroupKey` (ADR-0025).
- The combined `register-complete` makes the consume+revoke+insert invariant atomic server-side (R11) — impossible to guarantee from edge-TS.
- ADR-0017's carve-out scope is *narrowed in blast radius* (store moves to Rust), not widened.

**Cons:**
- Adds new Worker endpoints + Worker-backed web adapters (more code than reusing an in-memory port).
- An extra network hop (SvelteKit Worker → Rust Worker) on registration/sign-in — acceptable for a low-RPS management plane.
- KV-based admin sessions keep best-effort revocation within KV's propagation window (D5/R3) — but that is the session decision, independent of B1.

### Option B — B2: a web-tier Hyperdrive/Postgres binding; invite/credential CRUD + HMAC compare in edge-TS

**Pros:**
- Fewer Worker endpoints; the web tier owns its WebAuthn store end-to-end.
- No extra SvelteKit→Worker hop for invite/credential work.

**Cons:**
- **A second Postgres trust boundary** on the admin path — a Group-scoped DB credential held by the web Worker. Larger blast radius; two roles/RLS surfaces to keep correct instead of one.
- **Pulls the invite-token HMAC compare into edge-TS** — re-implementing (or FFI-binding) a constant-time, domain-tagged keyed compare outside the Rust core. Widens the ADR-0017 P4 carve-out from "ceremony + verdict" to "ceremony + verdict + a privacy-relevant crypto compare + credential CRUD." Directly against P4 and the reason ADR-0026 exists.
- Splits the per-Group crypto story across two runtimes. **Rejected.**

### Option C — keep the web-tier in-memory stores; no durable invite/credential persistence

**Pros:** zero new endpoints; smallest diff.

**Cons:** the registered passkey and pending invite are lost on every Worker cold start — the dashboard is not actually usable as a deployment. This is precisely the gap spec 009 exists to close (AC3/AC4). **Rejected** (non-viable).

### Sub-alternative (route naming) — `/api/admin/{invitations,credentials}/*` (two prefixes)

The architect's equally-valid option. **Rejected** in favour of the single `/api/admin/webauthn/*` prefix: one namespace is one thing to gate, name, and reason about as "the WebAuthn store surface," and it keeps the per-prefix contract assertions in T03 simple. Recorded here so the choice is explicit rather than implicit.

## Consequences

### Positive
- The admin surface keeps **one** Postgres trust boundary and **one** place the per-Group crypto + HMAC compare run (the Rust core/Worker). P4 intact; ADR-0026's "web → Worker, never Postgres" holds for the whole admin surface, not just member management.
- The `register-complete` transaction makes the single-use + revoke-priors + insert invariant atomic, closing the TOCTOU window edge-TS orchestration would have left open (R11/R15).
- The contract-freeze I5 negative gate becomes **structural** (separate PII-free DTOs), not a reviewer's vigilance.

### Negative / costs
- New endpoints + adapters + a pre-session `admin_guard` variant (the current `admin_guard` hard-requires `X-Admin-Id`; pre-session ops need a shared-secret-only variant). More surface than reusing the in-memory port.
- One extra network hop on registration/sign-in (low-RPS plane; acceptable).
- The blast radius of a leaked `ADMIN_API_SECRET` now also covers the WebAuthn store, not only member management — but it was already the whole admin surface under ADR-0026, so this is not a new class of exposure. The deferred defense-in-depth (verify the asserted `X-Admin-Id` is a real admin, ADR-0026 / DEFERRED R7) does not apply to the *pre-session* B1 ops, which carry no `X-Admin-Id` to verify.

### Neutral / follow-ups
- The **exact** op list + per-op request/response shapes + which session-bearing credential ops (the authenticated backup-key flow) ship now vs. defer are finalized in the contract task (T03); this ADR fixes the prefix, the combined-`register-complete` shape, and the pre-session auth carve-out — the parts T03 builds on.
- The custom-domain WebAuthn `RP_ID` cutover (spec 009 D7) is an operational event, not an architecture change; tracked in `DEFERRED.md`.
- The KV admin-session revocation window (D5/R3) and the seed `created_by` = Developer-identity gap (R18) are recorded in `DEFERRED.md` with WHEN triggers; neither is a B1-routing concern.

## Compliance

- **Constitution change?** No. P4 is *upheld* (the HMAC compare stays in the core; nothing new leaves `core::auth`); ADR-0017's carve-out scope is unchanged. No principle is amended, so no constitution version bump.
- **Stack-matrix change?** No. No new dependency: the new endpoints reuse `tokio-postgres`'s unnamed-statement family (ADR-0024), the existing `boundless_app` role, the existing `adminSharedSecret`/`AdminIdHeader` OpenAPI components, and the already-used `ADMIN_SESSIONS`-class KV. A new contract *surface* is added to `api/openapi.yaml` (T03), but the stack matrix does not enumerate routes.
- **Migration of existing code?** No schema migration (D4 — tables `0007`/`0008` suffice). Code changes are additive (new store methods, new Worker routes, new web adapters); the frozen `/api/admin/auth/*` ceremony shapes and `/api/admin/members/*` are untouched. The web ceremony files (`register.ts`/`authenticate.ts`) are unchanged — they depend only on the ports, so swapping the in-memory store for a Worker-backed adapter preserves ADR-0017's ceremony (R12).

## References

- `specs/009-admin-web-deploy/spec.md` — D1 (B1 routing), D2 (frozen contract), D3 (RLS by `GROUP_ID`), D4 (no migration); AC4a/AC4b/AC13/AC14.
- `specs/009-admin-web-deploy/plan.md` §3 (architecture), §6 (API surface), §10 (open decisions 1–2).
- ADR-0026 — admin→Worker shared-secret BFF gate (the surface this extends).
- ADR-0017 — admin WebAuthn on the edge; the P4 carve-out (whose scope is unchanged here).
- ADR-0015 — admin invitation channel (Developer-initiated; the opaque single-use token).
- ADR-0024 — Hyperdrive unnamed `query_typed*` statements (the store methods' execution path).
- ADR-0025 — per-Group key lifecycle (the web tier never holds the `GroupKey`).
- `docs/privacy-invariants.md` — I5 (audited reads), I10 (scrubbed logs), I11 (Developer-issued admins).
- `core/crypto/src/hashing.rs` — `admin_invitation_token_matches` (the constant-time compare composed by invite-resolve).
- `api/openapi.yaml` — the frozen `/api/admin/auth/*` (browser ceremony) and `/api/admin/members/*` (the BFF pattern the new prefix mirrors).
