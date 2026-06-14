import { defineConfig, devices } from '@playwright/test';

// Deployed-edge ceremony smoke (spec 009 T12) — the AC10 passkey leg a pure-bash/curl smoke can't do.
// Driven by scripts/smoke-deployed-admin-web.sh when the operator opts in (DEPLOYED_CEREMONY=1): points
// Chromium's CDP virtual authenticator at the REAL deployed admin-web routes. Unlike the default
// playwright.config.ts (tests/e2e against `vite dev`) there is NO webServer — the edge is already up —
// and the testDir is separate so the normal e2e job never picks these up. Runs live at T13.

const base = process.env.DEPLOYED_BASE;
if (base === undefined || base === '') {
	throw new Error(
		'DEPLOYED_BASE must be set to the deployed admin-web origin ' +
			'(e.g. https://boundless-admin-web.<account>.workers.dev). ' +
			'This config drives a passkey ceremony against a LIVE deployment — never `vite dev`.',
	);
}

export default defineConfig({
	testDir: 'tests/smoke',
	fullyParallel: false,
	workers: 1,
	forbidOnly: !!process.env.CI,
	reporter: 'list',
	use: { baseURL: base },
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
