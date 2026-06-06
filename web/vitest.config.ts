import { defineConfig } from 'vitest/config';

// Unit/integration tests: the framework-agnostic WebAuthn edge-verification module (spec 001 T09,
// src/**), the API contract-freeze test (spec 001 T10, tests/contract/** — parses the frozen
// api/openapi.yaml), and the cross-cutting verification gates (spec 001 T16, tests/cross-platform/**
// — the AC12 catalog parity + pseudo-locale generation, and the AC13 network allow-list wrapper).
// The real-ceremony AC20 test + the AC12 zz-ZZ render test run under Playwright (tests/e2e), not here.
export default defineConfig({
  test: {
    include: ['src/**/*.test.ts', 'tests/contract/**/*.test.ts', 'tests/cross-platform/**/*.test.ts'],
    environment: 'node',
  },
});
