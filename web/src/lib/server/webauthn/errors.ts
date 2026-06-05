// Admin-WebAuthn edge error codes. These MUST match docs/error-codes.md (P12) — the
// `webauthn_error_codes_match_registry` test asserts the set below is a subset of the
// registered `ADMIN_*` codes. No new codes are introduced by T09.
//
// The codes are runtime values (an array, not just a TS union) so the registry-parity test
// can introspect them; the union type is derived from the array.

/** Every error this module can raise. Order is irrelevant; membership is what's asserted. */
export const WEBAUTHN_ERROR_CODES = [
  'ADMIN_INVITE_EXPIRED',
  'ADMIN_INVITE_CONSUMED',
  'ADMIN_WEBAUTHN_UV_REQUIRED',
  'ADMIN_WEBAUTHN_VERIFICATION_FAILED',
  'ADMIN_WEBAUTHN_CHALLENGE_EXPIRED',
] as const;

export type WebAuthnErrorCode = (typeof WEBAUTHN_ERROR_CODES)[number];

/**
 * Client routing for each code (mirrors the `Routes to` column in docs/error-codes.md):
 * a dead invite link sends the admin to the terminal `InviteExpired` screen; a WebAuthn
 * ceremony failure keeps them on `register_credential` to retry.
 */
export type RoutesTo = 'InviteExpired' | 'register_credential';

const ROUTES: Record<WebAuthnErrorCode, RoutesTo> = {
  ADMIN_INVITE_EXPIRED: 'InviteExpired',
  ADMIN_INVITE_CONSUMED: 'InviteExpired',
  ADMIN_WEBAUTHN_UV_REQUIRED: 'register_credential',
  ADMIN_WEBAUTHN_VERIFICATION_FAILED: 'register_credential',
  ADMIN_WEBAUTHN_CHALLENGE_EXPIRED: 'register_credential',
};

/**
 * A verification failure carrying a stable error code + its client route. Carries no PII
 * and no secret material (the message is the code itself) — safe to surface to the shell,
 * which maps the code to a catalog string and logs via `boundless::logging::emit()` (P2).
 */
export class WebAuthnError extends Error {
  readonly code: WebAuthnErrorCode;
  readonly routesTo: RoutesTo;

  constructor(code: WebAuthnErrorCode) {
    super(code);
    this.name = 'WebAuthnError';
    this.code = code;
    this.routesTo = ROUTES[code];
  }
}
