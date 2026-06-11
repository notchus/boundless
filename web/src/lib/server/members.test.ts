// Request-shape + outcome-mapping unit tests for the admin member BFF client (spec 008 T10). Proves
// the real `WorkerMembersClient` presents the ADR-0026 credentials (Bearer shared secret + X-Admin-Id)
// and the correct method/URL/body to the Worker, and maps each frozen-contract status to its typed
// outcome — WITHOUT a live Worker (the live deployed round-trip is the deferred shell). Also covers the
// fail-closed `selectMembersClient` decision. Node env; no SvelteKit-virtual imports (the module is pure).

import { afterEach, describe, expect, it, vi } from 'vitest';

import {
	InMemoryMembersClient,
	selectMembersClient,
	WorkerMembersClient,
	type MembersClient,
} from './members';

const BASE = 'https://worker.example';
const SECRET = 'shh-shared-secret';
const ADMIN = '00000000-0000-0000-0000-0000000000aa';

function mockFetch(status: number, body: unknown): ReturnType<typeof vi.fn> {
	const fn = vi.fn(async () => new Response(body === undefined ? null : JSON.stringify(body), { status }));
	globalThis.fetch = fn as unknown as typeof fetch;
	return fn;
}

function lastCall(fn: ReturnType<typeof vi.fn>): { url: string; init: RequestInit } {
	const call = fn.mock.calls.at(-1);
	if (!call) throw new Error('fetch was not called');
	return { url: call[0] as string, init: (call[1] ?? {}) as RequestInit };
}

function headerVal(init: RequestInit, name: string): string | undefined {
	return (init.headers as Record<string, string> | undefined)?.[name];
}

const client = (): WorkerMembersClient => new WorkerMembersClient(BASE, SECRET);

afterEach(() => {
	vi.restoreAllMocks();
});

