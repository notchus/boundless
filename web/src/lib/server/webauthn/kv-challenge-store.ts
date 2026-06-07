// Real Cloudflare KV implementation of the `ChallengeStore` port (ADR-0017 D3) — spec 001 T15-shell
// leg A. The production store for the one-time-use, ~5-minute-TTL admin-WebAuthn ceremony challenge.
//
// This replaces the interim in-memory stub (`MemoryChallengeStore`) whenever a Cloudflare KV binding
// is present: `@sveltejs/adapter-cloudflare` exposes it as `platform.env.CHALLENGES` (on the edge, and
// during `vite dev` via wrangler's `getPlatformProxy()` → a local Miniflare KV). The selection itself
// lives here too — `selectChallengeStore` (below) is the pure decision the composition root
// (`webauthn-deps.ts`) calls: real KV when bound, the in-memory fallback only in dev, else fail closed.
//
// CONSUME-ONCE is BEST-EFFORT, by design. KV has no atomic get-and-delete, so `take` reads then
// deletes; under a tight concurrent replay two reads could observe the same value before the delete
// propagates. That is acceptable here because consume-once is defence-in-depth, not the sole control:
// the ceremony key is also carried in a single short-lived httpOnly+Secure+SameSite=strict cookie, the
// `@simplewebauthn` verifier binds the signed assertion to this exact challenge (the unconditional
// control), and — for authenticators that maintain a non-zero signature counter — the sign-count check
// also rejects a replayed authenticator. KV's own TTL is the backstop. This matches the KV-based design
// decided in ADR-0017 D3.

import type { KVNamespace } from '@cloudflare/workers-types';

import type { ChallengeStore } from './ports';

/** Cloudflare KV's minimum `expirationTtl` (seconds). A smaller value is rejected by the KV API. */
const KV_MIN_EXPIRATION_TTL_SECS = 60;

/**
 * Clamp a requested TTL up to KV's 60-second minimum. The only caller passes `CHALLENGE_TTL_SECS`
 * (300), so this never bites in practice; the floor exists purely so the store can never throw a KV
 * range error on a hypothetical sub-60 TTL. (Clamping *up* only ever lengthens a challenge's life
 * slightly — consume-once still bounds it — so it cannot weaken the one-time-use property.)
 */
export function kvExpirationTtl(ttlSecs: number): number {
	return Math.max(ttlSecs, KV_MIN_EXPIRATION_TTL_SECS);
}

export class KvChallengeStore implements ChallengeStore {
	constructor(private readonly kv: KVNamespace) {}

	async put(key: string, challenge: string, ttlSecs: number): Promise<void> {
		await this.kv.put(key, challenge, { expirationTtl: kvExpirationTtl(ttlSecs) });
	}

	async take(key: string): Promise<string | null> {
		// `get` yields null when the key is absent or past its KV-native TTL. Delete makes it
		// consume-once (best-effort — see the file header); delete is idempotent if already gone.
		const challenge = await this.kv.get(key, 'text');
		await this.kv.delete(key);
		return challenge;
	}
}

/**
 * Pick the challenge store for a request. Returns the real Cloudflare **KV** store when a `CHALLENGES`
 * binding is present (the edge, and `vite dev`/Playwright via adapter-cloudflare's getPlatformProxy).
 *
 * When the binding is ABSENT it falls back to the per-isolate in-memory store ONLY when
 * `allowInMemoryFallback` is true (dev/test) — otherwise it **fails closed** by throwing. The in-memory
 * store is a per-isolate `Map`: on a real multi-isolate edge deploy a challenge stored by one isolate
 * could be looked up in another, silently breaking the one-time-use guarantee (ADR-0017 D3) and masking
 * a binding misconfiguration that should hard-fail. Callers pass `dev` ($app/environment) as the flag,
 * so a production build with no `CHALLENGES` binding refuses to serve rather than degrade quietly.
 *
 * Pure (no SvelteKit-virtual imports) so it is unit-testable under the bare Vitest config — the
 * imperative shell (`webauthn-deps.ts`) supplies the platform binding, the fallback, and `dev`.
 */
export function selectChallengeStore(
	kv: KVNamespace | undefined,
	fallback: ChallengeStore,
	allowInMemoryFallback: boolean,
): ChallengeStore {
	if (kv) return new KvChallengeStore(kv);
	if (allowInMemoryFallback) return fallback;
	throw new Error(
		'Admin-WebAuthn challenge store unavailable: the CHALLENGES KV namespace is not bound. ' +
			'Refusing the in-memory fallback outside dev — it cannot guarantee one-time-use WebAuthn ' +
			'challenges across edge isolates (ADR-0017 D3).',
	);
}
