// Audit log (spec 008 T10 / AC9) — the first-class, not-buried view of every admin PII read: who, when,
// which FIELD NAMES (never values), and the request id. Reading the log is itself NOT an audited read
// (it discloses no PII). Optionally filtered to one member via `?member_id=`.

import { getMembersClient } from '$lib/server/members-deps';
import { requireAdminId } from '$lib/server/session';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ url, cookies }) => {
	const adminId = requireAdminId(cookies);
	const memberId = url.searchParams.get('member_id') ?? undefined;
	const entries = await getMembersClient().auditLog(adminId, { memberId });
	return { entries, memberId: memberId ?? '' };
};
