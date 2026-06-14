// Composition root for the admin-WebAuthn routes (spec 001 T15) — builds the `WebAuthnDeps` the T09
// verification functions consume.
//
// FUNCTIONAL-CORE / IMPERATIVE-SHELL: the verification LOGIC (T09, `$lib/server/webauthn`) is
// unchanged; this file only wires its ports. The **challenge** store is now the real Cloudflare **KV**
// store (`KvChallengeStore`, ADR-0017 D3) whenever the `CHALLENGES` binding is present — on the edge
// under adapter-cloudflare, and in `vite dev`/Playwright where the adapter exposes a local Miniflare KV
// (T15-shell leg A). It falls back to the in-memory store only for Vitest unit tests / adapterless runs.
// The **invite/credential** stores are now the **Worker-backed** B1 adapters (`selectInviteStore` /
// `selectCredentialStore`, spec 009 T05/T07) whenever `ADMIN_WORKER_BASE` + `ADMIN_API_SECRET` are
// configured (the deployed edge) — the web tier holds zero Postgres + zero crypto; the invite-token HMAC
// runs in the Rust core (ADR-0017 P4 carve-out / ADR-0027). They fall back to the in-memory stores ONLY in
// dev/test (the selectors fail closed in prod, so a misconfigured deploy refuses to serve a fake backend).
// The RP config is resolved by the pure `resolveRpConfig`. A SINGLE per-request `WorkerRegistrationHandshake`
// is shared by both stores so `register.ts`'s three-call registration tail coalesces into the one atomic
// `register-complete` Worker op (R11), while `register.ts`/`authenticate.ts` stay unchanged (R12).

import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
import type { Cookies } from '@sveltejs/kit';
import type { PublicKeyCredentialCreationOptionsJSON } from '@simplewebauthn/server';

import { buildRegistrationOptions, CHALLENGE_TTL_SECS, evaluateInvite, WebAuthnError } from '$lib/server/webauthn';
import type { ChallengeStore, Clock, RpConfig, WebAuthnDeps } from '$lib/server/webauthn';
import { selectChallengeStore } from '$lib/server/webauthn/kv-challenge-store';
import { resolveRpConfig } from '$lib/server/webauthn/rp-config';
import {
	selectCredentialStore,
	selectInviteStore,
	WorkerRegistrationHandshake,
} from '$lib/server/webauthn/worker-stores';
import {
	MemoryChallengeStore,
	MemoryCredentialStore,
	MemoryInviteStore,
} from '$lib/server/webauthn/testing/memory-stores';

/** Real server clock (unix seconds) — the only ambient input; everything else is injected. */
const clock: Clock = { now: () => Math.floor(Date.now() / 1000) };

// In-memory backend — now the dev/test FALLBACK for all three ports. `challenges` is used when no
// `CHALLENGES` KV binding is present (Vitest/adapterless); `invites`/`credentials` are used when
// `ADMIN_WORKER_BASE`/`ADMIN_API_SECRET` are unconfigured (dev/e2e under `vite dev`, where the Worker-backed
// selectors fall back to these). `let` so the dev-only `/api/test/reset` seam can swap them for per-test
// isolation. In production the selectors return the real stores (or fail closed); these are never reached.
let challenges = new MemoryChallengeStore(clock);
let invites = new MemoryInviteStore();
let credentials = new MemoryCredentialStore();

/**
 * Pick the challenge store for this request: the real Cloudflare **KV** store (ADR-0017 D3) when the
 * `CHALLENGES` binding is present — on the edge under adapter-cloudflare, or in `vite dev` where the
 * adapter exposes a local Miniflare KV via wrangler's getPlatformProxy.
 *
 * When the binding is absent this **fails closed** outside dev: the in-memory fallback is dev/test-only
 * (Vitest unit tests + any adapterless run), because on a real multi-isolate edge deploy a per-isolate
 * `Map` cannot guarantee consume-once across isolates (it would silently break one-time-use and mask a
 * binding misconfiguration). The decision lives in the pure `selectChallengeStore`; here we just supply
 * the binding, the current fallback singleton (so a dev `/api/test/reset` swap is honoured), and `dev`.
 */
function challengeStore(platform: App.Platform | undefined): ChallengeStore {
	return selectChallengeStore(platform?.env?.CHALLENGES, challenges, dev);
}

/**
 * Relying-Party config. Production MUST pin these via env (`WEBAUTHN_RP_ID`/`WEBAUTHN_ORIGIN`/
 * `WEBAUTHN_RP_NAME`, the wrangler.toml `[vars]`) to the known admin domain — never trust the request
 * Host. The pure `resolveRpConfig` (spec 009 T09) owns the policy: env wins; dev falls back to the request
 * URL (localhost); outside dev an unset RP config fails closed (throws) rather than trusting the Host.
 */
function rpConfig(url: URL): RpConfig {
	// Narrow `$env/dynamic/private` (an index-signature Record) to the RP slice the resolver reads.
	return resolveRpConfig(
		{
			WEBAUTHN_RP_NAME: env.WEBAUTHN_RP_NAME,
			WEBAUTHN_RP_ID: env.WEBAUTHN_RP_ID,
			WEBAUTHN_ORIGIN: env.WEBAUTHN_ORIGIN,
		},
		url,
		{ dev },
	);
}

/**
 * Build the deps for one request. Selects the real KV challenge store when `platform.env.CHALLENGES` is
 * bound (else the in-memory fallback), and the Worker-backed invite/credential stores when
 * `ADMIN_WORKER_BASE`+`ADMIN_API_SECRET` are configured (else the dev fallbacks; fail-closed in prod).
 * ONE `WorkerRegistrationHandshake` is shared by both stores for this request — that is the R11
 * register-complete coalescing (`markConsumed` stashes the token, `insert` fires the one atomic op).
 */
