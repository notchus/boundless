// Admin onboarding UI — full-stack e2e (spec 001 T15; AC2/AC11b/AC1(b)).
//
// Drives the REAL SvelteKit routes (under `vite dev`) + the real admin auth endpoints + the T09
// verification core (wired to the interim in-memory backend), with Chromium's CDP virtual
// authenticator producing genuine WebAuthn bytes through the actual page UI (@simplewebauthn/browser).
// axe-core asserts zero a11y violations per route × {default, dark, RTL}. The dev-only /api/test/*
// seams seed/reset the in-memory invite store.

import AxeBuilder from '@axe-core/playwright';
import { expect, test, type CDPSession, type Page } from '@playwright/test';

const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

async function expectNoA11yViolations(page: Page): Promise<void> {
	const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).analyze();
	const summary = results.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length }));
	expect(summary, JSON.stringify(summary, null, 2)).toEqual([]);
}

async function attachAuthenticator(
	page: Page,
	opts: { userVerified?: boolean } = {},
): Promise<CDPSession> {
	const client = await page.context().newCDPSession(page);
	await client.send('WebAuthn.enable');
	await client.send('WebAuthn.addVirtualAuthenticator', {
		options: {
			protocol: 'ctap2',
			transport: 'internal',
			hasResidentKey: true,
			hasUserVerification: true,
			isUserVerified: opts.userVerified ?? true,
			automaticPresenceSimulation: true,
		},
	});
	return client;
}

async function resetBackend(page: Page): Promise<void> {
	const res = await page.request.post('/api/test/reset');
	expect(res.ok()).toBeTruthy();
}

async function seedLiveInvite(page: Page, token: string, adminId = 'admin-e2e'): Promise<void> {
	const expiresAt = Math.floor(Date.now() / 1000) + 3600;
	const res = await page.request.post('/api/test/seed-invite', {
		data: { token, adminId, groupId: 'grp-e2e', expiresAt },
	});
	expect(res.ok()).toBeTruthy();
}

/** Navigate and wait for client hydration (html[data-hydrated], set in +layout.svelte) so a
 * keyboard/click can't land before Svelte has wired its handlers. */
async function gotoReady(page: Page, path: string): Promise<void> {
	await page.goto(path);
	await page.locator('html[data-hydrated="true"]').waitFor({ state: 'attached', timeout: 15_000 });
}

const ONBOARD_LIVE = (token: string) => `/admin/onboard/${token}`;
const ONBOARD_EXPIRED = '/admin/onboard/this-token-was-never-issued';

