// Admin onboarding message catalog (spec 001 T15; P8 — no user-visible string literals in views).
//
// English is the source-of-truth locale; other locales (Swiss German `gsw`, RTL `ar`/`he`, the
// pseudo-locale `zz-ZZ` for AC12) are added here or, in production, delivered via the signed
// Cloudflare KV manifest (ADR-0014, O2) — the shell. `t()` falls back to the source locale per key,
// so the layout/a11y bar (incl. RTL via `dir`) can be tested before translations land.
//
// Keys & copy follow docs/voice-and-tone.md: sentence case; no exclamation marks; periods only on
// full sentences (imperative instructions), not on UI labels. Two keys are the spec's i18n table
// (`admin.onboarding.register_credential`, `admin.onboarding.invite_expired`); the rest are the
// affordance/status/success copy the screens require — FLAGGED FOR OWNER REVIEW (like T11/T12).

import { pseudoCatalog } from './pseudo';

export const en = {
	// — Registration landing (/admin/onboard/[token]) —
	/** Spec i18n table. The registration prompt + page heading. */
	'admin.onboarding.register_credential': 'Set up your security key or passkey.',
	/** Spec i18n table. The InviteExpired terminal screen. */
	'admin.onboarding.invite_expired': 'This invitation has expired. Ask the developer for a new one.',
	'admin.onboarding.register_explainer':
		'Boundless uses a passkey or security key instead of a password.',
	'admin.onboarding.register_action': 'Set up your key',
	'admin.onboarding.registering': 'Setting up your key…',
	'admin.onboarding.registered': 'Your key is set up.',
	'admin.onboarding.go_to_signin': 'Go to sign in',

	// — Sign in (/admin/signin) — assertion only, no password (AC2) —
	'admin.signin.title': 'Sign in to Boundless.',
	'admin.signin.explainer': 'Use the passkey or security key you set up.',
	'admin.signin.action': 'Sign in with your key',
	'admin.signin.signing_in': 'Signing in…',
	'admin.signin.failed': "That didn't work. Try again, or ask the developer for help.",

	// — Signed-in placeholder (/admin) — the admin dashboard itself is spec 008 —
	'admin.home.signed_in': "You're signed in.",
} as const;

/** Every catalog key. Derived from the source locale so missing/extra keys are a compile error. */
export type MessageKey = keyof typeof en;

/** A complete per-locale catalog (a translation supplies every key). */
export type Catalog = Record<MessageKey, string>;

/** The source-of-truth locale; `t()` falls back to it for any missing key/locale. */
export const SOURCE_LOCALE = 'en';

/** The pseudo-locale (AC12 / P8). Generated from `en`, not translated — see `./pseudo.ts`. */
export const PSEUDO_LOCALE = 'zz-ZZ';

/**
 * Loaded catalogs. `en` is the source of truth; `zz-ZZ` is the generated pseudo-locale (AC12) —
 * opt-in via `?locale=zz-ZZ`, so production copy is unaffected. Real translations arrive via the
 * signed KV manifest (ADR-0014). Partial: a locale may omit keys (→ source fallback in `t()`).
 */
export const catalogs: Record<string, Partial<Catalog>> = {
	en,
	[PSEUDO_LOCALE]: pseudoCatalog(en),
};
