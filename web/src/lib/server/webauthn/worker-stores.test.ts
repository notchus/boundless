// Request-shape + outcome-mapping unit tests for the Worker-backed admin-WebAuthn stores (spec 009
// T05). Proves the adapters present the ADR-0027 pre-session credential (Bearer shared secret, NO
// X-Admin-Id) and the correct method/URL/body to the Worker's B1 endpoints, map each frozen-contract
// status to its port outcome, and coalesce register.ts's three-call registration tail into ONE atomic
// `register-complete` via the shared handshake (R11) — WITHOUT a live Worker. Also covers the
// fail-closed `selectInviteStore`/`selectCredentialStore`. Node env; no SvelteKit-virtual imports.

import { afterEach, describe, expect, it, vi } from 'vitest';

import { WebAuthnError } from './errors';
import type { CredentialStore, InviteStore, StoredCredential } from './ports';
import { MemoryCredentialStore, MemoryInviteStore } from './testing/memory-stores';
import {
	selectCredentialStore,
	selectInviteStore,
	WorkerCredentialStore,
	WorkerInviteStore,
	WorkerRegistrationHandshake,
} from './worker-stores';

const BASE = 'https://worker.example';
const SECRET = 'shh-shared-secret';

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

function body(init: RequestInit): unknown {
	return JSON.parse(init.body as string);
}

afterEach(() => {
	vi.restoreAllMocks();
});

describe('WorkerInviteStore — pre-session shared-secret credential + frozen B1 request shape', () => {
	const store = (h = new WorkerRegistrationHandshake()): WorkerInviteStore => new WorkerInviteStore(BASE, SECRET, h);

	it('load: POST invite/resolve {token} with Bearer secret and NO x-admin-id → 200 InviteRecord', async () => {
		const fn = mockFetch(200, {
			admin_id: '00000000-0000-0000-0000-0000000000aa',
			group_id: '00000000-0000-0000-0000-000000000001',
			expires_at: 100_000,
			consumed_at: null,
		});
		const rec = await store().load('tok-123');

		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/webauthn/invite/resolve');
		expect(init.method).toBe('POST');
		expect(headerVal(init, 'authorization')).toBe(`Bearer ${SECRET}`);
		expect(headerVal(init, 'x-admin-id')).toBeUndefined(); // pre-session: NO acting admin id
		expect(body(init)).toEqual({ token: 'tok-123' }); // token in the BODY (R13), never the URL
		expect(rec).toEqual({
			adminId: '00000000-0000-0000-0000-0000000000aa',
			groupId: '00000000-0000-0000-0000-000000000001',
			expiresAt: 100_000,
			consumedAt: null,
		});
	});

	it('load: 404 (unknown/cross-tenant, value-free) → null; an unexpected status → value-free throw', async () => {
		mockFetch(404, { error_code: 'ADMIN_INVITE_NOT_FOUND' });
		expect(await store().load('nope')).toBeNull();

		mockFetch(500, { error_code: 'oops' });
		await expect(store().load('boom')).rejects.toThrow(/HTTP 500/);
	});

	it('markConsumed: stashes the token in the handshake, makes NO network call (consume is folded into register-complete)', async () => {
		const fn = mockFetch(200, {});
		const handshake = new WorkerRegistrationHandshake();
		await store(handshake).markConsumed('tok-xyz', 12_345);
		expect(fn).not.toHaveBeenCalled();
		expect(handshake.presentedToken).toBe('tok-xyz');
	});
});

