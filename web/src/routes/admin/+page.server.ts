// Signed-in admin home placeholder (spec 001 T15). The real admin dashboard / member-management UI
// is spec 008; this only proves the post-assertion session works. No session → back to sign-in.

import { redirect } from '@sveltejs/kit';

import { ADMIN_SESSION_COOKIE, getSession } from '$lib/server/session';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = ({ cookies }) => {
	if (getSession(cookies.get(ADMIN_SESSION_COOKIE)) === null) {
		redirect(307, '/admin/signin');
	}
	return {};
};
