// Miniflare integration tests for the spec-008 **T09** admin member-management routes.
//
// Runs the real Rust→wasm Worker over a real local Postgres (the emulated Hyperdrive binding → the DB
// seeded by scripts/setup-worker-test-db.sh: migrations 0001–0011 + a bootstrapped Group with a
// KEK-wrapped `delegated_keys` row). Each call composes the real core `MemberService` over the real
// `PgMemberStore` (P4): validate → load+unwrap the Group key with the test KEK → encrypt name/address/
// phone → atomic insert. The ADR-0026 shared secret + `X-Admin-Id` gate every request.
//
// The DB is fresh per `setup-worker-test-db.sh` run but NOT reset between tests, so each test issues a
// UNIQUE phone and asserts only its own member (never global counts) — robust to accumulated state.

import { SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';

const BASE = 'https://worker.example';
// Must match `ADMIN_API_SECRET` in vitest.config.ts.
const SECRET = 'test-admin-shared-secret-do-not-deploy';
// The BFF-asserted acting admin id (any uuid — the I5 audit actor / `created_by`).
const ADMIN_ID = '00000000-0000-0000-0000-0000000000aa';

function adminHeaders(extra: Record<string, string> = {}, secret = SECRET): Record<string, string> {
	return {
		'content-type': 'application/json',
		authorization: `Bearer ${secret}`,
		'x-admin-id': ADMIN_ID,
		...extra,
	};
}

// A per-run random 4-digit base + a per-call counter → a distinct +1 555 <base> <counter> E.164 number,
// unique WITHIN a run (the counter) AND ACROSS runs (the random base) — so re-running `pnpm test` without
// a DB reset (the DB persists between runs until setup-worker-test-db.sh drops it) does not hit stale
// duplicate-phone collisions. (Test-only randomness — the Rust core forbids ambient randomness, not this.)
const RUN_BASE = String(Math.floor(Math.random() * 9000) + 1000); // 1000..9999
let phoneCounter = 0;
function uniquePhone(): string {
	phoneCounter += 1;
	return `+1555${RUN_BASE}${String(phoneCounter).padStart(3, '0')}`;
}

/* eslint-disable @typescript-eslint/no-explicit-any */
async function issue(name: string, phone: string, address: string): Promise<any> {
	const res = await SELF.fetch(`${BASE}/api/admin/members`, {
		method: 'POST',
		headers: adminHeaders(),
		body: JSON.stringify({ name, phone, address, roles: ['rider'] }),
	});
	expect(res.status).toBe(201);
	return (await res.json()) as any;
}

describe('admin member-management (spec 008 T09 — MemberService over PgMemberStore)', () => {
	it('worker_issue_member_round_trip: issue → 201 + code, then audited detail decrypts the PII', async () => {
		const phone = uniquePhone();
		const res = await SELF.fetch(`${BASE}/api/admin/members`, {
			method: 'POST',
			headers: adminHeaders(),
			body: JSON.stringify({ name: 'Maria', phone, address: '12 Olive St', roles: ['rider'] }),
		});
		expect(res.status).toBe(201);
		const issued = (await res.json()) as any;
		expect(issued.member.name).toBe('Maria');
		expect(issued.member.roles).toEqual(['rider']);
		expect(issued.member.onboarding_status).toBe('issued_not_onboarded');
		expect(typeof issued.onboarding_code).toBe('string');
		expect(issued.onboarding_code.length).toBeGreaterThan(0);
		expect(typeof issued.code_expires_at).toBe('number');
		const memberId = issued.member.member_id as string;

		// The audited detail read decrypts the PII round-trip (I1/I3 ciphertext → plaintext).
		const detailRes = await SELF.fetch(`${BASE}/api/admin/members/${memberId}`, {
			headers: adminHeaders(),
		});
		expect(detailRes.status).toBe(200);
		const detail = (await detailRes.json()) as any;
		expect(detail.name).toBe('Maria');
		expect(detail.phone).toBe(phone); // already E.164 → normalize is a no-op
		expect(detail.address).toBe('12 Olive St');
		expect(detail.roles).toEqual(['rider']);
		expect(typeof detail.updated_at).toBe('number');
	});

	it('worker_detail_read_emits_audit: a detail read writes an I5 audit row (names only)', async () => {
		const phone = uniquePhone();
		const issued = await issue('Daniel', phone, '5 Birch Rd');
		const memberId = issued.member.member_id as string;

		await SELF.fetch(`${BASE}/api/admin/members/${memberId}`, { headers: adminHeaders() });

		const logRes = await SELF.fetch(`${BASE}/api/admin/audit-log?member_id=${memberId}`, {
			headers: adminHeaders(),
		});
		expect(logRes.status).toBe(200);
		const log = (await logRes.json()) as any;
		const mine = (log.entries as any[]).filter((e) => e.member_id === memberId);
		expect(mine.length).toBeGreaterThanOrEqual(1);
		const entry = mine[mine.length - 1];
		expect(entry.admin_id).toBe(ADMIN_ID);
		expect(entry.fields).toEqual(['name', 'phone', 'address']); // names, never values (AC9)
		expect(typeof entry.request_id).toBe('string');
		// The audit log carries NO PII value (P2/AC9).
		const raw = JSON.stringify(log);
		expect(raw).not.toContain('Daniel');
		expect(raw).not.toContain('5 Birch Rd');
		expect(raw).not.toContain(phone);
	});

	it('worker_regenerate_code: regenerate mints a fresh Onboarding Code', async () => {
		const issued = await issue('Margaret', uniquePhone(), '9 Cedar Ln');
		const memberId = issued.member.member_id as string;
		const res = await SELF.fetch(`${BASE}/api/admin/members/${memberId}/regenerate-code`, {
			method: 'POST',
			headers: adminHeaders(),
		});
		expect(res.status).toBe(200);
		const body = (await res.json()) as any;
		expect(typeof body.onboarding_code).toBe('string');
		expect(body.onboarding_code.length).toBeGreaterThan(0);
		expect(body.onboarding_code).not.toBe(issued.onboarding_code); // a fresh code supersedes the prior
		expect(typeof body.code_expires_at).toBe('number');
	});

	it('worker_duplicate_phone_links_existing: re-issuing a phone surfaces-and-links the existing member', async () => {
		const phone = uniquePhone();
		const first = await issue('Tobias', phone, '3 Elm St');
		const dupRes = await SELF.fetch(`${BASE}/api/admin/members`, {
			method: 'POST',
			headers: adminHeaders(),
			body: JSON.stringify({ name: 'Imposter', phone, address: '99 Fake Rd', roles: ['driver'] }),
		});
		expect(dupRes.status).toBe(409);
		const dup = (await dupRes.json()) as any;
		expect(dup.error_code).toBe('ADMIN_MEMBER_DUPLICATE_PHONE');
		expect(dup.existing.member_id).toBe(first.member.member_id);
		// The EXISTING member is surfaced (name only) — never the imposter's submitted name/address.
		expect(dup.existing.name).toBe('Tobias');
		expect(JSON.stringify(dup)).not.toContain('99 Fake Rd');
	});

	it('worker_error_response_contains_no_submitted_pii: a validation error never echoes the submitted PII (R10)', async () => {
		const sentinelName = 'SENTINEL-NAME-9z7q';
		const sentinelAddr = 'SENTINEL-ADDR-9z7q';
		const badPhone = 'not-a-phone-9z7q';
		const res = await SELF.fetch(`${BASE}/api/admin/members`, {
			method: 'POST',
			headers: adminHeaders(),
			body: JSON.stringify({
				name: sentinelName,
				phone: badPhone,
				address: sentinelAddr,
				roles: ['rider'],
			}),
		});
		expect(res.status).toBe(400);
		const raw = await res.text();
		expect(raw).toContain('ADMIN_MEMBER_PHONE_INVALID'); // a stable, PII-free code
		expect(raw).not.toContain(sentinelName);
		expect(raw).not.toContain(sentinelAddr);
		expect(raw).not.toContain(badPhone);
	});

	it('worker_admin_endpoints_reject_without_shared_secret: fail closed (ADR-0026)', async () => {
		// No Authorization header → 401 (before any DB connect) with the ErrorBody code (contract shape).
		const noAuth = await SELF.fetch(`${BASE}/api/admin/members`, {
			headers: { 'x-admin-id': ADMIN_ID },
		});
		expect(noAuth.status).toBe(401);
		expect(((await noAuth.json()) as any).error_code).toBe('ADMIN_UNAUTHORIZED');
		// Wrong shared secret → 401.
		const wrong = await SELF.fetch(`${BASE}/api/admin/members`, {
			headers: adminHeaders({}, 'wrong-secret'),
		});
		expect(wrong.status).toBe(401);
		expect(((await wrong.json()) as any).error_code).toBe('ADMIN_UNAUTHORIZED');
		// An Admin role cannot be issued (I11/AC10) even with a valid secret.
		const adminRole = await SELF.fetch(`${BASE}/api/admin/members`, {
			method: 'POST',
			headers: adminHeaders(),
			body: JSON.stringify({
				name: 'X',
				phone: uniquePhone(),
				address: 'Y',
				roles: ['admin'],
			}),
		});
		expect(adminRole.status).toBe(400);
		expect(await adminRole.text()).toContain('ADMIN_MEMBER_ROLE_FORBIDDEN');
	});
});
