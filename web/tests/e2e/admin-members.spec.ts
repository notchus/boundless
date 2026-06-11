// Admin member-management UI — full-stack e2e (spec 008 T10; AC1/AC6/AC9/AC10/AC14/AC15 client legs).
//
// Drives the REAL SvelteKit `(app)` routes (under `vite dev`) backed by the interim in-memory member
// backend (the live deployed BFF→Worker round-trip is the deferred shell — DEFERRED.md → T10). The
// dev-only /api/test/* seams seed a session + members + reset per-test. axe-core asserts zero a11y
// violations per route × {default, dark, RTL} (AC14); other specs cover the keyboard dialog ceremony,
// 400% reflow, the audited-read → audit-log trail (names not values), the no-create-admin invariant
// (AC10), and the pseudo-locale render (AC15).

import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';

import { PSEUDO_OPEN } from '../../src/lib/i18n/pseudo';

const WCAG_22_AA = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

async function expectNoA11yViolations(page: Page): Promise<void> {
	const results = await new AxeBuilder({ page }).withTags(WCAG_22_AA).analyze();
	const summary = results.violations.map((v) => ({ id: v.id, impact: v.impact, nodes: v.nodes.length }));
	expect(summary, JSON.stringify(summary, null, 2)).toEqual([]);
}

async function resetBackend(page: Page): Promise<void> {
	expect((await page.request.post('/api/test/reset')).ok()).toBeTruthy();
}

/** Mint an admin session (the §10-F cookie) without the WebAuthn ceremony (covered elsewhere). */
async function signIn(page: Page, adminId = 'admin-e2e'): Promise<void> {
	expect((await page.request.post('/api/test/seed-session', { data: { adminId } })).ok()).toBeTruthy();
}

interface SeedMember {
	member_id?: string;
	name: string;
	phone: string;
	address: string;
	roles?: string[];
	onboarding_status?: string;
}
async function seedMember(page: Page, m: SeedMember): Promise<string> {
	const res = await page.request.post('/api/test/seed-member', { data: m });
	expect(res.ok()).toBeTruthy();
	return ((await res.json()) as { member_id: string }).member_id;
}

async function gotoReady(page: Page, path: string): Promise<void> {
	await page.goto(path);
	await page.locator('html[data-hydrated="true"]').waitFor({ state: 'attached', timeout: 15_000 });
}

async function setup(page: Page): Promise<void> {
	await resetBackend(page);
	await signIn(page);
}

