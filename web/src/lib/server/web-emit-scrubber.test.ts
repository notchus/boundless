// AC8 (web leg) — the scrubbed server-log sink (spec 009 T08, P2/I10). Proves `scrub()` redacts secrets /
// the URL-embedded invite token before they reach the sink, that the calm operator strings survive
// secret-free, and that `emit`/`logServerError` only ever write scrubbed output. Pure (no SvelteKit
// virtuals) → bare-Vitest.

import { afterEach, describe, expect, it, vi } from 'vitest';

import { emit, logServerError, scrub } from './log';

// A realistic opaque single-use invite token: 64 hex chars (256-bit), mixed letters+digits.
const INVITE_TOKEN = 'a1b2c3d4'.repeat(8);

describe('scrub() — redacts secrets and high-entropy tokens', () => {
	it('redacts an authorization Bearer shared secret', () => {
		const out = scrub('BFF→Worker authorization: Bearer sk-live-supersecretvalue-9876543210');
		expect(out).not.toContain('supersecretvalue');
		expect(out).toContain('Bearer [redacted]');
	});

	it('redacts a Postgres connection string (the DB password rides in it)', () => {
		const out = scrub('connect failed: postgres://boundless_app:hunter2@ep-neon.example/neondb?sslmode=require');
		expect(out).not.toContain('hunter2');
		expect(out).not.toContain('ep-neon.example');
		expect(out).toContain('postgres://[redacted]');
	});

	it('redacts a URL-embedded opaque invite token (R13), keeping the path prefix', () => {
		const out = scrub(`GET /api/admin/auth/invite/${INVITE_TOKEN} 500`);
		expect(out).not.toContain(INVITE_TOKEN);
		expect(out).toContain('/api/admin/auth/invite/[redacted]');
	});

	it('redacts a standalone high-entropy token/secret blob', () => {
		const token = 'AbC123dEf456GhI789jKl012MnO345pQr678'; // 36 chars, mixed
		expect(scrub(`token=${token}`)).not.toContain(token);
		expect(scrub(`token=${token}`)).toContain('[redacted]');
	});

	it('spares an ordinary long word with no digit (the letter+digit guard)', () => {
		const word = 'thisisanordinarylongidentifierwithnodigits'; // 42 chars, all letters
		expect(scrub(`route ${word}`)).toBe(`route ${word}`);
	});

	it('leaves a calm operator fail-closed string intact and secret-free', () => {
		const operator =
			'Admin session store unavailable: the ADMIN_SESSIONS KV namespace is not bound. Refusing the ' +
			'in-memory fallback outside dev (spec 009 D5/AC2).';
		const out = scrub(operator);
		expect(out).toBe(operator); // no redaction — it carries no secret/token
		expect(out).not.toContain('[redacted]');
	});

	it('is idempotent', () => {
		const once = scrub(`Bearer ${INVITE_TOKEN}`);
		expect(scrub(once)).toBe(once);
	});
});

describe('emit() — the sole sanctioned console sink, always scrubbed', () => {
	afterEach(() => vi.restoreAllMocks());

	it('writes a scrubbed line (a secret in a field never reaches console)', () => {
		const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
		emit('error', 'BFF call failed', { authorization: 'Bearer leaked-secret-1234567890' });
		expect(spy).toHaveBeenCalledTimes(1);
		const line = String(spy.mock.calls[0]?.[0]);
		expect(line).not.toContain('leaked-secret-1234567890');
		expect(line).toContain('[redacted]');
		expect(line).toContain('[error]');
	});

	it('routes each level to its console method (info uses console.info, not the bare log method)', () => {
		const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
		const info = vi.spyOn(console, 'info').mockImplementation(() => {});
		const bareLog = vi.spyOn(console, 'log').mockImplementation(() => {});
		emit('warn', 'a warning');
		emit('info', 'an info');
		expect(warn).toHaveBeenCalledTimes(1);
		expect(info).toHaveBeenCalledTimes(1);
		expect(bareLog).not.toHaveBeenCalled(); // the sink never uses the bare log method (hook-forbidden)
	});
});

describe('logServerError() — routes uncaught throws through the sink', () => {
	afterEach(() => vi.restoreAllMocks());

	it('logs the route PATTERN and scrubs the error message; never the live token', () => {
		const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
		logServerError({
			error: new Error(`boom while resolving Bearer leaked-from-error-9999999999`),
			routeId: '/admin/onboard/[token]',
			status: 500,
		});
		expect(spy).toHaveBeenCalledTimes(1);
		const line = String(spy.mock.calls[0]?.[0]);
		expect(line).toContain('unhandled server error');
		expect(line).toContain('/admin/onboard/[token]'); // the safe pattern, not url.pathname
		expect(line).not.toContain('leaked-from-error-9999999999'); // scrubbed (Bearer rule)
	});
});
