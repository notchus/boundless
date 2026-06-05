import { describe, expect, it } from 'vitest';

import { FixedClock, MemoryChallengeStore } from './testing/memory-stores';

const NOW = 1_700_000_000;

// ADR-0017 D3: the WebAuthn challenge is one-time-use with a ~5-minute TTL. These assert the
// KV semantics the production store must honour (the in-memory fake stands in for KV here).
describe('challenge_one_time_use', () => {
  it('a stored challenge can be taken exactly once', async () => {
    const store = new MemoryChallengeStore(new FixedClock(NOW));
    await store.put('k', 'chal', 300);
    expect(await store.take('k')).toBe('chal');
    expect(await store.take('k')).toBeNull(); // consumed on first read
  });

  it('a challenge at/after its TTL is not returned (server-time)', async () => {
    const clock = new FixedClock(NOW);
    const store = new MemoryChallengeStore(clock);
    await store.put('k', 'chal', 300);
    clock.advance(300); // now == expiresAt → expired
    expect(await store.take('k')).toBeNull();
  });

  it('an absent key yields null', async () => {
    const store = new MemoryChallengeStore(new FixedClock(NOW));
    expect(await store.take('missing')).toBeNull();
  });
});
