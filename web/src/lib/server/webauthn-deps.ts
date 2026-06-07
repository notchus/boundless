// Composition root for the admin-WebAuthn routes (spec 001 T15) — builds the `WebAuthnDeps` the T09
// verification functions consume.
//
// FUNCTIONAL-CORE / IMPERATIVE-SHELL: the verification LOGIC (T09, `$lib/server/webauthn`) is
// unchanged; this file only wires its ports. The **challenge** store is now the real Cloudflare **KV**
// store (`KvChallengeStore`, ADR-0017 D3) whenever the `CHALLENGES` binding is present — on the edge
// under adapter-cloudflare, and in `vite dev`/Playwright where the adapter exposes a local Miniflare KV
// (T15-shell leg A). It falls back to the in-memory store only for Vitest unit tests / adapterless runs.
// The **invite/credential** stores remain in-memory (the **Postgres** stores over Hyperdrive — incl. the
// invite-token HMAC routed through the core per ADR-0017's P4 carve-out — are T15-shell leg B,
// DEFERRED.md). The RP config is derived from the request URL (env-overridable). This mirrors the Rust
// Worker composing a real store for one port and an in-memory `DeviceStore` for another until its
// dependency lands. There is no production deploy yet, so the in-memory invite/credential backend cannot
// accidentally serve real traffic.

import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
import type { Cookies } from '@sveltejs/kit';
import type { PublicKeyCredentialCreationOptionsJSON } from '@simplewebauthn/server';

import { buildRegistrationOptions, CHALLENGE_TTL_SECS, evaluateInvite, WebAuthnError } from '$lib/server/webauthn';
import type { ChallengeStore, Clock, RpConfig, WebAuthnDeps } from '$lib/server/webauthn';
import { selectChallengeStore } from '$lib/server/webauthn/kv-challenge-store';
import {
	MemoryChallengeStore,
	MemoryCredentialStore,
	MemoryInviteStore,
} from '$lib/server/webauthn/testing/memory-stores';

/** Real server clock (unix seconds) — the only ambient input; everything else is injected. */
const clock: Clock = { now: () => Math.floor(Date.now() / 1000) };

// In-memory backend. `challenges` is now only the FALLBACK (used when no KV binding is present, e.g.
// Vitest); `invites`/`credentials` remain the live interim stores (leg B). `let` so the dev-only
// `/api/test/reset` seam can swap them.
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
 * `WEBAUTHN_RP_NAME`) to the known admin domain — never trust the request Host. For the local/test
 * slice we derive from the request URL (localhost), which the env override supersedes when set.
 */
function rpConfig(url: URL): RpConfig {
	return {
		rpName: env.WEBAUTHN_RP_NAME ?? 'Boundless',
		rpID: env.WEBAUTHN_RP_ID ?? url.hostname,
		origin: env.WEBAUTHN_ORIGIN ?? url.origin,
	};
}

/** Build the deps for one request. Reads the current store singletons (so a dev reset is honored);
 *  selects the real KV challenge store when `platform.env.CHALLENGES` is bound (else in-memory). */
export function getWebAuthnDeps(url: URL, platform?: App.Platform): WebAuthnDeps {
	return { rp: rpConfig(url), clock, challenges: challengeStore(platform), invites, credentials };
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
	const verdict = evaluateInvite(await invites.load(token), clock.now());
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

export interface SeedInviteInput {
	readonly token: string;
	readonly adminId: string;
	readonly groupId: string;
	readonly expiresAt: number;
	readonly consumedAt?: number | null;
}

/** Seed a pending-admin invitation (dev/test only — the real invite is minted by T08's Worker). */
export function seedInvite(input: SeedInviteInput): void {
	invites.add(input.token, {
		adminId: input.adminId,
		groupId: input.groupId,
		expiresAt: input.expiresAt,
		consumedAt: input.consumedAt ?? null,
	});
}

/** Reset the interim in-memory backend (dev/test only) for per-test isolation. */
export function resetStores(): void {
	challenges = new MemoryChallengeStore(clock);
	invites = new MemoryInviteStore();
	credentials = new MemoryCredentialStore();
}
