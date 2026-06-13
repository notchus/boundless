// Worker-backed admin-WebAuthn persistence adapters (spec 009 **T05**, ADR-0027) — the production
// `InviteStore`/`CredentialStore` implementations that move WebAuthn invite + credential persistence
// behind the Rust admin Worker's B1 endpoints (`/api/admin/webauthn/*`, T04). Under Option B1 the web
// tier keeps **zero direct Postgres and zero crypto**: the invite-token HMAC compare runs in the core
// (the ADR-0017 P4 carve-out, AC4b) — this module only speaks HTTP to the Worker.
//
// FUNCTIONAL-CORE / IMPERATIVE-SHELL (mirrors `members.ts` / `kv-challenge-store.ts`): this module is
// PURE — wire DTOs derived by hand from the frozen `api/openapi.yaml` (no TS codegen yet), the two
// Worker fetch adapters, and the fail-closed `selectInviteStore`/`selectCredentialStore` decisions. It
// imports NO SvelteKit-virtual modules, so it is unit-testable under the bare Vitest config; the shell
// (`webauthn-deps.ts`, T07) supplies the Worker base URL + secret (from `$env/dynamic/private`), the
// in-memory fallbacks, `dev`, and a per-request handshake. Living under `src/lib/server/` means the
// shared secret can never leak into client code.
//
// ## The pre-session credential (ADR-0026/0027)
// These four ops run BEFORE a verified admin session exists (the admin is being registered or
// authenticated), so they carry the server-to-server **shared secret only** — NO `X-Admin-Id` (the
// deliberate difference from `members.ts`). A non-outcome status is an operator fault → a value-free
// throw that never echoes the response body (P2).
//
// ## Why a handshake (R11 + R12)
// The frozen contract collapses the registration write into ONE atomic op — `register-complete`
// (consume-invite + revoke-priors + insert, R11) — because edge-TS cannot make three network calls
// atomic. But `register.ts` is **unchanged** (R12): its `verifyRegistration` tail still calls three
// SEPARATE port methods — `invites.markConsumed` → `credentials.revokeAllForAdmin` →
// `credentials.insert`. The `WorkerRegistrationHandshake` is the minimal shared per-request state that
// bridges the two: `markConsumed` stashes the presented token (no network), `insert` reads it and fires
// the single atomic `register-complete`, and `revokeAllForAdmin` is a no-op (the revoke happens
// server-side in that same txn). The two stores must be constructed sharing ONE handshake per request
// (the shell's job, T07).
//
// ## base64url at the seam
// `bytea` wire fields are base64url-no-pad. The mapping matches the `@simplewebauthn` port types
// (`webauthn/ports.ts`): `public_key` is DECODED inbound to a `Uint8Array` (`StoredCredential.publicKey`
// is bytes) and ENCODED outbound from the ceremony's `Uint8Array`; `credential_id` stays a base64url
// STRING both ways (the port type). `aaguid` is the subtle one: `@simplewebauthn` gives register.ts a
// **dashed-hex UUID string** (e.g. "adce0002-35bc-…", the port form) while the wire/core
// `NewAdminCredential.aaguid` is base64url-no-pad of the 16 AAGUID **bytes** — so the adapter CONVERTS
// it (UUID-string ↔ base64url-bytea) in both directions. register.ts is frozen (R12), so this
// conversion can only live here; a malformed/absent aaguid is OMITTED, never shipped as wrong bytes.

import type { AuthenticatorTransportFuture } from '@simplewebauthn/server';

import { WebAuthnError } from './errors';
import type { CredentialStore, InviteRecord, InviteStore, StoredCredential } from './ports';

// ── Wire DTOs — hand-derived from the frozen `api/openapi.yaml` B1 schemas (single-sourced in
//    `core::server::admin_webauthn`; snake_case, base64url-no-pad bytea, epoch-second ints). ──

interface AdminInviteRecordWire {
	readonly admin_id: string;
	readonly group_id: string;
	readonly expires_at: number;
	readonly consumed_at: number | null;
}

interface AdminCredentialWire {
	readonly credential_id: string;
	readonly admin_id: string;
	readonly public_key: string;
	readonly sign_count: number;
	readonly transports?: string[];
	readonly aaguid?: string;
	readonly revoked_at: number | null;
}

