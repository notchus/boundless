import { defineConfig, devices } from '@playwright/test';

// Two e2e suites, both Chromium (the only engine with a CDP virtual authenticator):
//   • webauthn.spec.ts (T09) — verification core with real ceremony bytes; serves its own
//     http://localhost page via route fulfillment and needs NO app server.
//   • admin-onboarding.spec.ts (T15) — the real SvelteKit admin onboarding UI + endpoints + axe,
//     driven against the dev server below. The dev server (vite dev) keeps the dev-only /api/test/*
//     seed/reset seams available; a production build would 404 them.
export default defineConfig({
	testDir: 'tests/e2e',
	fullyParallel: false,
	// One worker = fully serial across files. The specs share ONE dev server with a single global
	// in-memory backend (WebAuthn invites/credentials + the spec-008 member store) and global dev-only
	// /api/test/reset|seed seams; running spec FILES on parallel workers would let one file's reset wipe
	// another's seeded session/state mid-test. Serial is correct for that shared-backend design.
	workers: 1,
	forbidOnly: !!process.env.CI,
	reporter: 'list',
	webServer: {
		command: 'pnpm dev --port 4173 --strictPort',
		url: 'http://localhost:4173/admin/signin',
		reuseExistingServer: !process.env.CI,
		stdout: 'ignore',
		stderr: 'pipe',
		timeout: 120_000,
	},
	use: { baseURL: 'http://localhost:4173' },
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
