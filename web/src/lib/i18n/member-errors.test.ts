// Member error-code → catalog-key parity (spec 008 T10). Asserts the member-error i18n map covers
// exactly the spec-008 member codes (docs/error-codes.md) and that each maps to an existing catalog
// key — so it cannot silently drift from the registry. Mirrors `errors.test.ts` (the WebAuthn map).

import { describe, expect, it } from 'vitest';

import { en } from './catalog';
import { MEMBER_ERROR_CATALOG_KEYS, memberErrorCatalogKey } from './member-errors';

// The spec-008 member-management codes a person can SEE (docs/error-codes.md "Admin member-management
// — issuance"), EXCLUDING the ADR-0026 BFF-gate codes (ADMIN_UNAUTHORIZED/ADMIN_BAD_REQUEST), which the
// BFF never surfaces to the user. Hardcoded here (the codes are Rust-side); the reviewer + error-codes.md
// are the source of truth, so a new UI-facing member code must be added in both places.
const EXPECTED_MEMBER_UI_CODES = [
	'ADMIN_MEMBER_PHONE_INVALID',
	'ADMIN_MEMBER_ADDRESS_INVALID',
	'ADMIN_MEMBER_ROLES_REQUIRED',
	'ADMIN_MEMBER_DUPLICATE_PHONE',
	'ADMIN_MEMBER_EDIT_STALE',
	'ADMIN_MEMBER_NOT_FOUND',
	'ADMIN_GROUP_KEY_MISSING',
	'ADMIN_MEMBER_ROLE_FORBIDDEN',
];

describe('admin member error-code → catalog-key map', () => {
	it('covers exactly the spec-008 UI-facing member codes (no drift)', () => {
		expect(new Set(Object.keys(MEMBER_ERROR_CATALOG_KEYS))).toEqual(new Set(EXPECTED_MEMBER_UI_CODES));
	});

	it('maps every code to a key that exists in the catalog', () => {
		for (const code of EXPECTED_MEMBER_UI_CODES) {
			expect(en[memberErrorCatalogKey(code)]).toBeDefined();
		}
	});

	it('matches docs/error-codes.md for the representative codes', () => {
		expect(memberErrorCatalogKey('ADMIN_MEMBER_PHONE_INVALID')).toBe('admin.member.phone_invalid');
		expect(memberErrorCatalogKey('ADMIN_MEMBER_DUPLICATE_PHONE')).toBe('admin.member.duplicate_phone');
		expect(memberErrorCatalogKey('ADMIN_MEMBER_EDIT_STALE')).toBe('admin.member.edit_stale');
		expect(memberErrorCatalogKey('ADMIN_GROUP_KEY_MISSING')).toBe('admin.member.group_key_missing');
	});

	it('falls back to a calm generic for an unknown code rather than leaking the raw code', () => {
		expect(memberErrorCatalogKey('SOMETHING_UNEXPECTED')).toBe('admin.member.error_generic');
	});
});
