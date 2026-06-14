// DEV-ONLY test seam: mint an admin session + set the §10-F cookie directly, so the Playwright member
// e2e can drive the authenticated `(app)` surface without re-running the full WebAuthn ceremony each
// test (that ceremony is covered by `admin-onboarding.spec.ts`). Hard-gated on `dev` (404 in any
// production build) AND tree-shaken from the prod bundle — proven by tests/build-gates/no-dev-seams.test.ts
// (AC5), so this session-minting seam is unreachable in prod (R21/I11). Kept under spec 009 T07 Option A;
// it writes to the dev-durable session backend (the Miniflare KV in `vite dev`). Mirrors the real
// sign-in's cookie, just without the assertion.

import { dev } from '$app/environment';
import { error, json } from '@sveltejs/kit';

import { ADMIN_SESSION_COOKIE, createSession, SESSION_COOKIE_OPTIONS } from '$lib/server/session';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, cookies, platform }) => {
	if (!dev) error(404);

	const body = (await request.json().catch(() => ({}))) as { adminId?: string };
	const adminId = typeof body.adminId === 'string' && body.adminId ? body.adminId : 'admin-e2e';
	cookies.set(ADMIN_SESSION_COOKIE, await createSession(adminId, platform), SESSION_COOKIE_OPTIONS);
	return json({ ok: true, adminId });
};
