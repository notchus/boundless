// Post-assertion admin session (spec 001 T15; spec 009 T06). After a verified WebAuthn sign-in
// assertion the route mints a session id and sets it as an **httpOnly + Secure + SameSite=Strict**
// cookie — the web leg of security R5's at-rest credential storage (never localStorage —
// forbidden-patterns). Admin sessions are separate from and shorter-lived than member sessions
// (ADR-0016).
//
// IMPERATIVE SHELL: the durable store + the fail-closed selection live in the pure
// `./kv-admin-session-store` (unit-tested under bare Vitest); this shell reads `dev` + the request
// `platform`, supplies the real clock + the dev fallback singleton, and exposes the async route-facing
// helpers. The session DATA persists in the `ADMIN_SESSIONS` Cloudflare KV (spec 009 D5) — surviving
// Worker cold starts (AC2) — falling back to a per-isolate in-memory store only in dev/test.

import { dev } from '$app/environment';
import { redirect, type Cookies } from '@sveltejs/kit';

import {
	type Clock,
	MemorySessionStore,
	selectSessionStore,
	type SessionStore,
	type SessionView,
} from './kv-admin-session-store';

// Re-export the cookie policy from the pure core so route imports stay `$lib/server/session`.
export { ADMIN_SESSION_COOKIE, SESSION_COOKIE_OPTIONS } from './kv-admin-session-store';
import { ADMIN_SESSION_COOKIE } from './kv-admin-session-store';

/**
 * Admin session lifetime. ADR-0016: admin sessions are "separate and shorter-lived" than the indefinite
 * member sessions — Sarah re-asserts her passkey when she returns (the laptop surface). 12 hours covers a
 * working day and expires by the next morning; a tunable constant, not a wire contract.
 */
const SESSION_TTL_SECS = 12 * 60 * 60;

/** Real server clock (unix seconds) — the only ambient input; the store takes it injected for tests. */
const clock: Clock = { now: () => Math.floor(Date.now() / 1000) };

// Dev/test fallback used ONLY when no `ADMIN_SESSIONS` binding is present (adapterless/bare runs). `let`
// so the dev `/api/test/reset` seam can swap it for per-test isolation. In `vite dev`/Playwright the
// adapter exposes a real Miniflare KV, so the live path is KV-backed even locally.
let fallback = new MemorySessionStore(clock, SESSION_TTL_SECS);

/** The session store for this request: the real KV store when `ADMIN_SESSIONS` is bound, else the dev
 *  fallback, else fail closed (mirrors `members-deps.ts`/`webauthn-deps.ts`). */
function sessionStore(platform: App.Platform | undefined): SessionStore {
	return selectSessionStore(platform?.env?.ADMIN_SESSIONS, clock, SESSION_TTL_SECS, fallback, dev);
}

/** Mint a session for a verified admin and return its opaque id (the cookie value). */
export async function createSession(adminId: string, platform: App.Platform | undefined): Promise<string> {
	return sessionStore(platform).create(adminId);
}

/** Resolve a session cookie value to its admin, or null if absent/unknown/expired (server-side TTL). */
export async function getSession(
	id: string | undefined,
	platform: App.Platform | undefined,
): Promise<SessionView | null> {
	// Short-circuit an absent cookie before constructing the store, so a no-session request stays a calm
	// null (→ redirect to sign-in) and never trips the fail-closed throw on a misconfigured prod (the
	// store's get() also guards undefined defensively — this is the no-store-construction fast path).
	if (id === undefined) return null;
	return sessionStore(platform).get(id);
}

/** Revoke (sign-out) a session — best-effort within KV's propagation window (D5/R3). No-op if absent. */
export async function revokeSession(id: string | undefined, platform: App.Platform | undefined): Promise<void> {
	if (id === undefined) return;
	await sessionStore(platform).revoke(id);
}

/** Test-only: clear the in-memory fallback (the dev `/api/test/reset` seam). KV-backed sessions in
 *  `vite dev` are isolated per test by their fresh-random ids, not by this reset. */
export function resetSessions(): void {
	fallback = new MemorySessionStore(clock, SESSION_TTL_SECS);
}

/**
 * Resolve the acting admin id or redirect to sign-in. Used by authenticated `(app)` loads AND form
 * actions — actions don't run the layout `load` that gates the group, so they must re-check the session
 * (and it yields the `adminId` that becomes the `X-Admin-Id` actor on the I5 audit row, ADR-0026).
 */
export async function requireAdminId(cookies: Cookies, platform: App.Platform | undefined): Promise<string> {
	const session = await getSession(cookies.get(ADMIN_SESSION_COOKIE), platform);
	if (session === null) redirect(307, '/admin/signin');
	return session.adminId;
}
