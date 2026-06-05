import type { RegistrationResponseJSON } from '@simplewebauthn/server';
import { describe, expect, it } from 'vitest';

import { WebAuthnError } from './errors';
import { evaluateInvite } from './invite';
import { verifyRegistration } from './register';
import { makeHarness } from './testing/harness';

const NOW = 1_700_000_000;
const TTL = 72 * 60 * 60;
// The invite check happens before any WebAuthn verification, so these tests never touch a
// real ceremony — the response is never read.
const DUMMY = {} as unknown as RegistrationResponseJSON;

describe('evaluateInvite (AC16 lifecycle policy)', () => {
  it('an unknown token is treated as expired (no existence oracle)', () => {
    expect(evaluateInvite(null, NOW)).toMatchObject({ status: 'expired', code: 'ADMIN_INVITE_EXPIRED' });
  });

  it('a consumed token is rejected as consumed (single-use)', () => {
    const record = { adminId: 'a', groupId: 'g', expiresAt: NOW + TTL, consumedAt: NOW - 1 };
    expect(evaluateInvite(record, NOW)).toMatchObject({ status: 'consumed', code: 'ADMIN_INVITE_CONSUMED' });
  });

  it('TTL is server-time inclusive: now == expiresAt is expired', () => {
    const record = { adminId: 'a', groupId: 'g', expiresAt: NOW, consumedAt: null };
    expect(evaluateInvite(record, NOW)).toMatchObject({ status: 'expired', code: 'ADMIN_INVITE_EXPIRED' });
  });

  it('a live, unconsumed, unexpired token resolves to its admin/group', () => {
    const record = { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + 1, consumedAt: null };
    expect(evaluateInvite(record, NOW)).toMatchObject({ status: 'live', adminId: 'admin-1', groupId: 'grp-1' });
  });
});

describe('ac16_invite_expired_routes_and_ttl_server_side', () => {
  it('a reused (consumed) link is rejected and routes to InviteExpired', async () => {
    const h = makeHarness(NOW);
    h.invites.add('tok', { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + TTL, consumedAt: NOW - 10 });
    await h.challenges.put('cer', 'challenge-value', 600); // live challenge → we reach the invite check

    await expect(
      verifyRegistration(h.deps, { ceremonyKey: 'cer', presentedToken: 'tok', response: DUMMY }),
    ).rejects.toMatchObject({ code: 'ADMIN_INVITE_CONSUMED', routesTo: 'InviteExpired' });
  });

  it('an expired link is rejected against SERVER time and routes to InviteExpired', async () => {
    const h = makeHarness(NOW);
    h.invites.add('tok', { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + TTL });
    await h.challenges.put('cer', 'challenge-value', 10_000_000); // long-lived → isolates invite TTL
    h.clock.set(NOW + TTL + 1); // server clock advances past the TTL (device clock is irrelevant)

    await expect(
      verifyRegistration(h.deps, { ceremonyKey: 'cer', presentedToken: 'tok', response: DUMMY }),
    ).rejects.toMatchObject({ code: 'ADMIN_INVITE_EXPIRED', routesTo: 'InviteExpired' });
  });

  it('an unknown link routes to InviteExpired and throws a WebAuthnError', async () => {
    const h = makeHarness(NOW);
    await h.challenges.put('cer', 'challenge-value', 600);

    const result = verifyRegistration(h.deps, { ceremonyKey: 'cer', presentedToken: 'nope', response: DUMMY });
    await expect(result).rejects.toBeInstanceOf(WebAuthnError);
    await expect(result).rejects.toMatchObject({ code: 'ADMIN_INVITE_EXPIRED', routesTo: 'InviteExpired' });
  });
});
