// Proves `KvChallengeStore` against a REAL Cloudflare KV namespace — wrangler's `getPlatformProxy()`
// boots an in-process Miniflare (the same workerd KV used in production) with NO Cloudflare account,
// reading the `CHALLENGES` binding from web/wrangler.toml. This is the real-KV counterpart of
// `challenge.test.ts` (which asserts the same ADR-0017 D3 semantics against the in-memory fake).
// Spec 001 T15-shell leg A.

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { getPlatformProxy } from 'wrangler';
import type { KVNamespace } from '@cloudflare/workers-types';

import { KvChallengeStore, kvExpirationTtl, selectChallengeStore } from './kv-challenge-store';
import type { ChallengeStore } from './ports';

describe('kvExpirationTtl', () => {
	it('floors a sub-60 TTL to KV’s 60-second minimum', () => {
		expect(kvExpirationTtl(1)).toBe(60);
		expect(kvExpirationTtl(59)).toBe(60);
	});

	it('passes a TTL ≥ 60 through unchanged (incl. the 300s challenge TTL)', () => {
		expect(kvExpirationTtl(60)).toBe(60);
		expect(kvExpirationTtl(300)).toBe(300);
	});
});

// A trivial fallback stand-in; the selector only stores the reference, never calls it. (The
// "binding present" branch is covered with the REAL Miniflare KV in the suite below — no fake KV.)
const fallback: ChallengeStore = { put: async () => {}, take: async () => null };

describe('selectChallengeStore — no-binding branches (pure, no Miniflare)', () => {
	it('fails closed: no binding + fallback disallowed (production) throws (ADR-0017 D3)', () => {
		expect(() => selectChallengeStore(undefined, fallback, false)).toThrow(/CHALLENGES KV namespace is not bound/);
	});

	it('falls back to the in-memory store when no binding but fallback is allowed (dev/test)', () => {
		expect(selectChallengeStore(undefined, fallback, true)).toBe(fallback);
	});
});

describe('KvChallengeStore (real Miniflare KV via getPlatformProxy — no Cloudflare account)', () => {
	let dispose: () => Promise<void>;
	let kv: KVNamespace;
	let store: KvChallengeStore;

	beforeAll(async () => {
		const proxy = await getPlatformProxy<{ CHALLENGES: KVNamespace }>({
			configPath: './wrangler.toml',
			persist: false,
		});
		dispose = proxy.dispose;
		kv = proxy.env.CHALLENGES;
		store = new KvChallengeStore(proxy.env.CHALLENGES);
	});

	afterAll(async () => {
		await dispose();
	});

	it('selectChallengeStore picks the real KV store whenever the binding is present (either dev flag)', () => {
		// When a binding exists, the dev flag is irrelevant — never the in-memory fallback.
		expect(selectChallengeStore(kv, fallback, false)).toBeInstanceOf(KvChallengeStore);
		expect(selectChallengeStore(kv, fallback, true)).toBeInstanceOf(KvChallengeStore);
	});

	it('round-trips a stored challenge', async () => {
		await store.put('k-roundtrip', 'chal-1', 300);
		expect(await store.take('k-roundtrip')).toBe('chal-1');
	});

	it('is consume-once: a second take returns null (ADR-0017 D3)', async () => {
		await store.put('k-once', 'chal-2', 300);
		expect(await store.take('k-once')).toBe('chal-2');
		expect(await store.take('k-once')).toBeNull();
	});

	it('returns null for an absent key', async () => {
		expect(await store.take('k-never-stored')).toBeNull();
	});

	it('isolates keys (taking one leaves the other intact)', async () => {
		await store.put('k-a', 'AAA', 300);
		await store.put('k-b', 'BBB', 300);
		expect(await store.take('k-a')).toBe('AAA');
		expect(await store.take('k-b')).toBe('BBB');
	});
});
