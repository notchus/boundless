// Shared test wiring: a deterministic `WebAuthnDeps` over the in-memory port fakes. Used by
// the Vitest unit tests and the Playwright real-ceremony test. Test-only (not a `.test.ts`,
// so Vitest does not execute it).

import type { WebAuthnDeps } from '../ports';
import {
  FixedClock,
  MemoryChallengeStore,
  MemoryCredentialStore,
  MemoryInviteStore,
} from './memory-stores';

/** RP config for tests. `localhost` is a WebAuthn-eligible secure context (Playwright). */
export const TEST_RP = {
  rpName: 'Boundless',
  rpID: 'localhost',
  origin: 'http://localhost',
} as const;

export interface Harness {
  readonly clock: FixedClock;
  readonly challenges: MemoryChallengeStore;
  readonly invites: MemoryInviteStore;
  readonly credentials: MemoryCredentialStore;
  readonly deps: WebAuthnDeps;
}

export function makeHarness(now: number): Harness {
  const clock = new FixedClock(now);
  const challenges = new MemoryChallengeStore(clock);
  const invites = new MemoryInviteStore();
  const credentials = new MemoryCredentialStore();
  return {
    clock,
    challenges,
    invites,
    credentials,
    deps: { rp: TEST_RP, clock, challenges, invites, credentials },
  };
}
