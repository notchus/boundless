// Relying-Party config resolution for admin WebAuthn (spec 009 T09, AC11; ADR-0017). Extracted from the
// `webauthn-deps.ts` shell into this PURE module so it is unit-testable under bare Vitest and so the
// fail-closed policy is a tested decision, not an inline `??` chain.
//
// Production MUST pin `WEBAUTHN_RP_ID`/`WEBAUTHN_ORIGIN` via env (wrangler.toml `[vars]`, surfaced through
// `$env/dynamic/private` on the deployed Worker). The RP_ID is the domain passkeys are BOUND to — deriving
// it from the request `Host` is a WebAuthn footgun (a spoofed Host would let an attacker-chosen RP_ID bind
// credentials), so outside dev an unset RP config **fails closed** (throws) rather than trusting the URL.
// In dev/test we fall back to the request URL (localhost) so the local server + Playwright e2e work.

import type { RpConfig } from './config';

/** The env subset this resolver reads (a slice of `$env/dynamic/private`). Values may be undefined. */
export interface RpEnv {
	readonly WEBAUTHN_RP_NAME?: string;
	readonly WEBAUTHN_RP_ID?: string;
	readonly WEBAUTHN_ORIGIN?: string;
}

/**
 * Resolve the WebAuthn Relying-Party config. Env wins; in dev the request URL is the fallback; outside dev
 * an unset `WEBAUTHN_RP_ID`/`WEBAUTHN_ORIGIN` **throws** (never trust the request Host — ADR-0017). `rpName`
 * is the OS passkey-UI label (not a Boundless catalog string), so it carries a plain default.
 */
export function resolveRpConfig(env: RpEnv, url: URL, opts: { dev: boolean }): RpConfig {
	const rpID = env.WEBAUTHN_RP_ID ?? (opts.dev ? url.hostname : undefined);
	const origin = env.WEBAUTHN_ORIGIN ?? (opts.dev ? url.origin : undefined);
	if (rpID === undefined || origin === undefined) {
		throw new Error(
			'WebAuthn RP config unavailable: WEBAUTHN_RP_ID / WEBAUTHN_ORIGIN are not set. Refusing to derive ' +
				'the Relying-Party ID from the request Host outside dev — a spoofed Host would bind passkeys to an ' +
				'attacker-chosen RP_ID (ADR-0017). Pin them in wrangler.toml [vars] for the deploy.',
		);
	}
	return { rpName: env.WEBAUTHN_RP_NAME ?? 'Boundless', rpID, origin };
}