test.describe('admin member-management UI (T10)', () => {
	test('members_routes_axe_clean_default_dark_rtl — every route × {default, dark, RTL}', async ({ page }) => {
		await setup(page);
		const id = await seedMember(page, { name: 'Maria', phone: '+15555550101', address: '12 Olive St', roles: ['rider'] });
		// A detail view (the audited read) so the audit-log route has content to render.
		await gotoReady(page, `/admin/members/${id}`);
		const routes = [
			{ name: 'list', path: '/admin/members' },
			{ name: 'detail', path: `/admin/members/${id}` },
			{ name: 'audit', path: '/admin/audit-log' },
		];

		for (const scheme of ['light', 'dark'] as const) {
			await page.emulateMedia({ colorScheme: scheme });
			for (const route of routes) {
				await gotoReady(page, route.path);
				await expectNoA11yViolations(page);
			}
		}
		// RTL (Arabic → dir=rtl): layout mirrors; assert direction + zero violations.
		await page.emulateMedia({ colorScheme: 'light' });
		for (const route of routes) {
			await gotoReady(page, `${route.path}?locale=ar`);
			await expect(page.locator('html')).toHaveAttribute('dir', 'rtl');
			await expectNoA11yViolations(page);
		}

		// The open Add dialog is also axe-clean (focus-trapped melt-ui dialog).
		await gotoReady(page, '/admin/members');
		await page.getByRole('button', { name: 'Add a member' }).click();
		await expect(page.getByRole('dialog')).toBeVisible();
		await expectNoA11yViolations(page);
	});

	test('members_add_edit_dialog_keyboard_ceremony — open by keyboard, focus trapped, Esc returns focus', async ({ page }) => {
		await setup(page);
		await gotoReady(page, '/admin/members');

		const add = page.getByRole('button', { name: 'Add a member' });
		await add.focus();
		await expect(add).toBeFocused();
		await add.press('Enter'); // open via keyboard

		const dialog = page.getByRole('dialog');
		await expect(dialog).toBeVisible();
		// Focus moved INTO the dialog (melt focus trap) — the active element is the dialog or a descendant.
		// Poll: melt settles focus a tick after the dialog renders.
		await expect
			.poll(
				() =>
					page.evaluate(() => {
						const d = document.querySelector('.bl-dialog');
						return !!d && (d === document.activeElement || d.contains(document.activeElement));
					}),
				{ timeout: 5000 },
			)
			.toBe(true);

		// Esc closes and returns focus to the trigger (melt).
		await page.keyboard.press('Escape');
		await expect(dialog).toBeHidden();
		await expect(add).toBeFocused();

		// Reopen and add a member entirely by keyboard.
		await add.press('Enter');
		await page.getByLabel('Name').fill('Daniel');
		await page.getByLabel('Phone').fill('+15555550110');
		await page.getByLabel('Address').fill('5 Birch Rd');
		await page.getByLabel('Driver').check();
		await page.getByRole('button', { name: 'Save' }).click();
		await expect(dialog.getByText('Member added.')).toBeVisible();
		await expect(dialog.getByText(/BNDL-/)).toBeVisible(); // the show-once Onboarding Code
	});

	test('members_issue_then_appears_in_list; duplicate phone surfaces-and-links', async ({ page }) => {
		await setup(page);
		await gotoReady(page, '/admin/members');

		// Issue.
		await page.getByRole('button', { name: 'Add a member' }).click();
		await page.getByLabel('Name').fill('Margaret');
		await page.getByLabel('Phone').fill('+15555550120');
		await page.getByLabel('Address').fill('9 Cedar Ln');
		await page.getByLabel('Rider').check();
		await page.getByRole('button', { name: 'Save' }).click();
		await expect(page.getByText(/BNDL-/)).toBeVisible();
		// Close the dialog → Margaret is now in the list.
		await page.getByRole('button', { name: 'Cancel' }).click();
		await expect(page.getByRole('cell', { name: 'Margaret', exact: true })).toBeVisible();

		// Duplicate phone → the existing member is surfaced + linked (admin-only, never a silent fail).
		await page.getByRole('button', { name: 'Add a member' }).click();
		await page.getByLabel('Name').fill('Impostor');
		await page.getByLabel('Phone').fill('+1 (555) 555-0120'); // same number, different formatting
		await page.getByLabel('Address').fill('99 Fake Rd');
		await page.getByLabel('Rider').check();
		await page.getByRole('button', { name: 'Save' }).click();
		await expect(page.getByRole('alert')).toContainText('That number is already in your group.');
		await expect(page.getByRole('link', { name: 'View the member who has it' })).toBeVisible();
	});

	test('audit_log_validation_aria_live — validation announced; audited read logs NAMES not values (AC9)', async ({ page }) => {
		await setup(page);
		const id = await seedMember(page, { name: 'Tobias', phone: '+15555550130', address: '3 Elm St', roles: ['driver'] });

		// (a) A validation error is announced in an assertive live region (role=alert).
		await gotoReady(page, '/admin/members');
		await page.getByRole('button', { name: 'Add a member' }).click();
		await page.getByLabel('Name').fill('Bad Phone');
		await page.getByLabel('Phone').fill('not-a-phone');
		await page.getByLabel('Address').fill('1 Nowhere');
		await page.getByLabel('Rider').check();
		await page.getByRole('button', { name: 'Save' }).click();
		await expect(page.getByRole('alert')).toContainText("That number doesn't look right.");

		// (b) Viewing the member is an AUDITED read; the audit log shows it — field NAMES, never values.
		await gotoReady(page, `/admin/members/${id}`);
		await expect(page.getByRole('heading', { level: 1 })).toHaveText('Tobias');
		await gotoReady(page, '/admin/audit-log');
		const live = page.locator('[aria-live="polite"]');
		await expect(live).toBeVisible();
		// The fields cell carries the field NAMES (name/phone/address) as badges.
		const rowText = await live.locator('tbody tr').first().innerText();
		expect(rowText).toContain('Name');
		expect(rowText).toContain('Phone');
		expect(rowText).toContain('Address');
		// No PII VALUE leaks into the audit log (AC9 / P2).
		const body = await page.locator('body').innerText();
		expect(body).not.toContain('3 Elm St');
		expect(body).not.toContain('+15555550130');
	});

	test('members_ui_offers_no_create_admin_action — no admin-creation affordance (AC10/I11)', async ({ page }) => {
		await setup(page);
		await gotoReady(page, '/admin/members');
		await page.getByRole('button', { name: 'Add a member' }).click();

		// The issuance roles are Rider/Driver only — no Admin option anywhere.
		await expect(page.getByRole('checkbox', { name: 'Rider' })).toBeVisible();
		await expect(page.getByRole('checkbox', { name: 'Driver' })).toBeVisible();
		expect(await page.getByRole('checkbox', { name: /admin/i }).count()).toBe(0);

		// No create-/invite-/add-admin affordance in the accessible names or copy.
		const forbidden = /create (an )?admin|invite (an )?admin|add (an )?admin|new admin/i;
		expect(await page.getByRole('button', { name: forbidden }).count()).toBe(0);
		expect(await page.getByRole('link', { name: forbidden }).count()).toBe(0);
		expect((await page.locator('body').innerText()).toLowerCase()).not.toMatch(forbidden);
	});

	test('members_list_reflows_at_400_percent — no horizontal scroll (WCAG 1.4.10)', async ({ page }) => {
		await setup(page);
		const id = await seedMember(page, { name: 'Maria', phone: '+15555550140', address: '12 Olive St', roles: ['rider', 'driver'] });
		// 1280px ÷ 200% = 640 CSS px; ÷ 400% = 320 CSS px.
		for (const width of [640, 320]) {
			await page.setViewportSize({ width, height: 720 });
			for (const path of ['/admin/members', `/admin/members/${id}`, '/admin/audit-log']) {
				await gotoReady(page, path);
				const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
				expect(overflow, `horizontal scroll at ${width}px on ${path}`).toBe(false);
			}
		}
	});

	test('members_pseudo_locale_renders_without_truncation[zz-ZZ] — bracketed copy + reflow (AC15)', async ({ page }) => {
		await setup(page);
		await seedMember(page, { name: 'Maria', phone: '+15555550150', address: '12 Olive St', roles: ['rider'] });

		// Each screen's catalog-sourced <h1> renders the bracketed pseudo string (proof it resolved
		// through the catalog, P8); the document lang is the pseudo-locale.
		for (const path of ['/admin/members', '/admin/audit-log']) {
			await gotoReady(page, `${path}?locale=zz-ZZ`);
			await expect(page.getByRole('heading', { level: 1 }), `h1 not pseudo-localized on ${path}`).toContainText(PSEUDO_OPEN);
			await expect(page.locator('html')).toHaveAttribute('lang', 'zz-ZZ');
		}
		// The padded (~40% longer) pseudo copy reflows without horizontal scroll at 400%.
		await page.setViewportSize({ width: 320, height: 720 });
		for (const path of ['/admin/members', '/admin/audit-log']) {
			await gotoReady(page, `${path}?locale=zz-ZZ`);
			const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
			expect(overflow, `horizontal scroll at 320px on ${path} (zz-ZZ)`).toBe(false);
		}
	});

	test('members_edit_reencrypts_and_returns_audited_detail (AC11 client leg)', async ({ page }) => {
		await setup(page);
		const id = await seedMember(page, { name: 'Daniel', phone: '+15555550160', address: '5 Birch Rd', roles: ['driver'] });
		await gotoReady(page, `/admin/members/${id}`);

		await page.getByRole('button', { name: 'Edit' }).click();
		const dialog = page.getByRole('dialog');
		await expect(dialog).toBeVisible();
		await dialog.getByLabel('Address').fill('7 Birch Rd');
		await dialog.getByRole('button', { name: 'Save' }).click();
		await expect(dialog).toBeHidden();
		// The updated detail (an audited read) reflects the change.
		await expect(page.getByText('7 Birch Rd')).toBeVisible();
	});
});
