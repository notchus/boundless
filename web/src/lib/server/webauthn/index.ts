// Admin WebAuthn edge-verification module (spec 001 T09; ADR-0017).
//
// Framework-agnostic, runs in the Cloudflare Workers runtime and in Node (so it is fully
// unit-testable). The deployable SvelteKit `+server.ts` routes + the real KV/Postgres port
// implementations + the post-assertion session cookie are the shell (T15 / DEFERRED.md).
//
// This module performs NO logging (it returns outcomes / throws WebAuthnError); the shell
// logs via `boundless::logging::emit()` and never logs the invite token or any secret (P2).

export type { RpConfig } from './config';
export {
  WEBAUTHN_ERROR_CODES,
  WebAuthnError,
  type RoutesTo,
  type WebAuthnErrorCode,
} from './errors';
export { evaluateInvite, type InviteVerdict } from './invite';
export type {
  ChallengeStore,
  Clock,
  CredentialStore,
  InviteRecord,
  InviteStore,
  StoredCredential,
  WebAuthnDeps,
} from './ports';
export {
  buildRegistrationOptions,
  CHALLENGE_TTL_SECS,
  verifyRegistration,
  type BuildRegistrationArgs,
  type RegistrationOutcome,
  type VerifyRegistrationArgs,
} from './register';
export {
  buildAuthenticationOptions,
  verifyAuthentication,
  type AuthenticationOutcome,
  type BuildAuthenticationArgs,
  type VerifyAuthenticationArgs,
} from './authenticate';
