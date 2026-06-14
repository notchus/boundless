// AC8 (web leg) — no raw `console` in web server/UI source (spec 009 T08, P2/I10). Every server log line
// MUST funnel through the scrubbed `emit()` sink in `$lib/server/log.ts` so secrets / PII / the invite
// token are redacted before they reach stdout/Logpush. A raw `console.*` anywhere else bypasses the
// scrubber. This is the structural backstop (the `production-invite-store-is-worker-backed.test.ts`
// precedent): scan `web/src/**` source (comments stripped, so doc lines that mention `console` don't
// false-positive), allow-listing only `log.ts` (the one sanctioned sink). Node env; reads its own tree.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const SRC = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'src');
const ALLOWED = join('lib', 'server', 'log.ts'); // the one sanctioned console sink

// Strip `/* */`, `<!-- -->`, and `//` line comments. The `(^|[^:])` guard on `//` preserves `://` inside
// code strings (e.g. a URL), so only a genuine line comment is removed.
function stripComments(src: string): string {
	return src
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.replace(/<!--[\s\S]*?-->/g, '')
		.split('\n')
		.map((line) => line.replace(/(^|[^:])\/\/.*$/, '$1'))
		.join('\n');
}

function webSourceFiles(): string[] {
	return readdirSync(SRC, { recursive: true })
		.map((e) => String(e))
		.filter((p) => (p.endsWith('.ts') || p.endsWith('.svelte')) && !p.endsWith('.test.ts'))
		.map((p) => join(SRC, p));
}

describe('AC8 — no raw console in web/src (all logging routes through the scrubbed emit() sink)', () => {
	it('only $lib/server/log.ts calls console; everything else uses emit()', () => {
		const files = webSourceFiles();
		// Non-vacuity: the scan found the allow-listed sink, and that sink really does call console
		// (so the allow-list is load-bearing, not a dead exception).
		const sink = files.find((f) => relative(SRC, f) === ALLOWED);
		expect(sink, 'log.ts (the sanctioned sink) must exist and be scanned').toBeTruthy();
		expect(/console\./.test(stripComments(readFileSync(sink!, 'utf8')))).toBe(true);

		const offenders: string[] = [];
		for (const file of files) {
			if (relative(SRC, file) === ALLOWED) continue;
			if (/console\./.test(stripComments(readFileSync(file, 'utf8')))) offenders.push(relative(SRC, file));
		}
		expect(
			offenders,
			`raw console.* bypasses the scrubbed emit() sink (P2/I10) — route these through $lib/server/log.ts:\n${offenders.join('\n')}`,
		).toEqual([]);
	});
});
