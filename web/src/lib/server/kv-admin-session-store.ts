// Durable admin-session store (spec 009 T06) — the persistent backend behind the post-assertion admin
// session cookie (§10-F). It replaces the spec-001 in-memory `Map` so a session survives Worker cold
// starts and multiple edge isolates (AC2). Backed by a Cloudflare **KV** namespace (`ADMIN_SESSIONS`),
// the same binding pattern the WebAuthn `CHALLENGES` KV already uses (D5/D6) — no new infra class, and
// (under Option B1) no extra Worker endpoint a Postgres session would have forced.
//
// FUNCTIONAL-CORE / IMPERATIVE-SHELL: this file is the PURE core — no SvelteKit-virtual imports
// (`$app`/`$env`), so it is unit-testable under the bare Vitest config (mirrors `kv-challenge-store.ts`).
// The imperative shell (`session.ts`) reads `dev` + the request `platform`, supplies the real clock and
// fallback, and exposes the async route-facing helpers.
//
// TWO TTL layers, by design (R2):
//   • KV-native `expirationTtl` — the eventual eviction backstop.
//   • an in-value `expiresAt` checked server-side on every `get` — the AUTHORITATIVE logical expiry.
// The server-side check is what makes TTL deterministically testable (advance an injected clock past
// `expiresAt` and the id is rejected immediately, without waiting for KV eviction) and is the real
// control — KV eviction is only a backstop, and `expirationTtl` is floored to KV's 60s minimum, which
// could otherwise outlive a shorter logical TTL.
//
// REVOCATION (sign-out) is best-effort within KV's propagation window (D5/R3): `revoke` deletes the key,
// but a delete can lag across isolates. Acceptable for a shorter-lived admin session (ADR-0016) and
// recorded as a known property, not a bug — the same window the `CHALLENGES` consume-once path lives with.

import type { KVNamespace } from '@cloudflare/workers-types';
import type { Cookies } from '@sveltejs/kit';

/** Stable cookie name for the post-assertion admin session (§10-F). */
export const ADMIN_SESSION_COOKIE = 'boundless_admin_session';

/**
 * §10-F cookie flags. `secure: true` is the contract; SvelteKit transparently relaxes it on
 * http://localhost for local dev/tests, so the wire bytes there omit `Secure` while the policy
 * (asserted in the store test) stays `secure: true`.
 */
export const SESSION_COOKIE_OPTIONS: Parameters<Cookies['set']>[2] = {
	httpOnly: true,
	secure: true,
	sameSite: 'strict',
	path: '/',
};

/** Cloudflare KV's minimum `expirationTtl` (seconds). A smaller value is rejected by the KV API. */
const KV_MIN_EXPIRATION_TTL_SECS = 60;

/**
 * Clamp a requested TTL up to KV's 60-second minimum. The admin `SESSION_TTL_SECS` is far above this,
 * so it never bites in practice; the floor exists only so the store can never throw a KV range error.
 * Clamping *up* lengthens only the KV-eviction backstop — the authoritative in-value `expiresAt` check
 * (R2) still rejects at the true logical TTL, so it cannot weaken expiry.
 */
function kvExpirationTtl(ttlSecs: number): number {
	return Math.max(ttlSecs, KV_MIN_EXPIRATION_TTL_SECS);
}

/** Injected unix-seconds clock (the only ambient input; injected for deterministic TTL tests). */
export interface Clock {
	now(): number;
}

/** What a resolved session yields to the caller — the acting admin id only (no other session metadata). */
export interface SessionView {
	readonly adminId: string;
}

/** The session-store port the shell selects between (KV in prod, in-memory in dev/test). */
export interface SessionStore {
	/** Mint a fresh opaque session id for a verified admin and persist it with its TTL. */
	create(adminId: string): Promise<string>;
	/** Resolve a session id to its admin, or null if absent (undefined cookie) / unknown / expired
	 *  (server-side TTL check). Accepts `undefined` so callers can pass `cookies.get(...)` directly. */
	get(id: string | undefined): Promise<SessionView | null>;
	/** Revoke (sign-out) a session id — best-effort within KV's propagation window (D5/R3). */
	revoke(id: string): Promise<void>;
}

/** The JSON shape persisted in KV under the opaque session id. */
interface StoredSession {
	readonly adminId: string;
	readonly expiresAt: number;
}

