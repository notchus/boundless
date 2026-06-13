// Authenticated admin-app layout gate (spec 008 T10). The `(app)` route group holds the member
// dashboard (`/admin/members`, `/admin/audit-log`) behind the post-assertion admin session (ADR-0017 /
// §10-F). The gate runs once here for the whole group; child loads/actions get the acting `adminId` via
// `await parent()` (it becomes the `X-Admin-Id` actor on the I5 audit row, ADR-0026). The existing
// unauthenticated onboarding routes (`/admin`, `/admin/signin`, `/admin/onboard/[token]`) sit OUTSIDE
// this group and are unaffected.

import { redirect } from '@sveltejs/kit';

import { ADMIN_SESSION_COOKIE, getSession } from '$lib/server/session';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ cookies, locals, platform }) => {
	const session = await getSession(cookies.get(ADMIN_SESSION_COOKIE), platform);
	if (session === null) redirect(307, '/admin/signin');
	return { locale: locals.locale, adminId: session.adminId };
};
