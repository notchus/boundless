// POST /api/admin/auth/signout — end the admin session (spec 009 T12, AC10).
//
// Revokes the durable session (deletes the `ADMIN_SESSIONS` KV key — best-effort within KV's
// propagation window, D5/R3) AND clears the httpOnly session cookie, then redirects to sign-in. A
// request that re-presents the now-revoked cookie resolves to no session, so the `(app)` layout gate
// (+layout.server.ts) redirects it back to `/admin/signin` — the assertion the deployed-edge smoke
// (smoke-deployed-admin-web.sh) drives.
//
// POST-only: the session cookie is SameSite=Strict, so this is not a CSRF target; keeping it a POST
// (no GET) means a cross-site or pre-fetched link can't silently sign an admin out.

import { redirect } from '@sveltejs/kit';

import { ADMIN_SESSION_COOKIE, revokeSession } from '$lib/server/session';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ cookies, platform }) => {
	// Delete the KV key first (revokeSession no-ops on an absent cookie), then drop the cookie. The
	// delete path MUST match SESSION_COOKIE_OPTIONS.path ('/') or the browser keeps the stale cookie.
	await revokeSession(cookies.get(ADMIN_SESSION_COOKIE), platform);
	cookies.delete(ADMIN_SESSION_COOKIE, { path: '/' });
	redirect(303, '/admin/signin');
};
