// AC5 — the dev-only `/api/test/*` seams are unreachable in the production build (spec 009 T07, R21/I11).
//
// The seams are guarded by `if (!dev) error(404)` with `dev` from `$app/environment`. SvelteKit inlines `dev`
// from NODE_ENV: `NODE_ENV=production` (or unset) → `dev=false` → the guard collapses to an unconditional
// `error(404)`; but ANY non-production NODE_ENV (e.g. `test`, which vitest sets) → `dev=true` → the guard is
// tree-shaken AWAY and the seam ships LIVE — and `vite build --mode production` does NOT override this (only
// NODE_ENV does — both verified empirically). So the I11 guarantee must be PINNED at the build entry point:
// `web/package.json`'s build script is `NODE_ENV=production vite build`.
//
// This test proves that pin is load-bearing: it runs the REAL `pnpm build` under a HOSTILE ambient
// `NODE_ENV=test`, then asserts every `/api/test/*` handler's FIRST statement is an unconditional `error(404)`
// (no `$app/environment`/`dev` reference survives). If the build-script pin is ever removed, this build ships
// the live seams and the assertions fail — the regression is caught HERE, not in prod. (Vite does not
// physically delete the now-dead body after `error(404)` because it can't prove `error()` throws, so we assert
// DOMINANCE, not deletion. The live deployed-edge 404 probe is the T13 leg.)

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { beforeAll, describe, expect, it } from 'vitest';

const WEB_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER_OUT = join(WEB_ROOT, '.svelte-kit', 'output', 'server', 'entries', 'endpoints', 'api', 'test');

// Each seam → a tell-tale identifier proving its seed/authority logic is present in the file (so the test
// is non-vacuous) but dominated by the leading `error(404)`.
const SEAMS = [
	{ route: 'seed-invite', logic: 'seedInvite' },
	{ route: 'seed-session', logic: 'createSession' }, // mints an admin session — the highest-value seam
	{ route: 'seed-member', logic: 'seedMember' },
	{ route: 'reset', logic: 'resetStores' },
] as const;

/** The body of the compiled `POST = async (…) => { … }` handler (everything after the arrow's `{`). */
function handlerBody(src: string): string {
	const m = /POST\s*=\s*async\s*\([^)]*\)\s*=>\s*\{/.exec(src);
	if (!m) throw new Error('could not locate the compiled POST handler');
	return src.slice(m.index + m[0].length).trimStart();
}

describe('AC5 — no reachable /api/test/* seam in the production build', () => {
	beforeAll(() => {
		// Run the REAL deploy build command (`pnpm build` = `NODE_ENV=production vite build`) under a HOSTILE
		// ambient `NODE_ENV=test`. The script's inline pin must win — proving the seam-stripping is
		// NODE_ENV-independent at the build boundary (and catching a future un-pinning regression). CI runs
		// `pnpm test` BEFORE `pnpm build`, so we cannot reuse a pre-existing artifact.
		execFileSync('pnpm', ['run', 'build'], {
			cwd: WEB_ROOT,
			stdio: 'pipe',
			env: { ...process.env, NODE_ENV: 'test' },
		});
	}, 180_000);

	it.each(SEAMS)('$route 404s before any seam logic (dev inlined to false)', ({ route, logic }) => {
		const file = join(SERVER_OUT, route, '_server.ts.js');
		expect(existsSync(file), `${route} endpoint not found in the build`).toBe(true);
		const src = readFileSync(file, 'utf8');

		// Non-vacuity: the seam's seed/authority logic IS compiled into the file…
		expect(src).toContain(logic);
		// …but `dev` was inlined to `false` — no runtime `dev`/$app/environment read survives…
		expect(/\bdev\b/.test(src), `${route} still references a runtime 'dev' binding`).toBe(false);
		expect(src).not.toContain('$app/environment');
		// …so the handler's FIRST statement is the unconditional 404 (it dominates the dead body).
		expect(handlerBody(src).startsWith('error(404)'), `${route} handler does not 404 first`).toBe(true);
	});
});
