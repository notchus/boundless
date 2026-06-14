// Admin sign-out — full-stack e2e (spec 009 T12; AC10 sign-out leg + AC11 Referrer-Policy integration).
//
// Drives the REAL SvelteKit routes under `vite dev`: mint a session via the dev `/api/test/seed-session`
// seam (the WebAuthn ceremony is covered by webauthn/admin-onboarding specs), confirm the `(app)` gate
// lets it through, POST `/api/admin/auth/signout`, then confirm the revoked cookie is bounced back to
// `/admin/signin`. This is the local tier for the sign-out route (a shell importing `$app`, so not
// bare-Vitest loadable — the same reason session.test.ts was retired at T06); the live edge proof is
// smoke-deployed-admin-web.sh + the deployed Playwright leg.

import { expect, test } from '@playwright/test';

test.describe('admin sign-out (T12)', () => {
	test('signout revokes the session — the revoked cookie is bounced to /admin/signin (AC10)', async ({ page }) => {
		// Fresh backend + a seeded admin session (the §10-F cookie, no ceremony).
		expect((await page.request.post('/api/test/reset')).ok()).toBeTruthy();
		expect((await page.request.post('/api/test/seed-session', { data: { adminId: 'admin-signout' } })).ok()).toBeTruthy();

		// Signed in: the (app) gate admits the request (no redirect away from /admin/members).
		await page.goto('/admin/members');
		await expect(page).toHaveURL(/\/admin\/members$/);

		// Sign out → the route revokes the session (KV delete) and clears the cookie.
		const res = await page.request.post('/api/admin/auth/signout');
		expect(res.ok()).toBeTruthy();

		// The now-revoked cookie no longer authorizes: the (app) gate redirects to sign-in (fail-closed).
		await page.goto('/admin/members');
		await expect(page).toHaveURL(/\/admin\/signin$/);
	});

	test('every response carries Referrer-Policy: no-referrer (AC11 / F13 — invite token leak guard)', async ({ page }) => {
		// The invite token rides in the onboard URL path; an unknown token still renders 200 (InviteExpired)
		// with the header set by hooks.server. Assert it at the integration tier (the unit proof is
		// security-headers.test.ts).
		const res = await page.request.get('/admin/onboard/referrer-policy-probe');
		expect(res.headers()['referrer-policy']).toBe('no-referrer');
	});
});
