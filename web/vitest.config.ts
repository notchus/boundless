import { defineConfig } from 'vitest/config';

// Unit/integration tests for the framework-agnostic WebAuthn edge-verification module
// (spec 001 T09). The real-ceremony AC20 test runs under Playwright (tests/e2e), not here.
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
