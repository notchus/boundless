// Maps an admin member-management error CODE (docs/error-codes.md spec-008) to the catalog message key
// a person sees. SEPARATE from `errors.ts` (the WebAuthn map) on purpose: `errors.test.ts` asserts that
// map's keys are EXACTLY the T09 WebAuthn set, so the member codes get their own map + parity test here.
//
// Client-safe by construction: plain code STRINGS in, a `MessageKey` out — no server-only import.
// `member-errors.test.ts` asserts this covers exactly the spec-008 member codes and that each maps to a
// key that exists in the catalog, so it cannot silently drift from `docs/error-codes.md`.
//
// The BFF-gate codes (`ADMIN_UNAUTHORIZED`/`ADMIN_BAD_REQUEST`, ADR-0026) are deliberately NOT here:
// the BFF always presents a valid shared secret, so those are operator faults the client never surfaces
// (the `WorkerMembersClient` throws on a 401 rather than rendering it).

import type { MessageKey } from './catalog';

export const MEMBER_ERROR_CATALOG_KEYS: Record<string, MessageKey> = {
	ADMIN_MEMBER_PHONE_INVALID: 'admin.member.phone_invalid',
	ADMIN_MEMBER_ADDRESS_INVALID: 'admin.member.address_invalid',
	ADMIN_MEMBER_ROLES_REQUIRED: 'admin.member.roles_required',
	ADMIN_MEMBER_DUPLICATE_PHONE: 'admin.member.duplicate_phone',
	ADMIN_MEMBER_EDIT_STALE: 'admin.member.edit_stale',
	ADMIN_MEMBER_NOT_FOUND: 'admin.member.not_found',
	ADMIN_GROUP_KEY_MISSING: 'admin.member.group_key_missing',
	// No client surface (the admin UI never offers the Admin role, I11/AC10); mapped to a calm generic
	// only as a defensive default if a tampered request ever provokes it.
	ADMIN_MEMBER_ROLE_FORBIDDEN: 'admin.member.error_generic',
};

/**
 * The catalog key for a member error code. Unknown codes fall back to a calm generic rather than
 * leaking a raw code to the user.
 */
export function memberErrorCatalogKey(code: string): MessageKey {
	return MEMBER_ERROR_CATALOG_KEYS[code] ?? 'admin.member.error_generic';
}
