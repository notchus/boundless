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

	// — Admin member-management (spec 008 T10) — Sarah's surface. The 17 keys in the spec §i18n
	//   table, plus the affordance/status/onboarding-status/audit copy the screens require (P8 — no
	//   hardcoded strings). Sentence case; no exclamation marks; periods only on full sentences; no
	//   "table"/"meeting" (glossary). The added (non-spec-table) keys are FLAGGED FOR OWNER REVIEW,
	//   like T11/T12/T15. `admin.member.code_expires` is ICU (`{when}` — a formatted server-time).
	// Nav / headings
	'admin.nav.skip': 'Skip to main content',
	'admin.nav.brand': 'Boundless',
	'admin.members.title': 'Members',
	'admin.members.audit_log': 'Audit log',
	'admin.member.detail_title': 'Member details',
	// List
	'admin.members.add': 'Add a member',
	'admin.members.search': 'Search members',
	'admin.members.filter_role': 'Filter by role',
	'admin.members.filter_status': 'Filter by status',
	'admin.members.filter_all': 'All',
	'admin.members.empty': 'No members yet.',
	'admin.member.status': 'Status',
	'admin.member.actions': 'Actions',
	'admin.member.actions_for': 'Actions for {name}',
	// Fields
	'admin.member.name': 'Name',
	'admin.member.phone': 'Phone',
	'admin.member.address': 'Address',
	'admin.member.role': 'Role',
	'admin.member.role_rider': 'Rider',
	'admin.member.role_driver': 'Driver',
	// A member may hold the Admin role too (multi-role, glossary); labelled correctly if a dual-role
	// member is viewed by id (the list excludes admins, but a deep-linked detail read does not).
	'admin.member.role_admin': 'Admin',
	// Actions
	'admin.member.save': 'Save',
	'admin.member.cancel': 'Cancel',
	'admin.member.view': 'View',
	'admin.member.edit': 'Edit',
	'admin.member.saving': 'Saving…',
	'admin.member.issued': 'Member added.',
	'admin.member.saved': 'Saved.',
	// Onboarding Code (shown once)
	'admin.member.onboarding_code': 'Onboarding code',
	'admin.member.code_expires': 'Code expires {when}',
	'admin.member.code_explainer': 'Read this to whoever is setting up the phone, or print it. It works once.',
	'admin.member.regenerate_code': 'Regenerate code',
	'admin.member.code_regenerated': 'A fresh code is ready.',
	// Onboarding status labels
	'admin.member.status_issued_not_onboarded': 'Not set up yet',
	'admin.member.status_onboarded': 'Set up',
	'admin.member.status_code_expired_or_lost': 'Code expired',
	'admin.member.status_needs_reonboarding': 'Needs setting up again',
	// Errors (voice register — honest, with a path forward)
	'admin.member.phone_invalid': "That number doesn't look right. Check and try again.",
	'admin.member.address_invalid': "That address doesn't look right. Check and try again.",
	'admin.member.roles_required': 'Choose at least one role.',
	'admin.member.duplicate_phone': 'That number is already in your group.',
	'admin.member.duplicate_view': 'View the member who has it',
	'admin.member.edit_stale': 'Someone else just changed this member. Refresh and try again.',
	'admin.member.not_found': 'That member is no longer here. Refresh the list.',
	'admin.member.group_key_missing': "Boundless isn't fully set up yet. Ask the developer.",
	'admin.member.error_generic': "That didn't work. Check the details and try again.",
	// Audit log (first-class — Sarah's privacy promise made checkable)
	'admin.audit.explainer':
		"Every time a member's details are opened, it's recorded here — who, when, and which fields. Field names only, never the values.",
	'admin.audit.when': 'When',
	'admin.audit.admin': 'Admin',
	'admin.audit.member': 'Member',
	'admin.audit.fields': 'Fields read',
	'admin.audit.request': 'Request',
	'admin.audit.empty': 'No reads logged yet.',
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