test.describe('admin onboarding UI (T15)', () => {
	test('ac2_no_password_field — the admin auth surface has no password input', async ({ page }) => {
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-nopw');

		for (const path of ['/admin/signin', ONBOARD_LIVE('tok-nopw'), ONBOARD_EXPIRED]) {
			await gotoReady(page, path);
			expect(await page.locator('input[type="password"]').count()).toBe(0);
			// No password-y input of any kind (autocomplete=current-password / name~=pass).
			expect(await page.locator('input[autocomplete*="password"]').count()).toBe(0);
			expect(await page.locator('input[name*="pass" i]').count()).toBe(0);
		}
	});

	test('web_onboarding_no_signup_route — no signup / create-account / request-access affordance (AC1(b))', async ({
		page,
	}) => {
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-nosignup');

		const forbidden = /sign\s*up|create (an )?account|request access|register an account/i;
		for (const path of ['/admin/signin', ONBOARD_LIVE('tok-nosignup'), ONBOARD_EXPIRED]) {
			await gotoReady(page, path);
			const body = (await page.locator('body').innerText()).toLowerCase();
			expect(body).not.toMatch(forbidden);
			// No link/button whose accessible name offers signup.
			expect(await page.getByRole('link', { name: forbidden }).count()).toBe(0);
			expect(await page.getByRole('button', { name: forbidden }).count()).toBe(0);
		}
	});

	test('ac11b_axe_zero_violations — every route × {default, dark, RTL}', async ({ page }) => {
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-axe');
		const routes = [
			{ name: 'signin', path: '/admin/signin' },
			{ name: 'onboard-live', path: ONBOARD_LIVE('tok-axe') },
			{ name: 'invite-expired', path: ONBOARD_EXPIRED },
		];

		for (const scheme of ['light', 'dark'] as const) {
			await page.emulateMedia({ colorScheme: scheme });
			for (const route of routes) {
				await gotoReady(page, route.path);
				await expectNoA11yViolations(page);
			}
		}
		// RTL (Arabic locale → dir=rtl): layout mirrors; assert direction + zero violations.
		await page.emulateMedia({ colorScheme: 'light' });
		for (const route of routes) {
			await gotoReady(page, `${route.path}?locale=ar`);
			await expect(page.locator('html')).toHaveAttribute('dir', 'rtl');
			await expectNoA11yViolations(page);
		}
	});

	test('reflow at 200% and 400% — no horizontal scroll (WCAG 1.4.10)', async ({ page }) => {
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-reflow');
		// 1280px ÷ 200% = 640 CSS px; ÷ 400% = 320 CSS px (the WCAG reflow target).
		for (const width of [640, 320]) {
			await page.setViewportSize({ width, height: 720 });
			for (const path of ['/admin/signin', ONBOARD_LIVE('tok-reflow')]) {
				await gotoReady(page, path);
				const overflow = await page.evaluate(
					() => document.documentElement.scrollWidth > window.innerWidth + 1,
				);
				expect(overflow, `horizontal scroll at ${width}px on ${path}`).toBe(false);
			}
		}
	});

	test('register a passkey, then sign in (AC2/AC16/AC20) — real ceremony + session cookie', async ({
		page,
	}) => {
		await attachAuthenticator(page, { userVerified: true });
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-happy', 'admin-happy');

		// — Register —
		await gotoReady(page, ONBOARD_LIVE('tok-happy'));
		await expect(page.getByRole('heading', { level: 1 })).toHaveText('Set up your security key or passkey.');
		await page.getByRole('button', { name: 'Set up your key' }).click();
		await expect(page.getByRole('status')).toContainText('Your key is set up.');
		await expectNoA11yViolations(page); // the registered-success state

		// Single-use (AC16): the invite is now consumed → re-resolving it is 410 InviteExpired
		// (the frozen contract status code; the opaque token leaks no existence on a 410).
		const recheck = await page.request.get('/api/admin/auth/invite/tok-happy');
		expect(recheck.status()).toBe(410);
		expect((await recheck.json()).routes_to).toBe('InviteExpired');

		// — Sign in (assertion only, AC2) → session cookie (§10-F) → admin home —
		await page.getByRole('link', { name: 'Go to sign in' }).click();
		await expect(page).toHaveURL(/\/admin\/signin$/);
		await page.getByRole('button', { name: 'Sign in with your key' }).click();
		await expect(page).toHaveURL(/\/admin$/);
		await expect(page.getByRole('heading', { level: 1 })).toHaveText("You're signed in.");
		await expectNoA11yViolations(page); // the signed-in state

		const session = (await page.context().cookies()).find((c) => c.name === 'boundless_admin_session');
		expect(session, 'admin session cookie set').toBeDefined();
		expect(session?.httpOnly).toBe(true);
		expect(session?.sameSite).toBe('Strict');
	});

	test('ac11b_webauthn_keyboard_only — the registration ceremony is keyboard-operable', async ({
		page,
	}) => {
		await attachAuthenticator(page, { userVerified: true });
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-kbd', 'admin-kbd');

		await gotoReady(page, ONBOARD_LIVE('tok-kbd'));
		// Tab to the single primary control and confirm focus lands on it (keyboard-reachable, with a
		// visible focus ring via the .bl-button focus-visible styles).
		await page.keyboard.press('Tab');
		const button = page.getByRole('button', { name: 'Set up your key' });
		await expect(button).toBeFocused();
		// Activate by keyboard on the focused control (Space — the canonical button activation) — the
		// ceremony runs and succeeds without a mouse.
		await button.press('Space');
		await expect(page.getByRole('status')).toContainText('Your key is set up.');
	});

	test('aria-live — InviteExpired and a sign-in binding error are announced (AC11b)', async ({
		page,
	}) => {
		// InviteExpired terminal → role=alert with the calm message.
		await gotoReady(page, ONBOARD_EXPIRED);
		await expect(page.getByRole('alert')).toContainText('This invitation has expired.');

		// Binding error: sign in with an authenticator that holds no credential for this RP → the
		// assertion fails → the page announces a retry message in an assertive live region.
		await attachAuthenticator(page, { userVerified: true });
		await resetBackend(page);
		await gotoReady(page, '/admin/signin');
		await page.getByRole('button', { name: 'Sign in with your key' }).click();
		await expect(page.getByRole('alert')).toContainText("That didn't work.");
	});
});
