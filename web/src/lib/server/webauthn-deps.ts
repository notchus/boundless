// Composition root for the admin-WebAuthn routes (spec 001 T15) — builds the `WebAuthnDeps` the T09
// verification functions consume.
//
// FUNCTIONAL-CORE / IMPERATIVE-SHELL: the verification LOGIC (T09, `$lib/server/webauthn`) is
// unchanged; this file only wires its ports. For the buildable + Playwright-testable slice the ports
// are the **same in-memory implementations** the T09 unit tests use (reused, not duplicated) and the
// RP config is derived from the request URL. The real Cloudflare **KV** challenge store + **Postgres**
// invite/credential stores (via the Worker, incl. the invite-token HMAC routed through the core per
// ADR-0017's P4 carve-out) are the **T15-shell** (DEFERRED.md). This mirrors the Rust Worker composing
// an in-memory `DeviceStore` until token encryption lands. There is no production deploy yet (no
// wrangler), so this interim in-memory backend cannot accidentally serve real traffic.

import { env } from '$env/dynamic/private';
import type { Cookies } from '@sveltejs/kit';
import type { PublicKeyCredentialCreationOptionsJSON } from '@simplewebauthn/server';

import { buildRegistrationOptions, evaluateInvite, WebAuthnError } from '$lib/server/webauthn';
import type { Clock, RpConfig, WebAuthnDeps } from '$lib/server/webauthn';
import {
	MemoryChallengeStore,
	MemoryCredentialStore,
	MemoryInviteStore,
} from '$lib/server/webauthn/testing/memory-stores';

/** Real server clock (unix seconds) — the only ambient input; everything else is injected. */
const clock: Clock = { now: () => Math.floor(Date.now() / 1000) };

// Interim in-memory backend (see header). `let` so the dev-only `/api/test/reset` seam can swap them.
let challenges = new MemoryChallengeStore(clock);
let invites = new MemoryInviteStore();
let credentials = new MemoryCredentialStore();

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

/** Build the deps for one request. Reads the current store singletons (so a dev reset is honored). */
export function getWebAuthnDeps(url: URL): WebAuthnDeps {
	return { rp: rpConfig(url), clock, challenges, invites, credentials };
}

// — Ceremony challenge key (the KV-challenge key in production) round-tripped via a short-lived
//   httpOnly cookie so the verify call can retrieve the challenge stored by the options call. —

export const CEREMONY_COOKIE = 'boundless_webauthn_ceremony';
export const CEREMONY_COOKIE_OPTIONS: Parameters<import('@sveltejs/kit').Cookies['set']>[2] = {
	httpOnly: true,
	secure: true,
	sameSite: 'strict',
	path: '/',
	maxAge: 300, // matches CHALLENGE_TTL_SECS
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
): Promise<InviteCeremony> {
	const deps = getWebAuthnDeps(url);
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
