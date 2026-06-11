// Member list + issuance (spec 008 T10). The BFF: `load` lists the group's members (PII-free
// summaries, server-side filtered via the frozen `?search=&role=&status=` params); the `issue` action
// creates a member through the Worker and returns the show-once Onboarding Code / the duplicate link /
// a validation reject. All calls carry the WebAuthn-verified `adminId` (ADR-0026); the raw PII the admin
// typed is forwarded to the Worker over TLS and never logged here (P2/R10).

import { fail } from '@sveltejs/kit';

import { getMembersClient } from '$lib/server/members-deps';
import type { IssuableRole, OnboardingStatus, Role } from '$lib/server/members';
import { requireAdminId } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';

const ROLES: readonly string[] = ['rider', 'driver', 'admin'];
const STATUSES: readonly string[] = [
	'issued_not_onboarded',
	'onboarded',
	'code_expired_or_lost',
	'needs_reonboarding',
];

function asRole(v: string | null): Role | undefined {
	return v && ROLES.includes(v) ? (v as Role) : undefined;
}
function asStatus(v: string | null): OnboardingStatus | undefined {
	return v && STATUSES.includes(v) ? (v as OnboardingStatus) : undefined;
}
function asIssuableRoles(values: FormDataEntryValue[]): IssuableRole[] {
	return values.map(String).filter((r): r is IssuableRole => r === 'rider' || r === 'driver');
}

export const load: PageServerLoad = async ({ url, cookies }) => {
	const adminId = requireAdminId(cookies);
	const search = url.searchParams.get('search')?.trim() || undefined;
	const role = asRole(url.searchParams.get('role'));
	const status = asStatus(url.searchParams.get('status'));
	const members = await getMembersClient().list(adminId, { search, role, status });
	return {
		members,
		filters: { search: search ?? '', role: role ?? '', status: status ?? '' },
	};
};

export const actions: Actions = {
	issue: async ({ request, cookies }) => {
		const adminId = requireAdminId(cookies);
		const fd = await request.formData();
		const name = String(fd.get('name') ?? '').trim();
		const phone = String(fd.get('phone') ?? '').trim();
		const address = String(fd.get('address') ?? '').trim();
		const roles = asIssuableRoles(fd.getAll('roles'));

		const outcome = await getMembersClient().issue(adminId, { name, phone, address, roles });
		switch (outcome.kind) {
			case 'issued':
				return {
					ok: true as const,
					issued: {
						member: outcome.member,
						onboarding_code: outcome.onboarding_code ?? null,
						code_expires_at: outcome.code_expires_at,
					},
				};
			case 'duplicate':
				return fail(409, { errorCode: 'ADMIN_MEMBER_DUPLICATE_PHONE', existing: outcome.existing });
			case 'rejected':
				return fail(400, { errorCode: outcome.code });
			case 'group_key_missing':
				return fail(503, { errorCode: 'ADMIN_GROUP_KEY_MISSING' });
			default: {
				const _exhaustive: never = outcome;
				throw new Error(`unhandled issue outcome: ${String(_exhaustive)}`);
			}
		}
	},
};