describe('WorkerCredentialStore — frozen B1 request shape + register-complete coalescing', () => {
	const handshake = (): WorkerRegistrationHandshake => new WorkerRegistrationHandshake();
	const credStore = (h: WorkerRegistrationHandshake): WorkerCredentialStore => new WorkerCredentialStore(BASE, SECRET, h);
	const inviteStore = (h: WorkerRegistrationHandshake): WorkerInviteStore => new WorkerInviteStore(BASE, SECRET, h);

	// A credential as register.ts's `verifyRegistration` builds it: publicKey is RAW BYTES.
	const cred = (): StoredCredential => ({
		credentialId: 'AQID', // base64url string already (the port type)
		adminId: '00000000-0000-0000-0000-0000000000aa',
		publicKey: new Uint8Array([4, 5, 6]), // → base64url "BAUG" on the wire
		signCount: 0,
		revokedAt: null,
	});

	it('findActive: POST credentials/lookup {credential_id} → 200 StoredCredential (public_key decoded to bytes)', async () => {
		const fn = mockFetch(200, {
			credential_id: 'AQID',
			admin_id: '00000000-0000-0000-0000-0000000000aa',
			public_key: 'BAUG', // base64url of [4,5,6]
			sign_count: 9,
			transports: ['usb', 'nfc'],
			revoked_at: null,
		});
		const stored = await credStore(handshake()).findActive('AQID');

		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/webauthn/credentials/lookup');
		expect(init.method).toBe('POST');
		expect(headerVal(init, 'authorization')).toBe(`Bearer ${SECRET}`);
		expect(headerVal(init, 'x-admin-id')).toBeUndefined();
		expect(body(init)).toEqual({ credential_id: 'AQID' });
		expect(stored?.credentialId).toBe('AQID'); // kept as the base64url string
		expect(stored?.publicKey).toBeInstanceOf(Uint8Array);
		expect(Array.from(stored?.publicKey ?? [])).toEqual([4, 5, 6]); // decoded to bytes for @simplewebauthn
		expect(stored?.signCount).toBe(9);
		expect(stored?.transports).toEqual(['usb', 'nfc']);
	});

	it('findActive: 404 → null', async () => {
		mockFetch(404, { error_code: 'ADMIN_CREDENTIAL_NOT_FOUND' });
		expect(await credStore(handshake()).findActive('missing')).toBeNull();
	});

	it('insert: after markConsumed, fires ONE register-complete {token, credential} (public_key encoded)', async () => {
		const h = handshake();
		// register.ts's tail order: markConsumed (stash) → revokeAllForAdmin (no-op) → insert (fire).
		const fn = mockFetch(200, { admin_id: '00000000-0000-0000-0000-0000000000aa' });
		await inviteStore(h).markConsumed('tok-reg', 1000);
		await credStore(h).revokeAllForAdmin('00000000-0000-0000-0000-0000000000aa', 1000); // no-op
		await credStore(h).insert(cred());

		// Exactly one network call across the three tail calls (revoke + markConsumed are local).
		expect(fn).toHaveBeenCalledTimes(1);
		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/webauthn/register-complete');
		expect(init.method).toBe('POST');
		expect(headerVal(init, 'authorization')).toBe(`Bearer ${SECRET}`);
		expect(body(init)).toEqual({
			token: 'tok-reg', // the token markConsumed stashed
			credential: { credential_id: 'AQID', public_key: 'BAUG', sign_count: 0 }, // public_key ENCODED
		});
		// The handshake is single-use: cleared after firing.
		expect(h.presentedToken).toBeNull();
	});

	it('insert: converts the ceremony dashed-hex UUID aaguid → base64url-no-pad of the 16 bytes (wire/core form)', async () => {
		const h = handshake();
		const fn = mockFetch(200, { admin_id: '00000000-0000-0000-0000-0000000000aa' });
		await inviteStore(h).markConsumed('tok-reg', 1000);
		// register.ts passes @simplewebauthn's aaguid verbatim — a dashed-hex UUID string.
		await credStore(h).insert({ ...cred(), aaguid: 'adce0002-35bc-c60a-648b-0b25f1f05503' });

		const sent = body(lastCall(fn).init) as { credential: { aaguid?: string } };
		// "adce0002-35bc-c60a-648b-0b25f1f05503" → 16 bytes → base64url-no-pad "rc4AAjW8xgpkiwsl8fBVAw".
		expect(sent.credential.aaguid).toBe('rc4AAjW8xgpkiwsl8fBVAw');
	});

	it('insert: a malformed/absent aaguid is OMITTED, never shipped as wrong bytes', async () => {
		const h = handshake();
		const fn = mockFetch(200, { admin_id: '00000000-0000-0000-0000-0000000000aa' });
		await inviteStore(h).markConsumed('tok-reg', 1000);
		await credStore(h).insert({ ...cred(), aaguid: 'not-a-uuid' });
		expect((body(lastCall(fn).init) as { credential: Record<string, unknown> }).credential).not.toHaveProperty('aaguid');
	});

	it('findActive: a base64url aaguid round-trips back to the dashed-hex UUID string (the port form)', async () => {
		mockFetch(200, {
			credential_id: 'AQID',
			admin_id: '00000000-0000-0000-0000-0000000000aa',
			public_key: 'BAUG',
			sign_count: 0,
			aaguid: 'rc4AAjW8xgpkiwsl8fBVAw', // base64url-no-pad of the 16 AAGUID bytes
			revoked_at: null,
		});
		const stored = await credStore(handshake()).findActive('AQID');
		expect(stored?.aaguid).toBe('adce0002-35bc-c60a-648b-0b25f1f05503');
	});

	it('insert without a prior markConsumed throws (no handshake — the additive backup-key path is not wired)', async () => {
		const fn = mockFetch(200, {});
		await expect(credStore(handshake()).insert(cred())).rejects.toThrow(/requires an invite handshake/);
		expect(fn).not.toHaveBeenCalled(); // never reaches the Worker
	});

	it('insert: a 400 (TOCTOU — invite consumed/expired meanwhile) → WebAuthnError(ADMIN_INVITE_CONSUMED)', async () => {
		const h = handshake();
		mockFetch(400, { error_code: 'ADMIN_INVITE_CONSUMED' });
		await inviteStore(h).markConsumed('tok-stale', 1000);
		const err = await credStore(h).insert(cred()).catch((e: unknown) => e);
		expect(err).toBeInstanceOf(WebAuthnError);
		expect((err as WebAuthnError).code).toBe('ADMIN_INVITE_CONSUMED');
		expect((err as WebAuthnError).routesTo).toBe('InviteExpired');
	});

	it('bumpSignCount: POST credentials/{id}/sign-count {sign_count} → 204', async () => {
		const fn = mockFetch(204, undefined);
		await credStore(handshake()).bumpSignCount('AQID', 7, 1000);
		const { url, init } = lastCall(fn);
		expect(url).toBe('https://worker.example/api/admin/webauthn/credentials/AQID/sign-count');
		expect(init.method).toBe('POST');
		expect(body(init)).toEqual({ sign_count: 7 });
	});

	it('bumpSignCount: an unexpected status → value-free throw', async () => {
		mockFetch(500, { error_code: 'oops' });
		await expect(credStore(handshake()).bumpSignCount('AQID', 7, 1000)).rejects.toThrow(/HTTP 500/);
	});

	it('listActiveByAdmin + revokeAllForAdmin are local no-ops (no frozen pre-session op)', async () => {
		const fn = mockFetch(200, {});
		expect(await credStore(handshake()).listActiveByAdmin('any-admin')).toEqual([]);
		await credStore(handshake()).revokeAllForAdmin('any-admin', 1000);
		expect(fn).not.toHaveBeenCalled();
	});
});

