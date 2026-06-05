// Admin WebAuthn **registration** — options + verification (ADR-0016 D4, ADR-0017; AC16/AC20).
// Pure orchestration over the ports; the real @simplewebauthn/server verifier does the crypto.

import { generateRegistrationOptions, verifyRegistrationResponse } from '@simplewebauthn/server';
import type {
  PublicKeyCredentialCreationOptionsJSON,
  RegistrationResponseJSON,
} from '@simplewebauthn/server';

import { WebAuthnError } from './errors';
import { requireLiveInvite } from './invite';
import type { WebAuthnDeps } from './ports';

/** WebAuthn challenge TTL in KV (ADR-0017 D3). */
export const CHALLENGE_TTL_SECS = 300;
const REGISTRATION_TIMEOUT_MS = 300_000;
/** COSE algorithm IDs offered, most-preferred first: EdDSA (-8), ES256 (-7), RS256 (-257). */
const SUPPORTED_ALGORITHM_IDS = [-8, -7, -257];

export interface BuildRegistrationArgs {
  /** Stable key for the KV challenge round-trip (shell-held, e.g. a cookie-bound id). */
  readonly ceremonyKey: string;
  /** The opaque invitation token presented on the registration link. */
  readonly presentedToken: string;
  /** WebAuthn user handle name (protocol identifier shown by the OS passkey UI; no PII). */
  readonly userName: string;
  readonly userDisplayName: string;
}

export interface VerifyRegistrationArgs {
  readonly ceremonyKey: string;
  readonly presentedToken: string;
  readonly response: RegistrationResponseJSON;
}

export interface RegistrationOutcome {
  readonly adminId: string;
  /** base64url credential id now stored against the admin. */
  readonly credentialId: string;
}

/**
 * Build registration ceremony options, baking in the ADR-0016 D4 policy:
 * `attestation: none`, `residentKey: preferred` (discoverable/usernameless), `userVerification:
 * required`. `excludeCredentials` lists the admin's existing active credentials so the same
 * authenticator isn't double-registered (a *different* backup key is still allowed → multi-cred).
 * Fails fast (InviteExpired/Consumed) if the invite is dead, so no ceremony is wasted.
 */
export async function buildRegistrationOptions(
  deps: WebAuthnDeps,
  args: BuildRegistrationArgs,
): Promise<PublicKeyCredentialCreationOptionsJSON> {
  const invite = await requireLiveInvite(deps.invites, args.presentedToken, deps.clock.now());
  const existing = await deps.credentials.listActiveByAdmin(invite.adminId);

  const options = await generateRegistrationOptions({
    rpName: deps.rp.rpName,
    rpID: deps.rp.rpID,
    // The WebAuthn user handle is the opaque admin id (no PII), ≤64 bytes.
    userID: new TextEncoder().encode(invite.adminId),
    userName: args.userName,
    userDisplayName: args.userDisplayName,
    attestationType: 'none',
    authenticatorSelection: {
      residentKey: 'preferred',
      userVerification: 'required',
    },
    supportedAlgorithmIDs: SUPPORTED_ALGORITHM_IDS,
    excludeCredentials: existing.map((c) => ({ id: c.credentialId, transports: c.transports })),
    timeout: REGISTRATION_TIMEOUT_MS,
  });

  await deps.challenges.put(args.ceremonyKey, options.challenge, CHALLENGE_TTL_SECS);
  return options;
}

/**
 * Verify a registration response and, on success, **consume the invite** (single-use, AC16),
 * **revoke the admin's prior credentials** (ADR-0016 D4 recovery), and persist the new credential.
 *
 * This is the **invite-gated** registration path: the initial registration (no priors → the revoke
 * is a no-op) and **lost-key recovery** (a Developer re-invite revokes the prior credential(s),
 * ADR-0016 D4). It is therefore revoke-and-replace by design. The other half of AC20 — enrolling an
 * *additional* backup key without revoking the first ("multiple credentials per Admin are
 * encouraged") — is an **authenticated** add-credential flow (the admin is already signed in, no
 * invite): it needs the post-assertion session, which is the deferred shell, so it lands with T15.
 * See DEFERRED.md → Server / admin-WebAuthn (T09). The store layer already supports >1 active
 * credential per admin (no revoke on a non-invite insert) — only this entry point is invite-gated.
 *
 * UV enforcement: we verify with `requireUserVerification: false` and then reject `uv=0`
 * ourselves, so a missing user-verification flag yields the distinct `ADMIN_WEBAUTHN_UV_REQUIRED`
 * code (R11) rather than a generic failure.
 */
export async function verifyRegistration(
  deps: WebAuthnDeps,
  args: VerifyRegistrationArgs,
): Promise<RegistrationOutcome> {
  const expectedChallenge = await deps.challenges.take(args.ceremonyKey);
  if (expectedChallenge == null) {
    throw new WebAuthnError('ADMIN_WEBAUTHN_CHALLENGE_EXPIRED');
  }

  // Invite must still be live (and gives us the adminId to bind the credential to).
  const invite = await requireLiveInvite(deps.invites, args.presentedToken, deps.clock.now());

  let verification;
  try {
    verification = await verifyRegistrationResponse({
      response: args.response,
      expectedChallenge,
      expectedOrigin: deps.rp.origin,
      expectedRPID: deps.rp.rpID,
      requireUserVerification: false,
    });
  } catch {
    throw new WebAuthnError('ADMIN_WEBAUTHN_VERIFICATION_FAILED');
  }
  if (!verification.verified || !verification.registrationInfo) {
    throw new WebAuthnError('ADMIN_WEBAUTHN_VERIFICATION_FAILED');
  }
  const info = verification.registrationInfo;
  if (!info.userVerified) {
    throw new WebAuthnError('ADMIN_WEBAUTHN_UV_REQUIRED');
  }

  const now = deps.clock.now();
  await deps.invites.markConsumed(args.presentedToken, now);
  await deps.credentials.revokeAllForAdmin(invite.adminId, now);
  await deps.credentials.insert({
    credentialId: info.credential.id,
    adminId: invite.adminId,
    publicKey: info.credential.publicKey,
    signCount: info.credential.counter,
    transports: info.credential.transports,
    aaguid: info.aaguid,
    revokedAt: null,
  });

  return { adminId: invite.adminId, credentialId: info.credential.id };
}