/** The `register-complete` request's credential leg (`NewAdminCredential`; no `admin_id`/`revoked_at`). */
interface NewAdminCredentialWire {
	credential_id: string;
	public_key: string;
	sign_count: number;
	transports?: string[];
	aaguid?: string;
}

// ── base64url-no-pad codec (workerd + Node both ship global `atob`/`btoa`). ──

function b64urlEncode(bytes: Uint8Array): string {
	let s = '';
	for (const b of bytes) s += String.fromCharCode(b);
	return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function b64urlDecode(value: string): Uint8Array<ArrayBuffer> {
	const b64 = value.replace(/-/g, '+').replace(/_/g, '/');
	const bin = atob(b64);
	const out = new Uint8Array(bin.length);
	for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
	return out;
}

/**
 * Convert the ceremony's AAGUID (the `@simplewebauthn` dashed-hex UUID string, the port form) to the
 * wire's base64url-no-pad of the 16 AAGUID **bytes** (`NewAdminCredential.aaguid`). Returns `undefined`
 * for anything that is not a clean 16-byte hex UUID, so a malformed/absent value is OMITTED rather than
 * silently shipped as wrong bytes (the Worker would otherwise base64url-decode a dashed UUID — all
 * valid base64url chars — into ~26 garbage bytes).
 */
function aaguidToWire(aaguid: string | undefined): string | undefined {
	if (aaguid === undefined) return undefined;
	const hex = aaguid.replace(/-/g, '');
	if (!/^[0-9a-fA-F]{32}$/.test(hex)) return undefined;
	const bytes = new Uint8Array(16);
	for (let i = 0; i < 16; i++) bytes[i] = parseInt(hex.slice(2 * i, 2 * i + 2), 16);
	return b64urlEncode(bytes);
}

/** The {@link aaguidToWire} inverse: the wire's base64url-no-pad of 16 bytes → the dashed-hex UUID
 *  string the port carries (the `@simplewebauthn` form). `undefined` if not exactly 16 bytes. */
function aaguidFromWire(value: string | undefined): string | undefined {
	if (value === undefined) return undefined;
	const bytes = b64urlDecode(value);
	if (bytes.length !== 16) return undefined;
	const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
	return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

// ── Shared B1 transport helpers (the pre-session shared-secret credential; value-free errors). ──

function b1Url(baseUrl: string, path: string): string {
	return `${baseUrl.replace(/\/$/, '')}${path}`;
}

/** Pre-session B1 headers: the Bearer shared secret only — NO `x-admin-id` (ADR-0027). */
function b1Headers(secret: string): Record<string, string> {
	return { authorization: `Bearer ${secret}`, 'content-type': 'application/json' };
}

async function b1Post(baseUrl: string, secret: string, path: string, body: unknown): Promise<Response> {
	return fetch(b1Url(baseUrl, path), {
		method: 'POST',
		headers: b1Headers(secret),
		body: JSON.stringify(body),
	});
}

/** A non-outcome status (401 BFF-misconfig, or anything unexpected) is an operator fault. Throw a
 *  value-free error — never echo the Worker's response body onto the BFF's own error/log path (P2). */
function fail(res: Response): Error {
	return new Error(`admin webauthn Worker call failed: HTTP ${res.status}`);
}

function toStoredCredential(w: AdminCredentialWire): StoredCredential {
	return {
		credentialId: w.credential_id, // keep the base64url string (port type is string)
		adminId: w.admin_id,
		publicKey: b64urlDecode(w.public_key), // → Uint8Array for @simplewebauthn
		signCount: w.sign_count,
		transports: w.transports as AuthenticatorTransportFuture[] | undefined,
		aaguid: aaguidFromWire(w.aaguid), // base64url-bytea → dashed-hex UUID string (the port form)
		revokedAt: w.revoked_at,
	};
}

function toNewCredentialWire(c: StoredCredential): NewAdminCredentialWire {
	const wire: NewAdminCredentialWire = {
		credential_id: c.credentialId, // already base64url
		public_key: b64urlEncode(c.publicKey), // Uint8Array → base64url
		sign_count: c.signCount,
	};
	if (c.transports) wire.transports = c.transports;
	// Convert the ceremony's dashed-hex UUID aaguid → base64url-no-pad of the 16 bytes (the wire/core
	// form); a malformed/absent value is omitted (never shipped as wrong bytes). See aaguidToWire.
	const aaguid = aaguidToWire(c.aaguid);
	if (aaguid !== undefined) wire.aaguid = aaguid;
	return wire;
}

/**
 * Per-request shared state bridging `register.ts`'s three-call registration tail to the single atomic
 * `register-complete` Worker op. Set by {@link WorkerInviteStore.markConsumed}; read+cleared by
 * {@link WorkerCredentialStore.insert}. The shell (T07) creates ONE per request and gives it to both
 * Worker stores. See the module header ("Why a handshake").
 */
export class WorkerRegistrationHandshake {
	/** The presented invitation token awaiting an atomic consume via `register-complete`. */
	presentedToken: string | null = null;
}

// ── The Worker-backed invite store (B1: invite/resolve + the consume folded into register-complete) ──

export class WorkerInviteStore implements InviteStore {
	constructor(
		private readonly baseUrl: string,
		private readonly secret: string,
		private readonly handshake: WorkerRegistrationHandshake,
	) {}

	async load(presentedToken: string): Promise<InviteRecord | null> {
		const res = await b1Post(this.baseUrl, this.secret, '/api/admin/webauthn/invite/resolve', {
			token: presentedToken,
		});
		if (res.status === 200) {
			const w = (await res.json()) as AdminInviteRecordWire;
			return { adminId: w.admin_id, groupId: w.group_id, expiresAt: w.expires_at, consumedAt: w.consumed_at };
		}
		if (res.status === 404) return null; // value-free no-match (unknown or cross-tenant) → expired verdict
		throw fail(res);
	}

	/**
	 * The single-use consume is NOT a standalone Worker op — it happens atomically inside
	 * `register-complete` (R11). So this only records the presented token for the
	 * `WorkerCredentialStore.insert` that immediately follows in `register.ts`'s unchanged tail (R12).
	 * No network call. (`now` is unused — server time is the Worker's.)
	 */
	async markConsumed(presentedToken: string, _now: number): Promise<void> {
		this.handshake.presentedToken = presentedToken;
	}
}

// ── The Worker-backed credential store (B1: lookup + sign-count; insert via register-complete) ──

export class WorkerCredentialStore implements CredentialStore {
	constructor(
		private readonly baseUrl: string,
		private readonly secret: string,
		private readonly handshake: WorkerRegistrationHandshake,
	) {}

	/**
	 * There is no frozen pre-session "list an admin's credentials" op — that is session-bearing
	 * (additive backup-key enrollment) and deferred (T03). The invite-gated registration
	 * revoke-and-replaces, so `excludeCredentials` is moot; usernameless sign-in passes
	 * `adminId=undefined` so this is never reached there. Returns `[]`.
	 */
	async listActiveByAdmin(_adminId: string): Promise<StoredCredential[]> {
		return [];
	}

	async findActive(credentialId: string): Promise<StoredCredential | null> {
		const res = await b1Post(this.baseUrl, this.secret, '/api/admin/webauthn/credentials/lookup', {
			credential_id: credentialId,
		});
		if (res.status === 200) return toStoredCredential((await res.json()) as AdminCredentialWire);
		if (res.status === 404) return null; // revoked / unknown / cross-tenant → verification fails
		throw fail(res);
	}

	/**
	 * The registration write (consume-invite + revoke-priors + insert) is ONE atomic Worker op
	 * (`register-complete`, R11). `insert` is the last of `register.ts`'s three calls, so by here the
	 * handshake holds the presented token (set by {@link WorkerInviteStore.markConsumed}). Fire the
	 * single atomic op; the prior-credential revoke happens server-side in the same txn (so
	 * {@link revokeAllForAdmin} is a no-op). The 200 body (`{admin_id}`) is ignored — `register.ts`
	 * already has the admin id from the resolved invite.
	 */
	async insert(credential: StoredCredential): Promise<void> {
		const token = this.handshake.presentedToken;
		if (token == null) {
			// No live invite handshake. The additive backup-key insert (authenticated, no invite) has no
			// frozen pre-session endpoint and is not wired (DEFERRED → the backup-key enrollment flow).
			throw new Error(
				'WorkerCredentialStore.insert requires an invite handshake (register-complete); the standalone ' +
					'additive-credential insert is not wired (no pre-session endpoint, ADR-0027)',
			);
		}
		this.handshake.presentedToken = null;
		const res = await b1Post(this.baseUrl, this.secret, '/api/admin/webauthn/register-complete', {
			token,
			credential: toNewCredentialWire(credential),
		});
		if (res.status === 200) return;
		// TOCTOU backstop: the invite was consumed/expired between the edge `evaluateInvite` and this
		// atomic consume. Surface the registered, value-free code (routes to InviteExpired); no body echo.
		if (res.status === 400) throw new WebAuthnError('ADMIN_INVITE_CONSUMED');
		throw fail(res);
	}

	/**
	 * No standalone pre-session revoke-all op: `register-complete` revokes the admin's prior credentials
	 * atomically in the same txn (ADR-0016 D4). So on the invite-gated path this is a no-op. (A
	 * standalone bulk-revoke would need its own session-bearing op; not wired.)
	 */
	async revokeAllForAdmin(_adminId: string, _now: number): Promise<void> {
		// intentionally empty — see the doc comment above.
	}

	async bumpSignCount(credentialId: string, newCount: number, _now: number): Promise<void> {
		const res = await b1Post(
			this.baseUrl,
			this.secret,
			`/api/admin/webauthn/credentials/${encodeURIComponent(credentialId)}/sign-count`,
			{ sign_count: newCount },
		);
		if (res.status === 204) return; // applied or only-if-greater no-op
		throw fail(res);
	}
}

// ── Fail-closed selection (mirrors `selectMembersClient` / `selectChallengeStore`) ──

/**
 * Pick the invite store for a request. Returns the real {@link WorkerInviteStore} (ADR-0027) when BOTH
 * the Worker base URL and the shared secret are configured (the deployed edge). When either is absent it
 * falls back to the in-memory store ONLY when `allowInMemoryFallback` is true (dev/e2e) — otherwise it
 * **fails closed** by throwing, so a production build with no Worker binding refuses to serve a fake
 * invite backend rather than degrade silently (AC1/AC4b). The `handshake` is shared with
 * {@link selectCredentialStore} for the same request (the register-complete coalescing).
 */
export function selectInviteStore(
	baseUrl: string | undefined,
	secret: string | undefined,
	fallback: InviteStore,
	allowInMemoryFallback: boolean,
	handshake: WorkerRegistrationHandshake,
): InviteStore {
	if (baseUrl && secret) return new WorkerInviteStore(baseUrl, secret, handshake);
	if (allowInMemoryFallback) return fallback;
	throw new Error(
		'Admin WebAuthn invite backend unavailable: ADMIN_WORKER_BASE / ADMIN_API_SECRET are not ' +
			'configured. Refusing the in-memory fallback outside dev — it would serve a fake invite backend ' +
			'and mask a deploy misconfiguration (ADR-0026/0027).',
	);
}

/**
 * Pick the credential store for a request — the {@link selectInviteStore} twin. Real
 * {@link WorkerCredentialStore} when configured; in-memory fallback only in dev; else **fails closed**.
 * Shares the `handshake` with the invite store (same request) so the registration write coalesces into
 * the single atomic `register-complete` op (R11).
 */
export function selectCredentialStore(
	baseUrl: string | undefined,
	secret: string | undefined,
	fallback: CredentialStore,
	allowInMemoryFallback: boolean,
	handshake: WorkerRegistrationHandshake,
): CredentialStore {
	if (baseUrl && secret) return new WorkerCredentialStore(baseUrl, secret, handshake);
	if (allowInMemoryFallback) return fallback;
	throw new Error(
		'Admin WebAuthn credential backend unavailable: ADMIN_WORKER_BASE / ADMIN_API_SECRET are not ' +
			'configured. Refusing the in-memory fallback outside dev — it would serve a fake credential ' +
			'backend and mask a deploy misconfiguration (ADR-0026/0027).',
	);
}
