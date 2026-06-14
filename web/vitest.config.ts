import { defineConfig } from 'vitest/config';

// Unit/integration tests: the framework-agnostic WebAuthn edge-verification module (spec 001 T09,
// src/**), the API contract-freeze test (spec 001 T10, tests/contract/** — parses the frozen
// api/openapi.yaml), the cross-cutting verification gates (spec 001 T16, tests/cross-platform/**
// — the AC12 catalog parity + pseudo-locale generation, the AC13 network allow-list wrapper, and the
// spec-009 T08 no-raw-console lint), and the build-artifact gates (spec 009 T07/T09, tests/build-gates/** —
// the AC5 dev-seam tree-shake proof + the AC15 wrangler-types/binding drift check; these run `vite build`
// / `wrangler types`, so they are slower and live in their own glob).
// The real-ceremony AC20 test + the AC12 zz-ZZ render test run under Playwright (tests/e2e), not here.
export default defineConfig({
  test: {
    include: [
      'src/**/*.test.ts',
      'tests/contract/**/*.test.ts',
      'tests/cross-platform/**/*.test.ts',
      'tests/build-gates/**/*.test.ts',
    ],
    environment: 'node',
  },
});
