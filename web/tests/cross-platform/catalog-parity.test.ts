// AC12 (P8) — cross-platform onboarding catalog parity + pseudo-locale generation. Spec 001 T16.
//
// This is the genuinely cross-cutting i18n gate: no per-platform test sees all five catalogs at
// once, so none can catch *drift between them*. A key added to the iOS Rider catalog but forgotten
// on Android would silently fall back to English (or the raw key) on that platform in any non-`en`
// locale — exactly the "string didn't resolve from the catalog / truncates" failure AC12 forbids.
//
// It reads the real, shipped catalogs (iOS `.xcstrings` JSON, Android `strings.xml`, the web TS
// catalog) and asserts:
//   1. iOS-Rider ⟺ Android-Rider and iOS-Driver ⟺ Android-Driver key sets are identical
//      (dot↔underscore normalized) — the mobile twins never drift.
//   2. the web admin catalog carries the spec's named admin keys.
//   3. every key on every platform has English copy AND a valid pseudo-locale expansion
//      (`pseudo_locale_renders_all_onboarding_screens`, the data proof; the rendered proof is the
//      Playwright spec tests/e2e/pseudo-locale.spec.ts).

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { IntlMessageFormat } from 'intl-messageformat';
import { describe, expect, it } from 'vitest';

import { en as webCatalog } from '../../src/lib/i18n/catalog';
import { PSEUDO_CLOSE, PSEUDO_OPEN, pseudoize } from '../../src/lib/i18n/pseudo';

const repo = (rel: string): string => fileURLToPath(new URL(`../../../${rel}`, import.meta.url));

const IOS_RIDER = 'apple/BoundlessRider/Sources/RiderShared/Localization/Onboarding.xcstrings';
const IOS_DRIVER = 'apple/BoundlessDriver/Sources/DriverShared/Localization/DriverOnboarding.xcstrings';
const ANDROID_RIDER = 'android/rider/shared/src/main/res/values/strings.xml';
const ANDROID_DRIVER = 'android/driver/app/src/main/res/values/strings.xml';

/** Normalize a catalog key to the underscore form so iOS (`a.b_c`) and Android (`a_b_c`) compare. */
const norm = (key: string): string => key.replace(/\./g, '_');

/** iOS String Catalog → Map<normalizedKey, enValue>. */
function parseXcstrings(rel: string): Map<string, string> {
	const json = JSON.parse(readFileSync(repo(rel), 'utf8')) as {
		strings: Record<string, { localizations?: { en?: { stringUnit?: { value?: string } } } }>;
	};
	const out = new Map<string, string>();
	for (const [key, entry] of Object.entries(json.strings)) {
		const value = entry.localizations?.en?.stringUnit?.value;
		expect(value, `iOS key ${key} (${rel}) has no en value`).toBeTruthy();
		out.set(norm(key), value as string);
	}
	return out;
}

/** Android strings.xml → Map<key, value>, excluding the per-app `app_name`. */
function parseStringsXml(rel: string): Map<string, string> {
	const xml = readFileSync(repo(rel), 'utf8');
	const out = new Map<string, string>();
	const re = /<string\s+name="([^"]+)"\s*>([\s\S]*?)<\/string>/g;
	for (let m = re.exec(xml); m !== null; m = re.exec(xml)) {
		const [, key, value] = m;
		if (key === 'app_name') continue;
		out.set(key as string, (value as string).trim());
	}
	return out;
}

const sortedKeys = (m: Map<string, string>): string[] => [...m.keys()].sort();

describe('cross-platform onboarding catalog parity (AC12 / P8)', () => {
	it('iOS Rider ⟺ Android Rider key sets are identical', () => {
		const ios = parseXcstrings(IOS_RIDER);
		const android = parseStringsXml(ANDROID_RIDER);
		expect(ios.size).toBeGreaterThanOrEqual(20);
		expect(sortedKeys(android)).toEqual(sortedKeys(ios));
	});

	it('iOS Driver ⟺ Android Driver key sets are identical', () => {
		const ios = parseXcstrings(IOS_DRIVER);
		const android = parseStringsXml(ANDROID_DRIVER);
		expect(ios.size).toBeGreaterThanOrEqual(3);
		expect(sortedKeys(android)).toEqual(sortedKeys(ios));
	});

	it('the web admin catalog carries the spec-named admin keys', () => {
		for (const key of ['admin.onboarding.register_credential', 'admin.onboarding.invite_expired']) {
			expect(webCatalog, `web catalog missing ${key}`).toHaveProperty(key);
		}
	});
});

describe('pseudo-locale generation (AC12 / P8)', () => {
	// Every onboarding string on every surface, keyed by a label for clear failures.
	const all: Array<[string, Map<string, string>]> = [
		['ios-rider', parseXcstrings(IOS_RIDER)],
		['ios-driver', parseXcstrings(IOS_DRIVER)],
		['android-rider', parseStringsXml(ANDROID_RIDER)],
		['android-driver', parseStringsXml(ANDROID_DRIVER)],
		['web-admin', new Map(Object.entries(webCatalog as Record<string, string>))],
	];

	it('pseudoize preserves ICU placeholders so the message still parses + interpolates', () => {
		// No shipped catalog uses an ICU {arg} yet, so this exercises pseudoize's brace-balancing
		// (incl. a nested plural) directly — proving the ICU-preservation invariant is non-vacuous
		// before the first real {arg} (e.g. {adminName}) lands in the web catalog.
		const src = 'Hello {adminName}, {count, plural, one {# device} other {# devices}}.';
		const out = pseudoize(src);
		expect(out).toContain('{adminName}');
		expect(out).toContain('{count, plural, one {# device} other {# devices}}');
		const formatted = new IntlMessageFormat(out, 'en').format({ adminName: 'Sarah', count: 2 });
		const text = Array.isArray(formatted) ? formatted.join('') : String(formatted);
		expect(text).toContain('Sarah');
		expect(text).toContain('2 devices');
	});

	it('pseudo_locale_renders_all_onboarding_screens — every key expands, padded + bracketed, ICU-preserving', () => {
		for (const [platform, catalog] of all) {
			expect(catalog.size, `${platform} catalog is empty`).toBeGreaterThan(0);
			for (const [key, value] of catalog) {
				const pseudo = pseudoize(value);
				const where = `${platform}:${key}`;
				// Bracketed (so a hardcoded literal would stand out un-bracketed in QA).
				expect(pseudo.startsWith(PSEUDO_OPEN), `${where} not opened`).toBe(true);
				expect(pseudo.endsWith(PSEUDO_CLOSE), `${where} not closed`).toBe(true);
				// Padded longer than the source (so a too-tight layout truncates visibly).
				expect(pseudo.length, `${where} not expanded`).toBeGreaterThan(value.length);
				// ICU placeholders preserved verbatim (so `{adminName}`-style interpolation still works).
				for (const arg of value.match(/\{[^{}]+\}/g) ?? []) {
					expect(pseudo, `${where} dropped ICU arg ${arg}`).toContain(arg);
				}
			}
		}
	});
});
