// AC12 (P8) — pseudo-locale render, the *rendered* proof. Spec 001 T16.
//
// The cross-platform Vitest gate (tests/cross-platform/catalog-parity.test.ts) proves every key
// has a pseudo expansion; this drives the real SvelteKit admin onboarding screens in `?locale=zz-ZZ`
// and asserts (i) the visible copy is the bracketed pseudo string — proof every rendered string
// resolved through the catalog (a hardcoded literal would show un-bracketed), and (ii) the padded
// (~40% longer) pseudo text reflows without horizontal scroll at the WCAG 200%/400% widths (the
// "renders without truncation" leg). Web is the one onboarding surface cheap to render here; the
// native zz-ZZ pixel-truncation snapshots are each platform's -shell (DEFERRED.md).

import { expect, test, type Page } from '@playwright/test';

import { PSEUDO_OPEN } from '../../src/lib/i18n/pseudo';

// — helpers (mirror tests/e2e/admin-onboarding.spec.ts; kept inline to avoid touching that file) —

async function gotoReady(page: Page, path: string): Promise<void> {
	await page.goto(path);
	await page.locator('html[data-hydrated="true"]').waitFor({ state: 'attached', timeout: 15_000 });
}

async function resetBackend(page: Page): Promise<void> {
	const res = await page.request.post('/api/test/reset');
	expect(res.ok()).toBeTruthy();
}

async function seedLiveInvite(page: Page, token: string): Promise<void> {
	const expiresAt = Math.floor(Date.now() / 1000) + 3600;
	const res = await page.request.post('/api/test/seed-invite', {
		data: { token, adminId: 'admin-zz', groupId: 'grp-zz', expiresAt },
	});
	expect(res.ok()).toBeTruthy();
}

const PL = (path: string): string => `${path}${path.includes('?') ? '&' : '?'}locale=zz-ZZ`;

test.describe('pseudo-locale render (T16, AC12)', () => {
	test('pseudo_locale_renders_all_onboarding_screens[zz-ZZ] — bracketed copy, no untranslated literal', async ({
		page,
	}) => {
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-zz');

		// Each onboarding screen's <h1> must render the bracketed pseudo string — i.e. it resolved
		// through the catalog (P8). signin · live registration · InviteExpired terminal.
		for (const path of ['/admin/signin', '/admin/onboard/tok-zz', '/admin/onboard/never-issued']) {
			await gotoReady(page, PL(path));
			const h1 = page.getByRole('heading', { level: 1 });
			await expect(h1, `h1 not pseudo-localized on ${path}`).toContainText(PSEUDO_OPEN);
			// No raw ASCII-letter run of a real English word leaks outside the brackets: the lang is
			// the pseudo-locale and the document direction is still ltr.
			await expect(page.locator('html')).toHaveAttribute('lang', 'zz-ZZ');
		}
	});

	test('reflow in zz-ZZ at 200% and 400% — no horizontal scroll (WCAG 1.4.10 with padded copy)', async ({
		page,
	}) => {
		await resetBackend(page);
		await seedLiveInvite(page, 'tok-zz-reflow');

		for (const width of [640, 320]) {
			await page.setViewportSize({ width, height: 720 });
			for (const path of ['/admin/signin', '/admin/onboard/tok-zz-reflow', '/admin/onboard/never']) {
				await gotoReady(page, PL(path));
				const overflow = await page.evaluate(
					() => document.documentElement.scrollWidth > window.innerWidth + 1,
				);
				expect(overflow, `horizontal scroll at ${width}px on ${path} (zz-ZZ)`).toBe(false);
			}
		}
	});
});
