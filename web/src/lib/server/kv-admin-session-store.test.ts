// Proves the durable admin-session store (spec 009 T06, AC2) against BOTH a REAL Cloudflare KV namespace
// — wrangler's `getPlatformProxy()` boots an in-process Miniflare (the same workerd KV used in production)
// with NO Cloudflare account, reading the `ADMIN_SESSIONS` binding from web/wrangler.toml — and the
// in-memory dev fallback. This is the session-store counterpart of `kv-challenge-store.test.ts`.
//
// AC2: a session persists across a simulated cold start (a fresh store instance over the same persisted
// KV resolves the id), the server-side TTL is enforced (an expired id → null → the caller redirects to
// sign-in), and a sign-out revokes it. Plus R1 (≥128-bit, non-derived id) and the §10-F cookie policy
// (migrated from the spec-001 session.test.ts, which can no longer load under bare Vitest now that
// `session.ts` imports `$app/environment`).

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { getPlatformProxy } from 'wrangler';
import type { KVNamespace } from '@cloudflare/workers-types';

import {
	ADMIN_SESSION_COOKIE,
	type Clock,
	KvSessionStore,
	MemorySessionStore,
	selectSessionStore,
	SESSION_COOKIE_OPTIONS,
	type SessionStore,
} from './kv-admin-session-store';

const TTL = 3600;
/** A mutable unix-seconds clock, so a test can advance time deterministically past `expiresAt` (R2). */
function movableClock(start: number): Clock & { set(t: number): void } {
	let t = start;
	return { now: () => t, set: (next) => (t = next) };
}

describe('§10-F admin session cookie policy', () => {
	it('cookie flags are httpOnly + Secure + SameSite=Strict, app-wide path', () => {
		expect(SESSION_COOKIE_OPTIONS.httpOnly).toBe(true);
		expect(SESSION_COOKIE_OPTIONS.secure).toBe(true);
		expect(SESSION_COOKIE_OPTIONS.sameSite).toBe('strict');
		expect(SESSION_COOKIE_OPTIONS.path).toBe('/');
	});

	it('uses a stable cookie name', () => {
		expect(ADMIN_SESSION_COOKIE).toBe('boundless_admin_session');
	});
});

// A trivial fallback stand-in; the selector only stores the reference, never calls it. (The
// "binding present" branch is covered with the REAL Miniflare KV in the suite below — no fake KV.)
const fallback: SessionStore = { create: async () => '', get: async () => null, revoke: async () => {} };
const clock: Clock = { now: () => 1_000_000 };

describe('selectSessionStore — no-binding branches (pure, no Miniflare)', () => {
	it('fails closed: no binding + fallback disallowed (production) throws (D5/AC2)', () => {
		expect(() => selectSessionStore(undefined, clock, TTL, fallback, false)).toThrow(
			/ADMIN_SESSIONS KV namespace is not bound/,
		);
	});

	it('falls back to the in-memory store when no binding but fallback is allowed (dev/test)', () => {
		expect(selectSessionStore(undefined, clock, TTL, fallback, true)).toBe(fallback);
	});
});

describe('MemorySessionStore (the dev/test fallback) — round-trip, TTL, revoke, entropy', () => {
	it('round-trips a session id to its admin', async () => {
		const store = new MemorySessionStore(clock, TTL);
		const id = await store.create('admin-mem');
		expect(await store.get(id)).toEqual({ adminId: 'admin-mem' });
	});

	it('returns null for an unknown id', async () => {
		const store = new MemorySessionStore(clock, TTL);
		expect(await store.get('not-a-session')).toBeNull();
	});

	it('returns null for an absent (undefined) cookie value', async () => {
		const store = new MemorySessionStore(clock, TTL);
		expect(await store.get(undefined)).toBeNull();
	});

	it('rejects an id at/after its logical expiry (server-side TTL, R2)', async () => {
		const mc = movableClock(1_000_000);
		const store = new MemorySessionStore(mc, 100); // expiresAt = now + 100
		const id = await store.create('admin-ttl');
		mc.set(1_000_099);
		expect(await store.get(id)).not.toBeNull(); // still live one second before expiry
		mc.set(1_000_100);
		expect(await store.get(id)).toBeNull(); // now >= expiresAt
	});

	it('revoke() makes a session id resolve to null (sign-out)', async () => {
		const store = new MemorySessionStore(clock, TTL);
		const id = await store.create('admin-revoke');
		await store.revoke(id);
		expect(await store.get(id)).toBeNull();
	});

	it('mints a ≥128-bit opaque id, fresh per call and NOT derived from the adminId (R1)', async () => {
		const store = new MemorySessionStore(clock, TTL);
		const a = await store.create('admin-entropy');
		const b = await store.create('admin-entropy');
		expect(a).not.toBe(b); // independent draws — not fixation/derivation
		expect(a).not.toContain('admin-entropy');
		expect(a).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i); // v4 UUID
	});
});

