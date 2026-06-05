// Error-code → catalog-key parity (spec 001 T15). Asserts the client-safe i18n map cannot drift from
// the server's WebAuthn error set (T09 `WEBAUTHN_ERROR_CODES`) nor from docs/error-codes.md.
// Relative imports (no `$lib` alias in Vitest); importing `../server/webauthn` is fine in the node
// test runner (it is never bundled into the client — that is the whole point of this guard).

import { describe, expect, it } from 'vitest';

import { WEBAUTHN_ERROR_CODES } from '../server/webauthn';
import { en } from './catalog';
import { ERROR_CATALOG_KEYS, errorCatalogKey } from './errors';

describe('admin error-code → catalog-key map', () => {
	it('covers exactly the T09 WebAuthn error codes (no drift)', () => {
		expect(new Set(Object.keys(ERROR_CATALOG_KEYS))).toEqual(new Set(WEBAUTHN_ERROR_CODES));
	});

	it('maps every code to a key that exists in the catalog', () => {
		for (const code of WEBAUTHN_ERROR_CODES) {
			const key = errorCatalogKey(code);
			expect(en[key]).toBeDefined();
		}
	});

	it('matches docs/error-codes.md: invite codes → invite_expired, ceremony codes → register_credential', () => {
		expect(errorCatalogKey('ADMIN_INVITE_EXPIRED')).toBe('admin.onboarding.invite_expired');
		expect(errorCatalogKey('ADMIN_INVITE_CONSUMED')).toBe('admin.onboarding.invite_expired');
		expect(errorCatalogKey('ADMIN_WEBAUTHN_UV_REQUIRED')).toBe('admin.onboarding.register_credential');
		expect(errorCatalogKey('ADMIN_WEBAUTHN_VERIFICATION_FAILED')).toBe(
			'admin.onboarding.register_credential',
		);
		expect(errorCatalogKey('ADMIN_WEBAUTHN_CHALLENGE_EXPIRED')).toBe(
			'admin.onboarding.register_credential',
		);
	});

	it('falls back defensively for an unknown code rather than leaking the raw code', () => {
		expect(errorCatalogKey('SOMETHING_UNEXPECTED')).toBe('admin.onboarding.register_credential');
	});
});
