// Cross-tenant isolation proof for the spec-009 **T04** Option B1 admin-WebAuthn routes (AC14 / D3 /
// R16) — the Worker-HTTP-level analog of server/test/cross-tenant.spec.ts (the member surface).
//
// The same real Rust→wasm Worker over the same local Postgres, connecting as the non-superuser, NON-
// `BYPASSRLS` `boundless_app` role (scripts/setup-worker-test-db.sh) so RLS genuinely applies. The
// Worker is RLS-scoped to its single `GROUP_ID` binding (Group A = …0001). The B1 seed
// (seed_worker_test_b1_pg.rs) plants in GROUP B (…2000…): a LIVE invitation
// (`boundless-test-invite-xtenant`, with the CORRECT token_hash under the Worker's HMAC_KEY) and an
// ACTIVE credential (`xtenant-b1-credential`). Both are seeded so that — ABSENT RLS — the Group-A
// Worker WOULD resolve them (the invite hash matches; the credential_id unique index is global). So a
// 404 / non-consumable is a genuine "RLS hid an otherwise-matching row" signal, not a mismatch (the
// non-vacuous form — the same philosophy as the cross-tenant member's real `updated_at`).
//
// The genuinely-live version (against the DEPLOYED edge, ≥2 Groups in Neon) is the operator-gated shell
// — DEFERRED.md → "Admin web deploy — B1 … deployed-edge".

import { SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';

const BASE = 'https://worker.example';
// Must match `ADMIN_API_SECRET` in vitest.config.ts.
const SECRET = 'test-admin-shared-secret-do-not-deploy';
// The Group-B fixtures seeded by seed_worker_test_b1_pg.rs (shared verbatim). The Worker is scoped to
// Group A, so both are invisible to it.
const XTENANT_TOKEN = 'boundless-test-invite-xtenant';
// base64url-no-pad of the seeded Group-B credential_id bytes (`b"xtenant-b1-credential"`).
const XTENANT_CREDENTIAL_ID = 'eHRlbmFudC1iMS1jcmVkZW50aWFs';

// Pre-session B1 headers (shared secret only — ADR-0027).
function b1Headers(): Record<string, string> {
	return { 'content-type': 'application/json', authorization: `Bearer ${SECRET}` };
}

/* eslint-disable @typescript-eslint/no-explicit-any */
async function post(path: string, body: unknown): Promise<Response> {
	return SELF.fetch(`${BASE}${path}`, { method: 'POST', headers: b1Headers(), body: JSON.stringify(body) });
}

describe('B1 cross-tenant isolation (spec 009 T04 — AC14 / D3)', () => {
	it('worker_cross_tenant_invite_resolve_isolated: a Group-A Worker cannot resolve/consume a Group-B invite, nor look up a Group-B credential', async () => {
		// 1. RESOLVE a Group-B invite token — RLS hides the row ⇒ the same value-free 404 as an unknown
		// token (no existence oracle). Non-vacuous: the seed used the CORRECT hash, so absent RLS this
		// WOULD resolve to a 200 invite record.
		const resolve = await post('/api/admin/webauthn/invite/resolve', { token: XTENANT_TOKEN });
		expect(resolve.status).toBe(404);
		expect(((await resolve.json()) as any).error_code).toBe('ADMIN_INVITE_NOT_FOUND');

		// 2. CREDENTIAL LOOKUP for a Group-B credential_id — invisible despite the GLOBAL unique index
		// (RLS scopes the read) ⇒ value-free 404. Absent RLS this would resolve the active credential.
		const lookup = await post('/api/admin/webauthn/credentials/lookup', {
			credential_id: XTENANT_CREDENTIAL_ID,
		});
		expect(lookup.status).toBe(404);
		expect(((await lookup.json()) as any).error_code).toBe('ADMIN_CREDENTIAL_NOT_FOUND');

		// 3. REGISTER-COMPLETE on the Group-B token — a cross-tenant WRITE cannot reach the row: the
		// invitation is invisible, so it is not consumable ⇒ value-free ADMIN_INVITE_CONSUMED, nothing
		// written (the txn rolls back). The store-level test proves the Group-B invite stays live; here
		// the clean 400 (not a 5xx from a partial write) is the RLS write-isolation signal.
		const register = await post('/api/admin/webauthn/register-complete', {
			token: XTENANT_TOKEN,
			credential: { credential_id: 'Y3Jvc3MtdGVuYW50LWV2aWw', public_key: 'ZXZpbC1wdWI', sign_count: 0 },
		});
		expect(register.status).toBe(400);
		expect(((await register.json()) as any).error_code).toBe('ADMIN_INVITE_CONSUMED');
	});
});