describe('KvSessionStore (real Miniflare KV via getPlatformProxy — no Cloudflare account)', () => {
	let dispose: () => Promise<void>;
	let kv: KVNamespace;

	beforeAll(async () => {
		const proxy = await getPlatformProxy<{ ADMIN_SESSIONS: KVNamespace }>({
			configPath: './wrangler.toml',
			persist: false,
		});
		dispose = proxy.dispose;
		kv = proxy.env.ADMIN_SESSIONS;
	});

	afterAll(async () => {
		await dispose();
	});

	it('selectSessionStore picks the real KV store whenever the binding is present (either dev flag)', () => {
		expect(selectSessionStore(kv, clock, TTL, fallback, false)).toBeInstanceOf(KvSessionStore);
		expect(selectSessionStore(kv, clock, TTL, fallback, true)).toBeInstanceOf(KvSessionStore);
	});

	it('round-trips a stored session', async () => {
		const store = new KvSessionStore(kv, clock, TTL);
		const id = await store.create('admin-kv');
		expect(await store.get(id)).toEqual({ adminId: 'admin-kv' });
	});

	it('persists across a simulated cold start: a fresh store over the same KV resolves the id (AC2)', async () => {
		const writer = new KvSessionStore(kv, clock, TTL);
		const id = await writer.create('admin-cold-start');
		// Drop `writer`; a brand-new store instance (the post-cold-start isolate) over the SAME KV binding.
		const reader = new KvSessionStore(kv, clock, TTL);
		expect(await reader.get(id)).toEqual({ adminId: 'admin-cold-start' });
	});

	it('enforces the server-side TTL: an expired id resolves to null even before KV evicts (R2)', async () => {
		const mc = movableClock(2_000_000);
		const store = new KvSessionStore(kv, mc, 100); // expiresAt = 2_000_100
		const id = await store.create('admin-kv-ttl');
		expect(await store.get(id)).not.toBeNull();
		mc.set(2_000_100); // now >= expiresAt — rejected by the in-value check, not KV eviction
		expect(await store.get(id)).toBeNull();
	});

	it('revoke() deletes the session: a revoked id resolves to null (sign-out, AC2)', async () => {
		const store = new KvSessionStore(kv, clock, TTL);
		const id = await store.create('admin-kv-revoke');
		expect(await store.get(id)).not.toBeNull();
		await store.revoke(id);
		expect(await store.get(id)).toBeNull();
	});

	it('returns null for an absent id', async () => {
		const store = new KvSessionStore(kv, clock, TTL);
		expect(await store.get('never-stored')).toBeNull();
	});

	it('returns null for an absent (undefined) cookie value without touching KV', async () => {
		const store = new KvSessionStore(kv, clock, TTL);
		expect(await store.get(undefined)).toBeNull();
	});

	it('mints a fresh ≥128-bit id per session, not derived from the adminId (R1 / no fixation, R4)', async () => {
		const store = new KvSessionStore(kv, clock, TTL);
		const a = await store.create('admin-kv-entropy');
		const b = await store.create('admin-kv-entropy');
		expect(a).not.toBe(b);
		expect(a).not.toContain('admin-kv-entropy');
		// The authenticated session id is an independent draw, never the pre-auth ceremony key (R4):
		// the sign-in route also sets a DISTINCT cookie name and deletes the ceremony cookie first
		// (asserted on the wire by the onboarding e2e). Here we prove the id is a fresh, opaque UUID.
		expect(a).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
	});
});
