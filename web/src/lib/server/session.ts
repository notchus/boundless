// Post-assertion admin session (spec 001 T15; plan §10-F). After a verified WebAuthn sign-in
// assertion the route mints a session id and sets it as an **httpOnly + Secure + SameSite=Strict**
// cookie — the web leg of security R5's at-rest credential storage (never localStorage —
// forbidden-patterns). The session DATA lives in an in-memory map here for the buildable slice; the
// **persistent** server-side session store (Cloudflare KV / Postgres) is the T15-shell (DEFERRED).
// Admin sessions are separate from and shorter-lived than member sessions (ADR-0016).

import { redirect, type Cookies } from '@sveltejs/kit';

export const ADMIN_SESSION_COOKIE = 'boundless_admin_session';

/** §10-F cookie flags. `secure: true` is the contract; SvelteKit transparently relaxes it on
 * http://localhost for local dev/tests, so the wire bytes there omit `Secure` while the policy
 * (asserted in session.test.ts) stays `secure: true`. */
export const SESSION_COOKIE_OPTIONS: Parameters<Cookies['set']>[2] = {
	httpOnly: true,
	secure: true,
	sameSite: 'strict',
	path: '/',
};

interface SessionRecord {
	readonly adminId: string;
	readonly createdAt: number;
}

// Interim in-memory session store — replaced by the persistent KV/Postgres store in the shell.
let sessions = new Map<string, SessionRecord>();

/** Mint a session for a verified admin and return its opaque id (the cookie value). */
export function createSession(adminId: string): string {
	const id = globalThis.crypto.randomUUID();
	sessions.set(id, { adminId, createdAt: Date.now() });
	return id;
}

/** Resolve a session cookie value to its admin, or null if absent/unknown. */
export function getSession(id: string | undefined): { readonly adminId: string } | null {
	if (id === undefined) return null;
	const record = sessions.get(id);
	return record === undefined ? null : { adminId: record.adminId };
}

/** Test-only: clear all sessions (the dev `/api/test/reset` seam). */
export function resetSessions(): void {
	sessions = new Map<string, SessionRecord>();
}

/**
 * Resolve the acting admin id or redirect to sign-in. Used by authenticated `(app)` loads AND form
 * actions — actions don't run the layout `load` that gates the group, so they must re-check the session
 * (and it yields the `adminId` that becomes the `X-Admin-Id` actor on the I5 audit row, ADR-0026).
 */
export function requireAdminId(cookies: Cookies): string {
	const session = getSession(cookies.get(ADMIN_SESSION_COOKIE));
	if (session === null) redirect(307, '/admin/signin');
	return session.adminId;
}
