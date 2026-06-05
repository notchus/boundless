// Admin WebAuthn **assertion** (sign-in) — options + verification (ADR-0016 D4; AC2/AC20).
// Sign-in is assertion-only — there is no password anywhere (AC2). Pure orchestration over
// the ports; the real @simplewebauthn/server verifier does the crypto.

import { generateAuthenticationOptions, verifyAuthenticationResponse } from '@simplewebauthn/server';
import type {
  AuthenticationResponseJSON,
  PublicKeyCredentialRequestOptionsJSON,
} from '@simplewebauthn/server';

import { WebAuthnError } from './errors';
import { CHALLENGE_TTL_SECS } from './register';
import type { WebAuthnDeps } from './ports';

const AUTHENTICATION_TIMEOUT_MS = 300_000;

export interface BuildAuthenticationArgs {
  readonly ceremonyKey: string;
  /** Optional: scope the assertion to one admin's credentials. Omit for usernameless/discoverable sign-in. */
  readonly adminId?: string;
}

export interface VerifyAuthenticationArgs {
  readonly ceremonyKey: string;
  readonly response: AuthenticationResponseJSON;
}

export interface AuthenticationOutcome {
  readonly adminId: string;
  readonly credentialId: string;
  readonly newSignCount: number;
}

/** Build assertion options with `userVerification: required` (ADR-0016 D4). */
export async function buildAuthenticationOptions(
  deps: WebAuthnDeps,
  args: BuildAuthenticationArgs,
): Promise<PublicKeyCredentialRequestOptionsJSON> {
  const allowCredentials =
    args.adminId === undefined
      ? undefined
      : (await deps.credentials.listActiveByAdmin(args.adminId)).map((c) => ({
          id: c.credentialId,
          transports: c.transports,
        }));

  const options = await generateAuthenticationOptions({
    rpID: deps.rp.rpID,
    userVerification: 'required',
    timeout: AUTHENTICATION_TIMEOUT_MS,
    ...(allowCredentials === undefined ? {} : { allowCredentials }),
  });

  await deps.challenges.put(args.ceremonyKey, options.challenge, CHALLENGE_TTL_SECS);
  return options;
}

/**
 * Verify an assertion and bump the stored signature counter. UV is enforced by us (verify
 * with `requireUserVerification: false`, then reject `uv=0`) so a missing user-verification
 * flag yields `ADMIN_WEBAUTHN_UV_REQUIRED` **regardless of what the client requested** (R11).
 * An unknown/revoked credential or a bad signature yields `ADMIN_WEBAUTHN_VERIFICATION_FAILED`.
 */
export async function verifyAuthentication(
  deps: WebAuthnDeps,
  args: VerifyAuthenticationArgs,
): Promise<AuthenticationOutcome> {
  const expectedChallenge = await deps.challenges.take(args.ceremonyKey);
  if (expectedChallenge == null) {
    throw new WebAuthnError('ADMIN_WEBAUTHN_CHALLENGE_EXPIRED');
  }

  const stored = await deps.credentials.findActive(args.response.id);
  if (stored == null) {
    throw new WebAuthnError('ADMIN_WEBAUTHN_VERIFICATION_FAILED');
  }

  let verification;
  try {
    verification = await verifyAuthenticationResponse({
      response: args.response,
      expectedChallenge,
      expectedOrigin: deps.rp.origin,
      expectedRPID: deps.rp.rpID,
      requireUserVerification: false,
      credential: {
        id: stored.credentialId,
        publicKey: stored.publicKey,
        counter: stored.signCount,
        transports: stored.transports,
      },
    });
  } catch {
    throw new WebAuthnError('ADMIN_WEBAUTHN_VERIFICATION_FAILED');
  }
  if (!verification.verified || !verification.authenticationInfo) {
    throw new WebAuthnError('ADMIN_WEBAUTHN_VERIFICATION_FAILED');
  }
  if (!verification.authenticationInfo.userVerified) {
    throw new WebAuthnError('ADMIN_WEBAUTHN_UV_REQUIRED');
  }

  const newSignCount = verification.authenticationInfo.newCounter;
  await deps.credentials.bumpSignCount(stored.credentialId, newSignCount, deps.clock.now());
  return { adminId: stored.adminId, credentialId: stored.credentialId, newSignCount };
}