/** Parse a stored value defensively — a malformed/legacy blob resolves to null (treated as no session). */
function parseStored(raw: string | null): StoredSession | null {
	if (raw === null) return null;
	try {
		const value = JSON.parse(raw) as Partial<StoredSession>;
		if (typeof value.adminId === 'string' && typeof value.expiresAt === 'number') {
			return { adminId: value.adminId, expiresAt: value.expiresAt };
		}
	} catch {
		// fall through — malformed value is treated as no session
	}
	return null;
}

export class KvSessionStore implements SessionStore {
	constructor(
		private readonly kv: KVNamespace,
		private readonly clock: Clock,
		private readonly ttlSecs: number,
	) {}

	async create(adminId: string): Promise<string> {
		// ≥128-bit opaque id (R1) — a v4 UUID is 122 bits of randomness, not derived from `adminId`.
		const id = globalThis.crypto.randomUUID();
		const value: StoredSession = { adminId, expiresAt: this.clock.now() + this.ttlSecs };
		await this.kv.put(id, JSON.stringify(value), { expirationTtl: kvExpirationTtl(this.ttlSecs) });
		return id;
	}

	async get(id: string | undefined): Promise<SessionView | null> {
		if (id === undefined) return null; // absent cookie — never hits KV
		const stored = parseStored(await this.kv.get(id, 'text'));
		if (stored === null) return null;
		// Authoritative server-side TTL (R2): reject at the logical expiry even if KV hasn't evicted yet.
		if (this.clock.now() >= stored.expiresAt) return null;
		return { adminId: stored.adminId };
	}

	async revoke(id: string): Promise<void> {
		// Idempotent if already gone; propagation is best-effort across isolates (D5/R3).
		await this.kv.delete(id);
	}
}

/**
 * In-memory fallback for dev/test (a per-isolate `Map`). Enforces the SAME server-side TTL as the KV
 * store via the injected clock. NEVER used in production — `selectSessionStore` fails closed there,
 * because a per-isolate `Map` cannot survive a cold start or be shared across edge isolates (AC2).
 */
export class MemorySessionStore implements SessionStore {
	private readonly sessions = new Map<string, StoredSession>();

	constructor(
		private readonly clock: Clock,
		private readonly ttlSecs: number,
	) {}

	async create(adminId: string): Promise<string> {
		const id = globalThis.crypto.randomUUID();
		this.sessions.set(id, { adminId, expiresAt: this.clock.now() + this.ttlSecs });
		return id;
	}

	async get(id: string | undefined): Promise<SessionView | null> {
		if (id === undefined) return null;
		const stored = this.sessions.get(id);
		if (stored === undefined) return null;
		if (this.clock.now() >= stored.expiresAt) return null;
		return { adminId: stored.adminId };
	}

	async revoke(id: string): Promise<void> {
		this.sessions.delete(id);
	}
}

/**
 * Pick the admin-session store for this request. Returns the real Cloudflare **KV** store when an
 * `ADMIN_SESSIONS` binding is present (the edge, and `vite dev`/Playwright via adapter-cloudflare's
 * getPlatformProxy). When the binding is ABSENT it falls back to the per-isolate in-memory store ONLY
 * when `allowInMemoryFallback` is true (dev/test) — otherwise it **fails closed** by throwing, because
 * an in-memory `Map` cannot survive a Worker cold start or be shared across edge isolates (AC2) and would
 * silently mask a binding misconfiguration. Callers pass `dev` ($app/environment) as the flag.
 *
 * Pure (no SvelteKit-virtual imports) so it is unit-testable under the bare Vitest config — the shell
 * (`session.ts`) supplies the platform binding, the clock, the TTL, the fallback singleton, and `dev`.
 */
export function selectSessionStore(
	kv: KVNamespace | undefined,
	clock: Clock,
	ttlSecs: number,
	fallback: SessionStore,
	allowInMemoryFallback: boolean,
): SessionStore {
	if (kv) return new KvSessionStore(kv, clock, ttlSecs);
	if (allowInMemoryFallback) return fallback;
	throw new Error(
		'Admin session store unavailable: the ADMIN_SESSIONS KV namespace is not bound. ' +
			'Refusing the in-memory fallback outside dev — it cannot persist sessions across Worker ' +
			'cold starts or edge isolates (spec 009 D5/AC2).',
	);
}
