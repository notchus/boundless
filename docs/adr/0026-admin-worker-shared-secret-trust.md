# ADR-0026: Admin→Worker trust for `/api/admin/*` is a shared-secret BFF gate + asserted admin id

- **Status:** Accepted
- **Date:** 2026-06-11
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0017 (admin WebAuthn on the SvelteKit edge), ADR-0015 (admin provisioning), ADR-0025 (per-Group key); I5 (admin PII reads audit-logged), I11 (no admin self-onboarding), P2/P4; spec 008 T09 (the deployable `/api/admin/members/*` Worker endpoints)

## Context

Spec 008 adds the admin member-management surface — `/api/admin/members/*` + `/api/admin/audit-log`.
The architecture (and the spec-008 plan §1 tension-1 resolution) is explicit that **the SvelteKit
admin tier calls the Rust Worker**, which composes the core `MemberService` (P4 — issuance crypto +
validation single-sourced in Rust; the SvelteKit routes own only session/cookie + presentation).

But **how** an admin request is *trusted* at the Worker was left open:

- **ADR-0017** put admin WebAuthn registration + assertion verification in the **SvelteKit edge**
  (`@simplewebauthn/server`), not the Worker. The admin's identity + credentials live in the web
  tier's own store; the Worker has **no admin-session notion** and there is no SvelteKit↔Worker
  channel today (the web tier talks to KV/Postgres directly for auth).
- The Worker's `/api/admin/members/*` endpoints need two trusted facts the Worker cannot itself
  establish: (a) the acting **`admin_id`** (a `MemberId`) for the I5 `audit_log.admin_id` /
  `created_by` actor, and (b) the **`group_id`** for the RLS `set_config(app.current_group_id, …)`
  tenant scoping.

Getting this wrong is the single highest-impact way the admin surface fails: an endpoint that does
real PII work (decrypt address/phone, mint Onboarding Codes) **without** a fail-closed auth gate is a
direct breach of the closed-group model (I11) and the audited-read invariant (I5).

## Decision

The Worker's `/api/admin/*` surface trusts a **shared-secret backend-for-frontend (BFF) gate**:

1. **Shared secret.** The WebAuthn-verified SvelteKit BFF presents a server-to-server bearer secret
   `ADMIN_API_SECRET` (an `Authorization: Bearer <secret>` header). The Worker compares it in
   **constant time** and **fails closed** without it (`401`); a missing `ADMIN_API_SECRET` binding is a
   `500` (misconfigured deploy). The secret is a `wrangler secret` at deploy and a test value injected
   by the miniflare harness — never committed (the `check-wrangler-credentials.sh` gate forbids a real
   value landing in `wrangler.toml`). It is **not** a client-facing credential: only the trusted admin
   BFF holds it.
2. **Asserted admin id.** The verified acting admin's id rides in the `X-Admin-Id` header. The Worker
   trusts it **because** the request carried the shared secret (the BFF is the trust boundary that
   already verified the admin via WebAuthn, ADR-0017). It becomes the I5 audit actor / `created_by`.
3. **Group from the single-install binding.** `group_id` is the `GROUP_ID` `[vars]` binding (one
   install = one Group), the RLS tenant every query is scoped to — never client-supplied.

The gate (`server/src/runtime/members.rs::admin_guard`) runs **before** any DB connect, so a reject is
cheap and leaks nothing. The OpenAPI contract models this as the `adminSharedSecret` security scheme +
the `AdminIdHeader` parameter (spec 008 T08); the contract test
`openapi_admin_surface_requires_shared_secret` asserts every admin op declares both (closing OpenAPI's
fail-open default for a future admin op).

## Alternatives considered

- **Defer the auth gate to T10 (endpoints unauthenticated behind a deploy guard).** Rejected: an
  endpoint that decrypts PII with no auth gate is a fail-open hazard even if "guarded"; the shared
  secret is cheap and ships the gate *with* the endpoints.
- **The Worker re-verifies the admin session itself.** Rejected for now: it would duplicate the
  SvelteKit session logic in Rust and read a session store that is currently in-memory in the web tier
  (ADR-0017 / T15-shell). It widens the P4 carve-out ADR-0017 deliberately bounded. Revisit only if a
  Worker-native admin session is ever wanted.
- **Fold the admin endpoints into SvelteKit (call Postgres directly, skip the Worker).** Rejected: it
  would move the per-Group crypto + validation out of the Rust core into edge-TS (violates P4 and the
  architecture's "server routes hit the same Workers API as the apps").

## Consequences

- **Trust boundary stated, not assumed.** The Worker's admin trust is exactly "a caller holding
  `ADMIN_API_SECRET`, asserting an `admin_id`." The blast radius of a leaked secret is the whole admin
  surface, so it is a Secrets-Store-class secret (rotated like any), and the gate fails closed.
- **The real SvelteKit→Worker call is T10.** T09 builds + miniflare-tests the Worker side (the secret +
  asserted id are injected in tests). Wiring the verified-admin SvelteKit BFF to present them is T10.
- **Defense-in-depth deferred (recorded in `DEFERRED.md` → T09):** the Worker could additionally verify
  the asserted `admin_id` actually holds `role = admin` in `members` (so a leaked secret cannot forge an
  arbitrary actor on the audit trail). Not required for v1 (the secret is the trust boundary); a cheap
  follow-up.
- **No new always-on infra** (P11): no sidecar, no second auth service — one shared secret + one header.
