// Deterministic Node coverage of the verify-path orchestration that the Playwright e2e proves
// with real ceremony bytes. Here the @simplewebauthn/server verifier is stubbed (vi.mock) so the
// pure branches over its result + the in-memory ports are exercised fast and hermetically:
//   • R11 / AC20 UV enforcement: userVerified=false → ADMIN_WEBAUTHN_UV_REQUIRED (reg + auth).
//   • verified=false → ADMIN_WEBAUTHN_VERIFICATION_FAILED.
//   • verifier throws (incl. the library's counter-regression / cloned-authenticator error) →
//     ADMIN_WEBAUTHN_VERIFICATION_FAILED.
//   • Registration success: challenge consumed, invite consumed (AC16), prior creds revoked (D4),
//     new credential inserted; re-presenting the now-consumed invite → ADMIN_INVITE_CONSUMED.
//   • A failed registration does NOT consume the invite (but does burn the one-time challenge).
//   • Authentication success bumps the stored signature counter.

import type {
  AuthenticationResponseJSON,
  RegistrationResponseJSON,
  VerifiedAuthenticationResponse,
  VerifiedRegistrationResponse,
} from '@simplewebauthn/server';
import { verifyAuthenticationResponse, verifyRegistrationResponse } from '@simplewebauthn/server';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { verifyAuthentication } from './authenticate';
import type { StoredCredential } from './ports';
import { verifyRegistration } from './register';
import type { Harness } from './testing/harness';
import { makeHarness } from './testing/harness';

vi.mock('@simplewebauthn/server', async (importActual) => {
  const actual = await importActual<typeof import('@simplewebauthn/server')>();
  return { ...actual, verifyRegistrationResponse: vi.fn(), verifyAuthenticationResponse: vi.fn() };
});

const NOW = 1_700_000_000;
const TTL = 72 * 60 * 60;
const DUMMY_REG = {} as unknown as RegistrationResponseJSON;

function regResult(opts: { verified?: boolean; userVerified?: boolean; credentialId?: string }): VerifiedRegistrationResponse {
  const { verified = true, userVerified = true, credentialId = 'cred-X' } = opts;
  if (!verified) {
    return { verified: false } as unknown as VerifiedRegistrationResponse;
  }
  return {
    verified: true,
    registrationInfo: {
      credential: { id: credentialId, publicKey: new Uint8Array([9, 9, 9]), counter: 0, transports: ['internal'] },
      aaguid: 'aaguid-x',
      userVerified,
    },
  } as unknown as VerifiedRegistrationResponse;
}

function authResult(opts: { verified?: boolean; userVerified?: boolean; newCounter?: number }): VerifiedAuthenticationResponse {
  const { verified = true, userVerified = true, newCounter = 5 } = opts;
  if (!verified) {
    return { verified: false } as unknown as VerifiedAuthenticationResponse;
  }
  return {
    verified: true,
    authenticationInfo: { newCounter, userVerified, credentialID: 'cred-X' },
  } as unknown as VerifiedAuthenticationResponse;
}

function liveHarness(): Harness {
  const h = makeHarness(NOW);
  h.invites.add('tok', { adminId: 'admin-1', groupId: 'grp-1', expiresAt: NOW + TTL });
  return h;
}