describe('WorkerMembersClient — ADR-0026 credentials + frozen-contract request shape', () => {
	it('list: GET /api/admin/members with filters + Bearer secret + X-Admin-Id', async () => {
		const fn = mockFetch(200, { members: [{ member_id: 'm1', name: 'Maria', roles: ['rider'], onboarding_status: 'onboarded' }] });
		const out = await client().list(ADMIN, { search: 'mar', role: 'rider', status: 'onboarded' });

		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/members?search=mar&role=rider&status=onboarded');
		expect(init.method ?? 'GET').toBe('GET');
		expect(headerVal(init, 'authorization')).toBe(`Bearer ${SECRET}`);
		expect(headerVal(init, 'x-admin-id')).toBe(ADMIN);
		expect(out).toEqual([{ member_id: 'm1', name: 'Maria', roles: ['rider'], onboarding_status: 'onboarded' }]);
	});

	it('list: omits absent query params (no trailing ?)', async () => {
		const fn = mockFetch(200, { members: [] });
		await client().list(ADMIN, {});
		expect(lastCall(fn).url).toBe('https://worker.example/api/admin/members');
	});

	it('issue: POST JSON body → 201 issued (with show-once code)', async () => {
		const fn = mockFetch(201, {
			member: { member_id: 'm2', name: 'Daniel', roles: ['driver'], onboarding_status: 'issued_not_onboarded' },
			onboarding_code: 'BNDL-ABCD1234',
			code_expires_at: 1700,
		});
		const out = await client().issue(ADMIN, { name: 'Daniel', phone: '+15555550100', address: '5 Birch Rd', roles: ['driver'] });

		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/members');
		expect(init.method).toBe('POST');
		expect(headerVal(init, 'content-type')).toBe('application/json');
		expect(JSON.parse(init.body as string)).toEqual({ name: 'Daniel', phone: '+15555550100', address: '5 Birch Rd', roles: ['driver'] });
		expect(out).toEqual({ kind: 'issued', member: expect.objectContaining({ member_id: 'm2' }), onboarding_code: 'BNDL-ABCD1234', code_expires_at: 1700 });
	});

	it('issue: 409 → duplicate (links existing); 400 → rejected(code); 503 → group_key_missing', async () => {
		mockFetch(409, { error_code: 'ADMIN_MEMBER_DUPLICATE_PHONE', existing: { member_id: 'm1', name: 'Maria', roles: ['rider'], onboarding_status: 'onboarded' } });
		expect(await client().issue(ADMIN, { name: 'x', phone: '+15555550100', address: 'y', roles: ['rider'] })).toEqual({
			kind: 'duplicate',
			existing: { member_id: 'm1', name: 'Maria', roles: ['rider'], onboarding_status: 'onboarded' },
		});

		mockFetch(400, { error_code: 'ADMIN_MEMBER_PHONE_INVALID' });
		expect(await client().issue(ADMIN, { name: 'x', phone: 'nope', address: 'y', roles: ['rider'] })).toEqual({ kind: 'rejected', code: 'ADMIN_MEMBER_PHONE_INVALID' });

		mockFetch(503, { error_code: 'ADMIN_GROUP_KEY_MISSING' });
		expect(await client().issue(ADMIN, { name: 'x', phone: '+15555550100', address: 'y', roles: ['rider'] })).toEqual({ kind: 'group_key_missing' });
	});

	it('detail: GET /{id} → 200 detail; 404 → not_found', async () => {
		const fn = mockFetch(200, { member_id: 'm1', name: 'Maria', phone: '+15555550100', address: '12 Olive St', roles: ['rider'], onboarding_status: 'onboarded', updated_at: 42 });
		const out = await client().detail(ADMIN, 'm1');
		expect(lastCall(fn).url).toBe('https://worker.example/api/admin/members/m1');
		expect(out).toEqual({ kind: 'detail', detail: expect.objectContaining({ phone: '+15555550100', updated_at: 42 }) });

		mockFetch(404, { error_code: 'ADMIN_MEMBER_NOT_FOUND' });
		expect(await client().detail(ADMIN, 'gone')).toEqual({ kind: 'not_found' });
	});

	it('edit: PATCH /{id} with expected_updated_at → 200 updated; 409 → stale; 404 → not_found', async () => {
		const fn = mockFetch(200, { member_id: 'm1', name: 'Maria', phone: '+15555550100', address: 'new', roles: ['rider'], onboarding_status: 'onboarded', updated_at: 43 });
		const out = await client().edit(ADMIN, 'm1', { address: 'new', expected_updated_at: 42 });
		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/members/m1');
		expect(init.method).toBe('PATCH');
		expect(JSON.parse(init.body as string)).toEqual({ address: 'new', expected_updated_at: 42 });
		expect(out).toEqual({ kind: 'updated', detail: expect.objectContaining({ updated_at: 43 }) });

		mockFetch(409, { error_code: 'ADMIN_MEMBER_EDIT_STALE' });
		expect(await client().edit(ADMIN, 'm1', { address: 'x', expected_updated_at: 1 })).toEqual({ kind: 'stale' });

		mockFetch(404, { error_code: 'ADMIN_MEMBER_NOT_FOUND' });
		expect(await client().edit(ADMIN, 'gone', { expected_updated_at: 1 })).toEqual({ kind: 'not_found' });
	});

	it('regenerateCode: POST /{id}/regenerate-code → 200 regenerated; 404 → not_found', async () => {
		const fn = mockFetch(200, { onboarding_code: 'BNDL-NEW00001', code_expires_at: 1800 });
		const out = await client().regenerateCode(ADMIN, 'm1');
		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/members/m1/regenerate-code');
		expect(init.method).toBe('POST');
		expect(out).toEqual({ kind: 'regenerated', onboarding_code: 'BNDL-NEW00001', code_expires_at: 1800 });

		mockFetch(404, { error_code: 'ADMIN_MEMBER_NOT_FOUND' });
		expect(await client().regenerateCode(ADMIN, 'gone')).toEqual({ kind: 'not_found' });
	});

	it('auditLog: GET /api/admin/audit-log?member_id= → entries', async () => {
		const fn = mockFetch(200, { entries: [{ timestamp: 1, admin_id: ADMIN, member_id: 'm1', fields: ['name', 'phone', 'address'], request_id: 'r1' }] });
		const out = await client().auditLog(ADMIN, { memberId: 'm1' });
		expect(lastCall(fn).url).toBe('https://worker.example/api/admin/audit-log?member_id=m1');
		expect(out).toEqual([{ timestamp: 1, admin_id: ADMIN, member_id: 'm1', fields: ['name', 'phone', 'address'], request_id: 'r1' }]);
	});

	it('a 401 (BFF misconfig) throws a value-free error (no body echo, P2)', async () => {
		mockFetch(401, { error_code: 'ADMIN_UNAUTHORIZED' });
		await expect(client().list(ADMIN, {})).rejects.toThrow(/HTTP 401/);
	});
});

describe('selectMembersClient — fail closed (ADR-0026)', () => {
	const fallback: MembersClient = new InMemoryMembersClient();

	it('returns the real Worker client when base + secret are configured', () => {
		expect(selectMembersClient(BASE, SECRET, fallback, false)).toBeInstanceOf(WorkerMembersClient);
	});

	it('falls back to the in-memory fake only in dev', () => {
		expect(selectMembersClient(undefined, undefined, fallback, true)).toBe(fallback);
		expect(selectMembersClient(BASE, undefined, fallback, true)).toBe(fallback); // partial config
	});

	it('throws (fails closed) when unconfigured outside dev', () => {
		expect(() => selectMembersClient(undefined, undefined, fallback, false)).toThrow(/Refusing the in-memory fallback/);
		expect(() => selectMembersClient(BASE, undefined, fallback, false)).toThrow(/ADMIN_API_SECRET/);
	});
});
