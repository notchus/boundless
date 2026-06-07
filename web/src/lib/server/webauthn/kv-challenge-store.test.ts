// Proves `KvChallengeStore` against a REAL Cloudflare KV namespace — wrangler's `getPlatformProxy()`
// boots an in-process Miniflare (the same workerd KV used in production) with NO Cloudflare account,
// reading the `CHALLENGES` binding from web/wrangler.toml. This is the real-KV counterpart of
// `challenge.test.ts` (which asserts the same ADR-0017 D3 semantics against the in-memory fake).
// Spec 001 T15-shell leg A.

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { getPlatformProxy } from 'wrangler';
import type { KVNamespace } from '@cloudflare/workers-types';

import { KvChallengeStore, kvExpirationTtl } from './kv-challenge-store';

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

describe('KvChallengeStore (real Miniflare KV via getPlatformProxy — no Cloudflare account)', () => {
	let dispose: () => Promise<void>;
	let store: KvChallengeStore;

	beforeAll(async () => {
		const proxy = await getPlatformProxy<{ CHALLENGES: KVNamespace }>({
			configPath: './wrangler.toml',
			persist: false,
		});
		dispose = proxy.dispose;
		store = new KvChallengeStore(proxy.env.CHALLENGES);
	});

	afterAll(async () => {
		await dispose();
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
