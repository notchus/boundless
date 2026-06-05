// Admin session unit tests (spec 001 T15). Asserts the §10-F cookie policy in code (the wire bytes
// relax `Secure` on http://localhost, so the policy — not the wire — is the authority here; the
// Playwright e2e asserts HttpOnly + SameSite=Strict on the wire). Relative import (no `$lib` in Vitest).

import { describe, expect, it } from 'vitest';

import {
	ADMIN_SESSION_COOKIE,
	createSession,
	getSession,
	resetSessions,
	SESSION_COOKIE_OPTIONS,
} from './session';

describe('admin session (§10-F)', () => {
	it('cookie flags are httpOnly + Secure + SameSite=Strict, app-wide path', () => {
		expect(SESSION_COOKIE_OPTIONS.httpOnly).toBe(true);
		expect(SESSION_COOKIE_OPTIONS.secure).toBe(true);
		expect(SESSION_COOKIE_OPTIONS.sameSite).toBe('strict');
		expect(SESSION_COOKIE_OPTIONS.path).toBe('/');
	});

	it('uses a stable cookie name', () => {
		expect(ADMIN_SESSION_COOKIE).toBe('boundless_admin_session');
	});

	it('round-trips a session id to its admin', () => {
		const id = createSession('admin-123');
		expect(getSession(id)).toEqual({ adminId: 'admin-123' });
	});

	it('returns null for an undefined or unknown id', () => {
		expect(getSession(undefined)).toBeNull();
		expect(getSession('not-a-session')).toBeNull();
	});

	it('resetSessions() clears state', () => {
		const id = createSession('admin-x');
		resetSessions();
		expect(getSession(id)).toBeNull();
	});
});
