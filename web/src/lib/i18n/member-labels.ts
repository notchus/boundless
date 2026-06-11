// Catalog-key helpers for the member onboarding-status + role enums (spec 008 T10). Keeps the
// enum→key mapping in one place so the list, detail, and filter UIs render the same labels (P8).

import type { OnboardingStatus, Role } from '../members-types';
import type { MessageKey } from './catalog';

/** The catalog key for an onboarding status (the 4 `admin.member.status_*` keys). */
export function statusKey(s: OnboardingStatus): MessageKey {
	return `admin.member.status_${s}` as MessageKey;
}

/** The catalog key for a role label. Exhaustive over `Role` so no role is ever mislabelled: the list
 *  excludes Admins (I11), but a dual-role member (e.g. a Rider who is also an Admin — multi-role is
 *  supported, glossary) viewed by id carries `admin` in `roles`, and it must read "Admin", not "Rider".
 *  A future 4th role becomes a compile error here. */
export function roleKey(r: Role): MessageKey {
	switch (r) {
		case 'driver':
			return 'admin.member.role_driver';
		case 'admin':
			return 'admin.member.role_admin';
		case 'rider':
			return 'admin.member.role_rider';
	}
}
