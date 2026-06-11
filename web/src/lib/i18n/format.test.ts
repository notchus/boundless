// i18n runtime tests (spec 001 T15). Relative imports (the `$lib` alias is not wired into Vitest —
// matches the T09 webauthn unit tests).

import { describe, expect, it } from 'vitest';

import { en } from './catalog';
import { direction, resolveLocale, t } from './index';
import { PSEUDO_CLOSE, PSEUDO_OPEN, pseudoize } from './pseudo';

describe('t() — ICU message formatting', () => {
	it('resolves a plain catalog key in the source locale', () => {
		expect(t('admin.signin.title', 'en')).toBe('Sign in to Boundless.');
		expect(t('admin.onboarding.register_credential', 'en')).toBe(
			'Set up your security key or passkey.',
		);
	});

	it('returns a string for every shipped key (no rich-element arrays leaking through)', () => {
		// spec 008 (T10) introduced the first ICU-arg keys (e.g. `admin.member.code_expires` → `{when}`,
		// `admin.member.actions_for` → `{name}`); supply sample args so every key — plain or ICU —
		// formats to a plain non-empty string.
		const sampleArgs = { when: 'soon', name: 'Sarah', adminName: 'Sarah', count: 1 };
		for (const key of Object.keys(en) as (keyof typeof en)[]) {
			const out = t(key, 'en', sampleArgs);
			expect(typeof out).toBe('string');
			expect(out.length).toBeGreaterThan(0);
		}
	});

	it('falls back to the source-locale message for a locale with no catalog (e.g. RTL ar)', () => {
		// No `ar` catalog ships yet — the message falls back to `en` while the page still renders RTL.
		expect(t('admin.signin.title', 'ar')).toBe(en['admin.signin.title']);
	});

	it('renders the generated pseudo-locale (zz-ZZ) catalog (AC12 — T16)', () => {
		// T16 wires `catalogs['zz-ZZ'] = pseudoCatalog(en)`, so the tag now resolves to the
		// generated pseudo string (single-sourced from the same `pseudoize`), not the en fallback.
		const out = t('admin.signin.title', 'zz-ZZ');
		expect(out).toBe(pseudoize(en['admin.signin.title']));
		expect(out).not.toBe(en['admin.signin.title']);
		expect(out.startsWith(PSEUDO_OPEN) && out.endsWith(PSEUDO_CLOSE)).toBe(true);
	});
});

describe('direction()', () => {
	it('is ltr for en and rtl for ar/he', () => {
		expect(direction('en')).toBe('ltr');
		expect(direction('en-US')).toBe('ltr');
		expect(direction('ar')).toBe('rtl');
		expect(direction('he')).toBe('rtl');
		expect(direction('ar-EG')).toBe('rtl');
	});
});

describe('resolveLocale()', () => {
	const noCookies = { get: () => undefined } as unknown as import('@sveltejs/kit').Cookies;

	it('prefers ?locale= over cookie/default and accepts an RTL locale for testing', () => {
		const url = new URL('https://admin.example/admin/signin?locale=ar');
		expect(resolveLocale(url, noCookies)).toBe('ar');
	});

	it('falls back to the default for a malformed tag', () => {
		const url = new URL('https://admin.example/admin/signin?locale=not__a__locale');
		expect(resolveLocale(url, noCookies)).toBe('en');
	});

	it('uses the cookie when no query param is present', () => {
		const cookies = { get: (n: string) => (n === 'boundless_locale' ? 'ar' : undefined) } as unknown as import('@sveltejs/kit').Cookies;
		const url = new URL('https://admin.example/admin/signin');
		expect(resolveLocale(url, cookies)).toBe('ar');
	});
});