export function getWebAuthnDeps(url: URL, platform?: App.Platform): WebAuthnDeps {
	const handshake = new WorkerRegistrationHandshake();
	return {
		rp: rpConfig(url),
		clock,
		challenges: challengeStore(platform),
		invites: selectInviteStore(env.ADMIN_WORKER_BASE, env.ADMIN_API_SECRET, invites, dev, handshake),
		credentials: selectCredentialStore(env.ADMIN_WORKER_BASE, env.ADMIN_API_SECRET, credentials, dev, handshake),
	};
}

// — Ceremony challenge key (the KV-challenge key in production) round-tripped via a short-lived
//   httpOnly cookie so the verify call can retrieve the challenge stored by the options call. —

export const CEREMONY_COOKIE = 'boundless_webauthn_ceremony';
export const CEREMONY_COOKIE_OPTIONS: Parameters<import('@sveltejs/kit').Cookies['set']>[2] = {
	httpOnly: true,
	secure: true,
	sameSite: 'strict',
	path: '/',
	maxAge: CHALLENGE_TTL_SECS, // single-sourced so the cookie window can't drift from the KV TTL
};

export function newCeremonyKey(): string {
	return globalThis.crypto.randomUUID();
}

/**
 * Resolve only whether an invite is usable (live vs dead) at server time — WITHOUT building
 * registration options or minting a challenge/cookie. Used by the SSR page load to pick the initial
 * view (register prompt vs InviteExpired) with no loading flash; the actual ceremony challenge is
 * minted lazily by the first register click (`startRegistrationCeremony` via the GET endpoint).
 */
export async function resolveInviteStatus(token: string): Promise<'live' | 'expired'> {
	// Resolve via the SELECTED invite store (the Worker-backed one in prod, the dev fallback otherwise) so
	// the SSR status matches the ceremony's store. `load` is read-only — the throwaway handshake is unused.
	const store = selectInviteStore(env.ADMIN_WORKER_BASE, env.ADMIN_API_SECRET, invites, dev, new WorkerRegistrationHandshake());
	const verdict = evaluateInvite(await store.load(token), clock.now());
	return verdict.status === 'live' ? 'live' : 'expired';
}

/** A resolved invitation: either WebAuthn registration options (live) or the dead-invite code. */
export type InviteCeremony =
	| { readonly status: 'live'; readonly options: PublicKeyCredentialCreationOptionsJSON }
	| { readonly status: 'expired'; readonly code: string };

/**
 * Resolve an invitation and, if live, build registration options + persist the challenge under a
 * fresh ceremony key set as the `CEREMONY_COOKIE`. Shared by the SSR page `load` (no loading flash)
 * and the contract `GET /api/admin/auth/invite/[token]` endpoint (client retries) — one source of
 * truth for the registration-ceremony prep. Never reveals existence via status code (both legs 200).
 */
export async function startRegistrationCeremony(
	url: URL,
	cookies: Cookies,
	token: string,
	platform?: App.Platform,
): Promise<InviteCeremony> {
	const deps = getWebAuthnDeps(url, platform);
	const ceremonyKey = newCeremonyKey();
	try {
		const options = await buildRegistrationOptions(deps, {
			ceremonyKey,
			presentedToken: token,
			// Non-PII display labels for the OS passkey UI (the opaque adminId is the userID).
			userName: 'boundless-admin',
			userDisplayName: 'Boundless admin',
		});
		cookies.set(CEREMONY_COOKIE, ceremonyKey, CEREMONY_COOKIE_OPTIONS);
		return { status: 'live', options };
	} catch (e) {
		if (e instanceof WebAuthnError) {
			return { status: 'expired', code: e.code };
		}
		throw e;
	}
}

// — Dev-only test seams (the `/api/test/*` routes guard these behind `dev`) —
//
// Spec 009 T07 / Option A: these seed/reset the DEV-DURABLE backends — here the in-memory invite/credential
// fallbacks that `vite dev`/Playwright actually run against (no Worker is configured in dev, so the
// selectors fall back to them). They are `dev`-gated AND tree-shaken from the production bundle (`dev`
// inlines to `false`), proven by `tests/build-gates/no-dev-seams.test.ts` (AC5) — so no authority-minting seam is
// reachable in prod (R21/I11). (The literal "delete the authority seams" was weighed against this and
// declined: an invite-seed is unavoidable for the dev onboarding e2e, and AC5 is the real guarantee — see
// DEFERRED.md spec 009 T07.)

export interface SeedInviteInput {
	readonly token: string;
	readonly adminId: string;
	readonly groupId: string;
	readonly expiresAt: number;
	readonly consumedAt?: number | null;
}

/** Seed a pending-admin invitation into the dev-durable (in-memory) backend (dev/test only; the real
 *  invite is operator-seeded into Postgres and resolved via the Worker B1 endpoint in prod). */
export function seedInvite(input: SeedInviteInput): void {
	invites.add(input.token, {
		adminId: input.adminId,
		groupId: input.groupId,
		expiresAt: input.expiresAt,
		consumedAt: input.consumedAt ?? null,
	});
}

/** Reset the dev-durable (in-memory) fallback backend (dev/test only) for per-test isolation. */
export function resetStores(): void {
	challenges = new MemoryChallengeStore(clock);
	invites = new MemoryInviteStore();
	credentials = new MemoryCredentialStore();
}
