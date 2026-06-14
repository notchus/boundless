// Deployed-edge passkey ceremony — the live AC10 proof (spec 009 T12). Runs ONLY against a deployed URL
// (playwright.deployed.config.ts), driven by scripts/smoke-deployed-admin-web.sh with DEPLOYED_CEREMONY=1.
//
// Chromium's CDP virtual authenticator produces genuine WebAuthn bytes through the REAL deployed routes
// (the same pattern as tests/e2e/admin-onboarding.spec.ts, but baseURL = the live edge and the invite
// comes from the operator seed — scripts/seed-admin-invite.sh — not the dev /api/test seam, which 404s in
// prod). The flow: register a passkey on a seeded invite → sign in → the live roster renders → issue one
// member → sign out → the revoked cookie is bounced to /admin/signin.
//
// The issued member is left in the real roster (there is no member-delete surface yet); use a unique
// timestamped phone/name so re-runs don't collide on the duplicate-phone path.

import { expect, test, type CDPSession, type Page } from '@playwright/test';

const INVITE_TOKEN = process.env.DEPLOYED_INVITE_TOKEN;

/** A discoverable, user-verified internal authenticator (the deployed RP uses usernameless assertion). */
async function attachAuthenticator(page: Page): Promise<CDPSession> {
	const client = await page.context().newCDPSession(page);
	await client.send('WebAuthn.enable');
	await client.send('WebAuthn.addVirtualAuthenticator', {
		options: {
			protocol: 'ctap2',
			transport: 'internal',
			hasResidentKey: true,
			hasUserVerification: true,
			isUserVerified: true,
			automaticPresenceSimulation: true,
		},
	});
	return client;
}

/** Navigate + wait for client hydration (html[data-hydrated], set in +layout.svelte). */
async function gotoReady(page: Page, path: string): Promise<void> {
	await page.goto(path);
	await page.locator('html[data-hydrated="true"]').waitFor({ state: 'attached', timeout: 20_000 });
}

test.describe('deployed admin ceremony (T12 / AC10)', () => {
	test('register → sign in → list → issue → sign out, all against the live edge', async ({ page }) => {
		expect(INVITE_TOKEN, 'set DEPLOYED_INVITE_TOKEN to a fresh seed-admin-invite.sh token').toBeTruthy();
		const token = INVITE_TOKEN as string;
		await attachAuthenticator(page);

		// — Register a passkey on the seeded invite (consumes it, single-use) —
		await gotoReady(page, `/admin/onboard/${encodeURIComponent(token)}`);
		await expect(page.getByRole('heading', { level: 1 })).toHaveText('Set up your security key or passkey.');
		await page.getByRole('button', { name: 'Set up your key' }).click();
		await expect(page.getByRole('status')).toContainText('Your key is set up.');

		// — Sign in (assertion only) → the §10-F session cookie → admin home —
		await page.getByRole('link', { name: 'Go to sign in' }).click();
		await expect(page).toHaveURL(/\/admin\/signin$/);
		await page.getByRole('button', { name: 'Sign in with your key' }).click();
		await expect(page).toHaveURL(/\/admin$/);
		await expect(page.getByRole('heading', { level: 1 })).toHaveText("You're signed in.");

		const session = (await page.context().cookies()).find((c) => c.name === 'boundless_admin_session');
		expect(session, 'admin session cookie set').toBeDefined();
		expect(session?.httpOnly).toBe(true);
		expect(session?.sameSite).toBe('Strict');

		// — The live roster renders (AC10: real data from the deployed Worker + Neon) —
		await gotoReady(page, '/admin/members');
		await expect(page).toHaveURL(/\/admin\/members$/);
		await expect(page.getByRole('button', { name: 'Add a member' })).toBeVisible();

		// — Issue one member → it appears in the list (unique phone/name so re-runs don't collide) —
		const stamp = String(Date.now()).slice(-7);
		const name = `Smoke ${stamp}`;
		await page.getByRole('button', { name: 'Add a member' }).click();
		await page.getByLabel('Name').fill(name);
		await page.getByLabel('Phone').fill(`+1555${stamp}`);
		await page.getByLabel('Address').fill('1 Smoke Test Way');
		await page.getByLabel('Rider').check();
		await page.getByRole('button', { name: 'Save' }).click();
		// Format-agnostic success assertion: the LIVE Worker mints a 64-char hex onboarding code
		// (core/server/src/secrets.rs), NOT the in-memory dev fake's `BNDL-` prefix, so assert the
		// "Member added." status (admin.member.issued), never the code's character format.
		await expect(page.getByText('Member added.')).toBeVisible();
		await page.getByRole('button', { name: 'Cancel' }).click();
		await expect(page.getByRole('cell', { name, exact: true })).toBeVisible();

		// — Sign out → the revoked cookie is bounced to /admin/signin (AC10 sign-out leg) —
		expect((await page.request.post('/api/admin/auth/signout')).ok()).toBeTruthy();
		await page.goto('/admin/members');
		await expect(page).toHaveURL(/\/admin\/signin$/);
	});
});
