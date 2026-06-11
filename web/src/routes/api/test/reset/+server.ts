// DEV-ONLY test seam: reset the interim in-memory backends (WebAuthn invites/credentials/challenges +
// the spec-008 member backend) and admin sessions for per-test isolation. Hard-gated on `dev` (404 in
// any production build).

import { dev } from '$app/environment';
import { error, json } from '@sveltejs/kit';

import { resetMembers } from '$lib/server/members-deps';
import { resetSessions } from '$lib/server/session';
import { resetStores } from '$lib/server/webauthn-deps';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async () => {
	if (!dev) error(404);
	resetStores();
	resetSessions();
	resetMembers();
	return json({ ok: true });
};