describe('selectInviteStore / selectCredentialStore — fail closed (ADR-0026/0027)', () => {
	const invFallback: InviteStore = new MemoryInviteStore();
	const credFallback: CredentialStore = new MemoryCredentialStore();
	const h = new WorkerRegistrationHandshake();

	it('return the real Worker store when base + secret are configured', () => {
		expect(selectInviteStore(BASE, SECRET, invFallback, false, h)).toBeInstanceOf(WorkerInviteStore);
		expect(selectCredentialStore(BASE, SECRET, credFallback, false, h)).toBeInstanceOf(WorkerCredentialStore);
	});

	it('fall back to the in-memory store only in dev', () => {
		expect(selectInviteStore(undefined, undefined, invFallback, true, h)).toBe(invFallback);
		expect(selectInviteStore(BASE, undefined, invFallback, true, h)).toBe(invFallback); // partial config
		expect(selectCredentialStore(undefined, undefined, credFallback, true, h)).toBe(credFallback);
		expect(selectCredentialStore(BASE, undefined, credFallback, true, h)).toBe(credFallback);
	});

	it('throw (fail closed) when unconfigured outside dev', () => {
		expect(() => selectInviteStore(undefined, undefined, invFallback, false, h)).toThrow(/Refusing the in-memory fallback/);
		expect(() => selectInviteStore(BASE, undefined, invFallback, false, h)).toThrow(/ADMIN_API_SECRET/);
		expect(() => selectCredentialStore(undefined, undefined, credFallback, false, h)).toThrow(/Refusing the in-memory fallback/);
		expect(() => selectCredentialStore(BASE, undefined, credFallback, false, h)).toThrow(/ADMIN_API_SECRET/);
	});
});
