import { defineConfig, devices } from '@playwright/test';

// AC20 real-ceremony WebAuthn test (spec 001 T09): drives Chromium's CDP virtual
// authenticator to produce genuine attestation/assertion bytes, fed through the real
// @simplewebauthn/server verifier. Chromium-only (the only engine with a virtual
// authenticator) — expressed as a single project so no per-test skip is needed.
// No webServer: the spec serves its own secure-context page from http://localhost via
// page.route fulfillment (localhost is a WebAuthn-eligible secure context).
export default defineConfig({
  testDir: 'tests/e2e',
  fullyParallel: false,
  forbidOnly: true,
  reporter: 'list',
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
