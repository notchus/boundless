import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { WEBAUTHN_ERROR_CODES } from './errors';

// P12: every error this module raises must be a registered code in docs/error-codes.md.
// Mirrors the Rust `auth_verdict_error_codes_match_registry` test. The path is anchored on
// this file (5 levels up: webauthn → server → lib → src → web → repo root).
describe('webauthn error codes', () => {
  it('webauthn_error_codes_match_registry: each code is documented in docs/error-codes.md', () => {
    const registryPath = fileURLToPath(new URL('../../../../../docs/error-codes.md', import.meta.url));
    const registry = readFileSync(registryPath, 'utf8');
    for (const code of WEBAUTHN_ERROR_CODES) {
      expect(registry.includes('`' + code + '`'), `${code} must be backtick-quoted in docs/error-codes.md`).toBe(
        true,
      );
    }
  });
});
