// AC11 (local rp-config leg) — the WebAuthn Relying-Party config is fully env-driven with no hard-coded
// host, and fails closed outside dev (spec 009 T09; ADR-0017). Pure → bare-Vitest.

import { describe, expect, it } from 'vitest';

import { resolveRpConfig } from './rp-config';

const URL_LOCAL = new URL('http://localhost:4173/admin/signin');
const URL_EDGE = new URL('https://boundless-admin-web.acme.workers.dev/admin/signin');

describe('resolveRpConfig — env-driven, fail-closed', () => {
	it('uses the env values verbatim and ignores the request URL (production)', () => {
		const rp = resolveRpConfig(
			{
				WEBAUTHN_RP_ID: 'admin.boundless.example',
				WEBAUTHN_ORIGIN: 'https://admin.boundless.example',
				WEBAUTHN_RP_NAME: 'Boundless Admin',
			},
			URL_EDGE, // a different host — must be ignored when env is set
			{ dev: false },
		);
		expect(rp).toEqual({
			rpID: 'admin.boundless.example',
			origin: 'https://admin.boundless.example',
			rpName: 'Boundless Admin',
		});
	});

	it('falls back to the request URL in dev (localhost) when env is unset', () => {
		const rp = resolveRpConfig({}, URL_LOCAL, { dev: true });
		expect(rp).toEqual({ rpID: 'localhost', origin: 'http://localhost:4173', rpName: 'Boundless' });
	});

	it('fails closed OUTSIDE dev when RP_ID/ORIGIN are unset — never derives a host from the request', () => {
		expect(() => resolveRpConfig({}, URL_EDGE, { dev: false })).toThrow(
			/WEBAUTHN_RP_ID \/ WEBAUTHN_ORIGIN are not set/,
		);
	});

	it('fails closed outside dev on a PARTIAL config (RP_ID set, ORIGIN missing)', () => {
		expect(() =>
			resolveRpConfig({ WEBAUTHN_RP_ID: 'admin.boundless.example' }, URL_EDGE, { dev: false }),
		).toThrow(/not set/);
	});

	it('defaults rpName to "Boundless" (the OS passkey-UI label, not a catalog string) when unset', () => {
		const rp = resolveRpConfig(
			{ WEBAUTHN_RP_ID: 'admin.boundless.example', WEBAUTHN_ORIGIN: 'https://admin.boundless.example' },
			URL_EDGE,
			{ dev: false },
		);
		expect(rp.rpName).toBe('Boundless');
	});
});
