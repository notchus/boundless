import type { RegistrationResponseJSON } from '@simplewebauthn/server';
import { describe, expect, it } from 'vitest';

import { buildRegistrationOptions, verifyRegistration } from './register';
import type { Harness } from './testing/harness';
import { makeHarness } from './testing/harness';

const NOW = 1_700_000_000;
const TTL = 72 * 60 * 60;
const DUMMY = {} as unknown as RegistrationResponseJSON;

function seedLiveInvite(h: Harness): void {
  h.invites.add('tok', { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + TTL });
}

describe('buildRegistrationOptions (AC20 policy)', () => {
  it('requires UV, no attestation, prefers resident keys, offers EdDSA/ES256/RS256', async () => {
    const h = makeHarness(NOW);
    seedLiveInvite(h);

    const options = await buildRegistrationOptions(h.deps, {
      ceremonyKey: 'cer',
      presentedToken: 'tok',
      userName: 'admin-invite',
      userDisplayName: 'Boundless Admin',
    });

    expect(options.authenticatorSelection?.userVerification).toBe('required');
    expect(options.attestation).toBe('none');
    expect(options.authenticatorSelection?.residentKey).toBe('preferred');
    expect(options.pubKeyCredParams.map((p) => p.alg)).toEqual(expect.arrayContaining([-8, -7, -257]));
    expect(h.challenges.has('cer')).toBe(true); // challenge persisted for the round-trip
  });

  it('binds the WebAuthn user handle to the opaque adminId, not to userName/displayName (no PII in the handle)', async () => {
    const h = makeHarness(NOW);
    seedLiveInvite(h);

    const options = await buildRegistrationOptions(h.deps, {
      ceremonyKey: 'cer',
      presentedToken: 'tok',
      userName: 'admin-invite',
      userDisplayName: 'Boundless Admin',
    });

    const handle = Buffer.from(options.user.id, 'base64url').toString('utf8');
    expect(handle).toBe('admin-1'); // the opaque admin id from the invite
    expect(handle).not.toBe('admin-invite');
    expect(handle).not.toBe('Boundless Admin');
  });

  it('excludeCredentials lists the admin existing active credentials (multi-cred safety)', async () => {
    const h = makeHarness(NOW);
    seedLiveInvite(h);
    await h.credentials.insert({
      credentialId: 'cred-A',
      adminId: 'admin-1',
      publicKey: new Uint8Array([1, 2, 3]),
      signCount: 0,
      revokedAt: null,
    });

    const options = await buildRegistrationOptions(h.deps, {
      ceremonyKey: 'cer',
      presentedToken: 'tok',
      userName: 'n',
      userDisplayName: 'd',
    });

    expect(options.excludeCredentials?.map((c) => c.id)).toContain('cred-A');
  });

  it('refuses to start a ceremony for a dead invite (no challenge issued)', async () => {
    const h = makeHarness(NOW);
    h.invites.add('tok', { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + TTL, consumedAt: NOW - 1 });

    await expect(
      buildRegistrationOptions(h.deps, {
        ceremonyKey: 'cer',
        presentedToken: 'tok',
        userName: 'n',
        userDisplayName: 'd',
      }),
    ).rejects.toMatchObject({ code: 'ADMIN_INVITE_CONSUMED' });
    expect(h.challenges.has('cer')).toBe(false);
  });
});

describe('verifyRegistration challenge handling (one-time-use)', () => {
  it('a missing/expired challenge yields ADMIN_WEBAUTHN_CHALLENGE_EXPIRED', async () => {
    const h = makeHarness(NOW);
    seedLiveInvite(h);

    await expect(
      verifyRegistration(h.deps, { ceremonyKey: 'never-put', presentedToken: 'tok', response: DUMMY }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_CHALLENGE_EXPIRED', routesTo: 'register_credential' });
  });
});
