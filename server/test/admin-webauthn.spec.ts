// Miniflare integration tests for the spec-009 **T04** Option B1 admin-WebAuthn routes
// (`/api/admin/webauthn/*`, ADR-0027).
//
// Runs the real Rust→wasm Worker over the real local Postgres seeded by scripts/setup-worker-test-db.sh:
// the B1 seed (server/store/examples/seed_worker_test_b1_pg.rs) creates two LIVE Group-A invitations —
// `boundless-test-invite-resolve` (read-only) and `boundless-test-invite-register` (consumed here) —
// with their token_hash computed IN THE CORE under the Worker's HMAC_KEY (so resolve matches, AC4b).
// Each call composes the real core `AdminWebAuthnStore` over the real `PgAuthStore` (P4), behind the
// ADR-0026 shared secret. The routes are PRE-SESSION: shared secret required, NO `X-Admin-Id`.
//
// NB: `register-complete` CONSUMES its invitation (single-use, AC4a/R15), so this file expects a fresh
// `setup-worker-test-db.sh` run (which re-seeds the live invites) — the canonical local/CI flow runs
// setup then test. The resolve assertions are read-only and re-run-safe.

import { SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';

const BASE = 'https://worker.example';
// Must match `ADMIN_API_SECRET` in vitest.config.ts.
const SECRET = 'test-admin-shared-secret-do-not-deploy';
// The Worker's single-install tenant (wrangler.toml `[vars] GROUP_ID` / the setup script's SEED_GROUP_ID).
const GROUP_A_ID = '00000000-0000-0000-0000-000000000001';
// Token labels seeded as LIVE Group-A invitations by seed_worker_test_b1_pg.rs (shared verbatim).
const TOKEN_RESOLVE = 'boundless-test-invite-resolve';
// The register-complete pool — the test claims the first still-live one (register-complete is
// single-use), so the suite survives this many re-runs without re-seeding. Keep in lock-step with the
// seed example's REGISTER_POOL.
const REGISTER_POOL = 24;

// Pre-session B1 headers: the shared secret only — NO `x-admin-id` (the admin is being registered /
// authenticated; ADR-0027). This is the deliberate difference from the member ops.
function b1Headers(secret = SECRET): Record<string, string> {
	return { 'content-type': 'application/json', authorization: `Bearer ${secret}` };
}

/* eslint-disable @typescript-eslint/no-explicit-any */
async function post(path: string, body: unknown, headers = b1Headers()): Promise<Response> {
	return SELF.fetch(`${BASE}${path}`, { method: 'POST', headers, body: JSON.stringify(body) });
}

// base64url-no-pad of raw bytes (the WebAuthn-byte wire convention the Worker decodes). `btoa` is the
// Web-standard encoder available in workerd; we URL-safe it and strip padding.
function b64url(bytes: Uint8Array): string {
	let s = '';
	for (const b of bytes) s += String.fromCharCode(b);
	return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}
// A per-run-unique credential id / public key (random bytes → base64url). Distinct per run so a
// re-run's insert never collides on the global `credential_id` unique index.
function freshBytes(n: number): string {
	return b64url(crypto.getRandomValues(new Uint8Array(n)));
}

// Claim the first still-LIVE register invite from the seeded pool (register-complete is single-use, so
// earlier runs consumed earlier tokens). Read-only resolve probing; `null` if the pool is exhausted
// (→ re-run setup-worker-test-db.sh).
async function claimLiveRegisterToken(): Promise<string | null> {
	for (let i = 0; i < REGISTER_POOL; i++) {
		const token = `boundless-test-invite-register-${i}`;
		const res = await post('/api/admin/webauthn/invite/resolve', { token });
		if (res.status === 200 && ((await res.json()) as any).consumed_at === null) return token;
	}
	return null;
}

describe('admin WebAuthn B1 surface (spec 009 T04 — AdminWebAuthnStore over PgAuthStore)', () => {
	it('worker_invite_resolve_round_trip: a live token resolves to its PII-free invite metadata; a wrong token → value-free 404', async () => {
		const res = await post('/api/admin/webauthn/invite/resolve', { token: TOKEN_RESOLVE });
		expect(res.status).toBe(200);
		const rec = (await res.json()) as any;
		// PII-free metadata: admin id (uuid), the Worker's group, the TTL instant, and live (null consumed).
		expect(typeof rec.admin_id).toBe('string');
		expect(rec.admin_id).toMatch(/^[0-9a-f-]{36}$/);
		expect(rec.group_id).toBe(GROUP_A_ID);
		expect(typeof rec.expires_at).toBe('number');
		expect(rec.consumed_at).toBeNull();
		// No member-PII field leaked into the invite metadata.
		const raw = JSON.stringify(rec);
		expect(raw).not.toContain('phone');
		expect(raw).not.toContain('address');

		// A wrong token → the same value-free 404 (no existence oracle, R16); never echoes the token.
		const miss = await post('/api/admin/webauthn/invite/resolve', { token: 'no-such-token-zzz' });
		expect(miss.status).toBe(404);
		expect(((await miss.json()) as any).error_code).toBe('ADMIN_INVITE_NOT_FOUND');
	});

	it('worker_register_complete_then_lookup_and_bump: consume + insert + usernameless lookup + only-if-greater bump (R10/R11), then single-use reject (AC4a)', async () => {
		const credentialId = freshBytes(16);
		const publicKey = freshBytes(32);
		const registerToken = await claimLiveRegisterToken();
		expect(registerToken, 'a live register invite (re-run setup-worker-test-db.sh if the pool is exhausted)').not.toBeNull();

		// register-complete: consume the live REGISTER invite + insert the credential, in one txn (R11).
		const done = await post('/api/admin/webauthn/register-complete', {
			token: registerToken!,
			credential: { credential_id: credentialId, public_key: publicKey, sign_count: 0 },
		});
		expect(done.status).toBe(200);
		const completed = (await done.json()) as any;
		expect(completed.admin_id).toMatch(/^[0-9a-f-]{36}$/); // the server-derived admin id

		// Usernameless lookup by credential_id resolves the active credential (admin id read OFF it).
		const look = await post('/api/admin/webauthn/credentials/lookup', { credential_id: credentialId });
		expect(look.status).toBe(200);
		const cred = (await look.json()) as any;
		expect(cred.credential_id).toBe(credentialId);
		expect(cred.admin_id).toBe(completed.admin_id);
		expect(cred.public_key).toBe(publicKey);
		expect(cred.sign_count).toBe(0);
		expect(cred.revoked_at).toBeNull();

		// Bump only-if-strictly-greater (clone-detection backstop, R10). 204, no body.
		const up = await post(`/api/admin/webauthn/credentials/${credentialId}/sign-count`, { sign_count: 5 });
		expect(up.status).toBe(204);
		const afterUp = (await (await post('/api/admin/webauthn/credentials/lookup', { credential_id: credentialId })).json()) as any;
		expect(afterUp.sign_count).toBe(5);

		// A lower count is a no-op (not an error); the stored counter stays 5.
		const down = await post(`/api/admin/webauthn/credentials/${credentialId}/sign-count`, { sign_count: 3 });
		expect(down.status).toBe(204);
		const afterDown = (await (await post('/api/admin/webauthn/credentials/lookup', { credential_id: credentialId })).json()) as any;
		expect(afterDown.sign_count).toBe(5);

		// Single-use: re-presenting the now-consumed token writes nothing → value-free ADMIN_INVITE_CONSUMED.
		const again = await post('/api/admin/webauthn/register-complete', {
			token: registerToken!,
			credential: { credential_id: freshBytes(16), public_key: freshBytes(32), sign_count: 0 },
		});
		expect(again.status).toBe(400);
		expect(((await again.json()) as any).error_code).toBe('ADMIN_INVITE_CONSUMED');
	});

	it('worker_b1_is_pre_session: the shared secret alone suffices — no X-Admin-Id required (ADR-0027)', async () => {
		// b1Headers carries NO x-admin-id; resolve still succeeds (pre-session). The member ops, by
		// contrast, reject a missing x-admin-id with ADMIN_BAD_REQUEST (proven in admin-members.spec.ts).
		const res = await post('/api/admin/webauthn/invite/resolve', { token: TOKEN_RESOLVE });
		expect(res.status).toBe(200);
		// Adding an x-admin-id changes nothing (it is ignored on the pre-session surface).
		const withId = await post('/api/admin/webauthn/invite/resolve', { token: TOKEN_RESOLVE }, {
			...b1Headers(),
			'x-admin-id': '00000000-0000-0000-0000-0000000000aa',
		});
		expect(withId.status).toBe(200);
	});

	it('worker_b1_fails_closed_without_secret: no/wrong shared secret → 401 (ADR-0026)', async () => {
		// No Authorization header → 401 ADMIN_UNAUTHORIZED (before any DB connect).
		const noAuth = await SELF.fetch(`${BASE}/api/admin/webauthn/invite/resolve`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ token: TOKEN_RESOLVE }),
		});
		expect(noAuth.status).toBe(401);
		expect(((await noAuth.json()) as any).error_code).toBe('ADMIN_UNAUTHORIZED');

		// Wrong secret → 401. Applies to every B1 op (credentials/lookup here).
		const wrong = await post('/api/admin/webauthn/credentials/lookup', { credential_id: 'AQID' }, b1Headers('wrong-secret'));
		expect(wrong.status).toBe(401);
		expect(((await wrong.json()) as any).error_code).toBe('ADMIN_UNAUTHORIZED');
	});

	it('invite_resolve_error_body_has_no_token: a no-match 404 never echoes the presented token (AC8/R13)', async () => {
		const sentinel = 'SENTINEL-INVITE-TOKEN-9z7q';
		const res = await post('/api/admin/webauthn/invite/resolve', { token: sentinel });
		expect(res.status).toBe(404);
		const raw = await res.text();
		expect(raw).toContain('ADMIN_INVITE_NOT_FOUND'); // a stable, value-free code
		expect(raw).not.toContain(sentinel); // the token never appears in the response (P2)

		// register-complete on an unknown token: value-free ADMIN_INVITE_CONSUMED, no token echoed.
		const rc = await post('/api/admin/webauthn/register-complete', {
			token: sentinel,
			credential: { credential_id: freshBytes(16), public_key: freshBytes(32), sign_count: 0 },
		});
		expect(rc.status).toBe(400);
		const rcRaw = await rc.text();
		expect(rcRaw).toContain('ADMIN_INVITE_CONSUMED');
		expect(rcRaw).not.toContain(sentinel);
	});
});
