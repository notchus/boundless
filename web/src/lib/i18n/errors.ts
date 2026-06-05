// Maps an admin-auth error CODE to the catalog message key a person sees (docs/error-codes.md).
//
// Client-safe by construction: the keys are plain code STRINGS (not the `WebAuthnErrorCode` type from
// `$lib/server/webauthn`, which is server-only and must never be imported into client-bundled code).
// `src/lib/i18n/errors.test.ts` (node) asserts this map's keys are exactly the T09
// `WEBAUTHN_ERROR_CODES` and that each maps to the key documented in error-codes.md — so it cannot
// silently drift from the server's error set.

import type { MessageKey } from './catalog';

/** Per error-codes.md → "Catalog key": the two invite codes → InviteExpired copy; the three
 * WebAuthn ceremony codes → the registration re-prompt (retry). */
export const ERROR_CATALOG_KEYS: Record<string, MessageKey> = {
	ADMIN_INVITE_EXPIRED: 'admin.onboarding.invite_expired',
	ADMIN_INVITE_CONSUMED: 'admin.onboarding.invite_expired',
	ADMIN_WEBAUTHN_UV_REQUIRED: 'admin.onboarding.register_credential',
	ADMIN_WEBAUTHN_VERIFICATION_FAILED: 'admin.onboarding.register_credential',
	ADMIN_WEBAUTHN_CHALLENGE_EXPIRED: 'admin.onboarding.register_credential',
};

/**
 * The catalog key for an error code. Unknown/unexpected codes fall back to the registration
 * re-prompt rather than leaking a raw code to the user (the sign-in page wraps ceremony failures in
 * its own retry copy, so this fallback is only ever a defensive default).
 */
export function errorCatalogKey(code: string): MessageKey {
	return ERROR_CATALOG_KEYS[code] ?? 'admin.onboarding.register_credential';
}
