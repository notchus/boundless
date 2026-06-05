import { defineConfig } from 'vitest/config';

// Unit/integration tests: the framework-agnostic WebAuthn edge-verification module (spec 001 T09,
// src/**) and the API contract-freeze test (spec 001 T10, tests/contract/** — parses the frozen
// api/openapi.yaml). The real-ceremony AC20 test runs under Playwright (tests/e2e), not here.
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts', 'tests/contract/**/*.test.ts'],
    environment: 'node',
  },
});
