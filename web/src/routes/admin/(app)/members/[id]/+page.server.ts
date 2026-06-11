// Member detail (spec 008 T10). `load` performs the AUDITED PII read (I5 — every view writes an
// audit_log row on the Worker); `edit` updates under optimistic concurrency (`expected_updated_at`;
// stale → calm copy, AC11); `regenerate` mints a fresh Onboarding Code (AC6). All calls carry the
// verified `adminId` (ADR-0026); typed PII is forwarded to the Worker over TLS, never logged (P2/R10).

import { fail } from '@sveltejs/kit';

import { getMembersClient } from '$lib/server/members-deps';
import type { EditMemberRequest, IssuableRole } from '$lib/server/members';
import { requireAdminId } from '$lib/server/session';
import type { Actions, PageServerLoad } from './$types';

function asIssuableRoles(values: FormDataEntryValue[]): IssuableRole[] {
	return values.map(String).filter((r): r is IssuableRole => r === 'rider' || r === 'driver');
}

export const load: PageServerLoad = async ({ params, url, cookies }) => {
	const adminId = requireAdminId(cookies);
	const outcome = await getMembersClient().detail(adminId, params.id);
	const openEdit = url.searchParams.get('edit') === '1';
	if (outcome.kind === 'detail') return { member: outcome.detail, errorCode: null, openEdit };
	if (outcome.kind === 'not_found')
		return { member: null, errorCode: 'ADMIN_MEMBER_NOT_FOUND' as const, openEdit: false };
	return { member: null, errorCode: 'ADMIN_GROUP_KEY_MISSING' as const, openEdit: false };
};

export const actions: Actions = {
	edit: async ({ request, params, cookies }) => {
		const adminId = requireAdminId(cookies);
		const fd = await request.formData();
		const req: EditMemberRequest = {
			name: String(fd.get('name') ?? '').trim(),
			phone: String(fd.get('phone') ?? '').trim(),
			address: String(fd.get('address') ?? '').trim(),
			roles: asIssuableRoles(fd.getAll('roles')),
			expected_updated_at: Number(fd.get('expected_updated_at') ?? 0),
		};
		const outcome = await getMembersClient().edit(adminId, params.id, req);
		switch (outcome.kind) {
			case 'updated':
				return { ok: true as const, updated: outcome.detail };
			case 'stale':
				return fail(409, { errorCode: 'ADMIN_MEMBER_EDIT_STALE' });
			case 'rejected':
				return fail(400, { errorCode: outcome.code });
			case 'not_found':
				return fail(404, { errorCode: 'ADMIN_MEMBER_NOT_FOUND' });
			case 'group_key_missing':
				return fail(503, { errorCode: 'ADMIN_GROUP_KEY_MISSING' });
			default: {
				const _exhaustive: never = outcome;
				throw new Error(`unhandled edit outcome: ${String(_exhaustive)}`);
			}
		}
	},

	regenerate: async ({ params, cookies }) => {
		const adminId = requireAdminId(cookies);
		const outcome = await getMembersClient().regenerateCode(adminId, params.id);
		if (outcome.kind === 'regenerated')
			return {
				ok: true as const,
				regenerated: {
					onboarding_code: outcome.onboarding_code ?? null,
					code_expires_at: outcome.code_expires_at,
				},
			};
		return fail(404, { errorCode: 'ADMIN_MEMBER_NOT_FOUND' });
	},
};