function existingCred(credentialId: string): StoredCredential {
  return { credentialId, adminId: 'admin-1', publicKey: new Uint8Array([1]), signCount: 0, revokedAt: null };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('verifyRegistration orchestration (mocked verifier)', () => {
  it('on success: consumes the challenge + invite (AC16), revokes priors (D4), inserts the credential', async () => {
    const h = liveHarness();
    await h.credentials.insert(existingCred('old-key')); // a prior credential to be revoked
    await h.challenges.put('reg', 'chal', 600);
    vi.mocked(verifyRegistrationResponse).mockResolvedValue(regResult({}));

    const outcome = await verifyRegistration(h.deps, { ceremonyKey: 'reg', presentedToken: 'tok', response: DUMMY_REG });

    expect(outcome).toMatchObject({ adminId: 'admin-1', credentialId: 'cred-X' });
    expect(h.invites.isConsumed('tok')).toBe(true); // AC16 consume-on-success
    expect(h.challenges.has('reg')).toBe(false); // one-time challenge consumed
    // D4: the prior credential is revoked, only the new one is active.
    expect((await h.credentials.listActiveByAdmin('admin-1')).map((c) => c.credentialId)).toEqual(['cred-X']);
    expect(h.credentials.all().find((c) => c.credentialId === 'old-key')?.revokedAt).toBe(NOW);
  });

  it('single-use loop: a successful registration consumes the invite, so re-presenting it → ADMIN_INVITE_CONSUMED', async () => {
    const h = liveHarness();
    await h.challenges.put('reg', 'chal', 600);
    vi.mocked(verifyRegistrationResponse).mockResolvedValue(regResult({}));
    await verifyRegistration(h.deps, { ceremonyKey: 'reg', presentedToken: 'tok', response: DUMMY_REG });

    await h.challenges.put('reg2', 'chal2', 600); // fresh challenge so we reach the invite check
    await expect(
      verifyRegistration(h.deps, { ceremonyKey: 'reg2', presentedToken: 'tok', response: DUMMY_REG }),
    ).rejects.toMatchObject({ code: 'ADMIN_INVITE_CONSUMED', routesTo: 'InviteExpired' });
  });

  it('rejects a uv=0 registration with ADMIN_WEBAUTHN_UV_REQUIRED and does NOT consume the invite (R11)', async () => {
    const h = liveHarness();
    await h.challenges.put('reg', 'chal', 600);
    vi.mocked(verifyRegistrationResponse).mockResolvedValue(regResult({ userVerified: false }));

    await expect(
      verifyRegistration(h.deps, { ceremonyKey: 'reg', presentedToken: 'tok', response: DUMMY_REG }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_UV_REQUIRED' });
    expect(h.invites.isConsumed('tok')).toBe(false);
  });

  it('rejects verified=false with ADMIN_WEBAUTHN_VERIFICATION_FAILED', async () => {
    const h = liveHarness();
    await h.challenges.put('reg', 'chal', 600);
    vi.mocked(verifyRegistrationResponse).mockResolvedValue(regResult({ verified: false }));

    await expect(
      verifyRegistration(h.deps, { ceremonyKey: 'reg', presentedToken: 'tok', response: DUMMY_REG }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' });
  });

  it('when the verifier throws: VERIFICATION_FAILED, invite stays unconsumed, challenge was burned', async () => {
    const h = liveHarness();
    await h.challenges.put('reg', 'chal', 600);
    vi.mocked(verifyRegistrationResponse).mockRejectedValue(new Error('bad attestation'));

    await expect(
      verifyRegistration(h.deps, { ceremonyKey: 'reg', presentedToken: 'tok', response: DUMMY_REG }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' });
    expect(h.invites.isConsumed('tok')).toBe(false); // AC16: only consumed on SUCCESS
    expect(h.challenges.has('reg')).toBe(false); // the one-time challenge is gone either way
  });
});

describe('verifyAuthentication orchestration (mocked verifier)', () => {
  const DUMMY_AUTH = { id: 'cred-X' } as unknown as AuthenticationResponseJSON;

  async function harnessWithCredential(): Promise<Harness> {
    const h = makeHarness(NOW);
    await h.credentials.insert(existingCred('cred-X'));
    await h.challenges.put('auth', 'chal', 600);
    return h;
  }

  it('on success: bumps the stored signature counter (anti-clone)', async () => {
    const h = await harnessWithCredential();
    vi.mocked(verifyAuthenticationResponse).mockResolvedValue(authResult({ newCounter: 5 }));

    const outcome = await verifyAuthentication(h.deps, { ceremonyKey: 'auth', response: DUMMY_AUTH });

    expect(outcome).toMatchObject({ adminId: 'admin-1', credentialId: 'cred-X', newSignCount: 5 });
    expect((await h.credentials.findActive('cred-X'))?.signCount).toBe(5);
  });

  it('rejects a uv=0 assertion with ADMIN_WEBAUTHN_UV_REQUIRED (R11 — regardless of client request)', async () => {
    const h = await harnessWithCredential();
    vi.mocked(verifyAuthenticationResponse).mockResolvedValue(authResult({ userVerified: false }));

    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'auth', response: DUMMY_AUTH }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_UV_REQUIRED' });
  });

  it('rejects verified=false with ADMIN_WEBAUTHN_VERIFICATION_FAILED', async () => {
    const h = await harnessWithCredential();
    vi.mocked(verifyAuthenticationResponse).mockResolvedValue(authResult({ verified: false }));

    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'auth', response: DUMMY_AUTH }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' });
  });

  it('maps a cloned-authenticator counter-regression throw to ADMIN_WEBAUTHN_VERIFICATION_FAILED', async () => {
    const h = await harnessWithCredential();
    // The library throws on a sign-count regression; we must surface a clean verification failure.
    vi.mocked(verifyAuthenticationResponse).mockRejectedValue(
      new Error('Response counter value 3 was lower than expected 5'),
    );

    await expect(
      verifyAuthentication(h.deps, { ceremonyKey: 'auth', response: DUMMY_AUTH }),
    ).rejects.toMatchObject({ code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' });
  });
});
