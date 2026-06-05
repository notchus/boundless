import type { AuthenticationResponseJSON } from '@simplewebauthn/server';
import { describe, expect, it } from 'vitest';

import { buildAuthenticationOptions, verifyAuthentication } from './authenticate';
import { makeHarness } from './testing/harness';

const NOW = 1_700_000_000;
const DUMMY = { id: 'unknown-cred' } as unknown as AuthenticationResponseJSON;

describe('buildAuthenticationOptions (AC2/AC20)', () => {
  it('requires user verification and persists the challenge', async () => {
    const h = makeHarness(NOW);
    const options = await buildAuthenticationOptions(h.deps, { ceremonyKey: 'cer' });
    expect(options.userVerification).toBe('required');
    expect(h.challenges.has('cer')).toBe(true);
  });

  it('scopes allowCredentials to an admin active credentials when adminId is given', async () => {
    const h = makeHarness(NOW);
    await h.credentials.insert({
      credentialId: 'cred-A',
      adminId: 'admin-1',
      publicKey: new Uint8Array([1]),
      signCount: 0,
      revokedAt: null,
    });
    const options = await buildAuthenticationOptions(h.deps, { ceremonyKey: 'cer', adminId: 'admin-1' });
    expect(options.allowCredentials?.map((c) => c.id)).toEqual(['cred-A']);
  });

  it('omits allowCredentials for usernameless/discoverable sign-in', async () => {
    const h = makeHarness(NOW);
    const options = await buildAuthenticationOptions(h.deps, { ceremonyKey: 'cer' });
    expect(options.allowCredentials ?? []).toHaveLength(0);
  });
});

describe('verifyAuthentication guards', () => {
  it('a missing/expired challenge yields ADMIN_WEBAUTHN_CHALLENGE_EXPIRED', async () => {
    const h = makeHarness(NOW);
    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'never-put', response: DUMMY }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_CHALLENGE_EXPIRED' });
  });

  it('an unknown/revoked credential yields ADMIN_WEBAUTHN_VERIFICATION_FAILED', async () => {
    const h = makeHarness(NOW);
    await h.challenges.put('cer', 'challenge-value', 600);
    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'cer', response: DUMMY }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' });
  });

  it('a credential revoked via D4 recovery can no longer authenticate (enforced at the verify layer)', async () => {
    const h = makeHarness(NOW);
    await h.credentials.insert({
      credentialId: 'cred-X',
      adminId: 'admin-1',
      publicKey: new Uint8Array([1]),
      signCount: 0,
      revokedAt: null,
    });
    await h.credentials.revokeAllForAdmin('admin-1', NOW); // Developer re-invite revokes prior creds
    await h.challenges.put('cer', 'challenge-value', 600);

    // findActive('cred-X') now returns null (revoked) → the unknown-credential branch.
    await expect(
      verifyAuthentication(h.deps, {
        ceremonyKey: 'cer',
        response: { id: 'cred-X' } as unknown as AuthenticationResponseJSON,
      }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' });
  });

  it('the verify path consumes the challenge one-time: a second call with the same key → CHALLENGE_EXPIRED', async () => {
    const h = makeHarness(NOW);
    await h.challenges.put('cer', 'challenge-value', 600);
    // First call takes (consumes) the challenge, then fails on the unknown credential.
    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'cer', response: DUMMY }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' });
    // The challenge is now gone → a replay with the same key is rejected before any verification.
    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'cer', response: DUMMY }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_CHALLENGE_EXPIRED' });
  });
});
