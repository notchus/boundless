// AC15 — binding/type drift gate (spec 009 T09, D6). Adding or renaming a Cloudflare *binding* in
// `wrangler.toml` without updating the `App.Platform.env` declaration in `src/app.d.ts` must fail the
// build. We run `wrangler types` (the source of truth for what `wrangler.toml` declares) and assert the
// set of *bindings* it derives equals the set declared in `app.d.ts`.
//
// Why not commit the generated `App.Platform` (the literal D6)? `wrangler types`' full output bundles the
// 14k-line workerd runtime types, whose global `KVNamespace` collides with the pinned
// `@cloudflare/workers-types` the project uses via `import type` (deliberately, to avoid global
// pollution); and the env-only output needs a *global* `KVNamespace` the project also avoids. So we keep
// the hand-typed `app.d.ts` and enforce AC15's INTENT with this drift check instead. `[vars]` (ADMIN_WORKER_BASE,
// WEBAUTHN_*) are typed as string-literals by wrangler and are read via `$env/dynamic/private`, NOT
// `platform.env` — so they are deliberately excluded here (only resource bindings are compared).

import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, describe, expect, it } from 'vitest';

const WEB_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const WRANGLER_BIN = join(WEB_ROOT, 'node_modules', '.bin', 'wrangler');
const tmp = mkdtempSync(join(tmpdir(), 'bndl-wrangler-types-'));

afterAll(() => rmSync(tmp, { recursive: true, force: true }));

/** Generate env-only types from wrangler.toml and return the `__BaseEnv_Env` body. */
function generatedEnvInterface(): string {
	const out = join(tmp, 'worker-configuration.d.ts');
	execFileSync(WRANGLER_BIN, ['types', out, '--include-runtime=false'], {
		cwd: WEB_ROOT,
		encoding: 'utf8',
		stdio: 'pipe',
	});
	const body = /interface __BaseEnv_Env \{([\s\S]*?)\}/.exec(readFileSync(out, 'utf8'));
	if (!body || body[1] === undefined) throw new Error('could not find __BaseEnv_Env in wrangler types output');
	return body[1];
}

/** A property is a BINDING (KVNamespace / DurableObjectNamespace / R2Bucket / …) iff its type is a bare
 *  identifier — NOT a `"string literal"` (a `[var]`) and NOT `string`. */
function bindingNames(envInterfaceBody: string): Set<string> {
	const names = new Set<string>();
	for (const line of envInterfaceBody.split('\n')) {
		const m = /^\s*(\w+)\??:\s*(.+?);?\s*$/.exec(line);
		const name = m?.[1];
		const type = m?.[2];
		if (name === undefined || type === undefined) continue;
		if (/^["']/.test(type) || type === 'string') continue; // a [var], not a binding
		names.add(name);
	}
	return names;
}

/** The binding keys declared in `App.Platform.env { … }` in src/app.d.ts. */
function appPlatformEnvKeys(): Set<string> {
	const src = readFileSync(join(WEB_ROOT, 'src', 'app.d.ts'), 'utf8');
	const block = /env\?:\s*\{([\s\S]*?)\};/.exec(src);
	const body = block?.[1];
	if (body === undefined) throw new Error('could not find App.Platform.env { … } in src/app.d.ts');
	const keys = new Set<string>();
	for (const m of body.matchAll(/(\w+)\??:/g)) if (m[1] !== undefined) keys.add(m[1]);
	return keys;
}

describe('AC15 — wrangler.toml bindings match the App.Platform.env types', () => {
	it('every wrangler.toml binding is declared in app.d.ts and vice-versa (no drift)', () => {
		const fromWrangler = bindingNames(generatedEnvInterface());
		const fromAppDts = appPlatformEnvKeys();

		// Non-vacuity: the scan actually found the two KV bindings (a broken regex/glob is a bug).
		expect(fromWrangler.has('CHALLENGES')).toBe(true);
		expect(fromWrangler.has('ADMIN_SESSIONS')).toBe(true);
		expect(fromAppDts.size).toBeGreaterThan(0);

		expect([...fromAppDts].sort(), 'app.d.ts App.Platform.env bindings drifted from wrangler.toml').toEqual(
			[...fromWrangler].sort(),
		);
	});
});
