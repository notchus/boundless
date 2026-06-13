// AC4b structural backstops (spec 009 T05):
//   (1) the PRODUCTION invite store is Worker-backed — `selectInviteStore` outside dev returns a
//       `WorkerInviteStore`, never the in-memory fake (the prod store reaches Postgres only via the
//       Worker, ADR-0027); and it fails closed when unconfigured (AC1).
//   (2) the edge-TS never re-implements the invite-token HMAC — that compare is core-only (the
//       ADR-0017 P4 carve-out). A structural lint asserts no `hmac`/`subtle` crypto appears anywhere in
//       `web/src/lib/server/webauthn/**` source (comments stripped, so the doc lines that *describe*
//       the carve-out don't false-positive). Node env; reads its own directory tree.

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { MemoryInviteStore } from './testing/memory-stores';
import { selectInviteStore, WorkerInviteStore, WorkerRegistrationHandshake } from './worker-stores';

const BASE = 'https://worker.example';
const SECRET = 'shh-shared-secret';

describe('AC4b — the production InviteStore is Worker-backed (never in-memory)', () => {
	const handshake = new WorkerRegistrationHandshake();

	it('selectInviteStore in prod (allow=false) returns a WorkerInviteStore, never a MemoryInviteStore', () => {
		const store = selectInviteStore(BASE, SECRET, new MemoryInviteStore(), false, handshake);
		expect(store).toBeInstanceOf(WorkerInviteStore);
		expect(store).not.toBeInstanceOf(MemoryInviteStore);
	});

	it('fails closed (throws) in prod when the Worker backend is unconfigured (AC1)', () => {
		expect(() => selectInviteStore(undefined, undefined, new MemoryInviteStore(), false, handshake)).toThrow(
			/Refusing the in-memory fallback/,
		);
	});
});

// — Strip `/* */` and `//` comments so the doc lines that DESCRIBE the carve-out ("...the HMAC compare
//   runs in the core...") are not mistaken for an implementation. The `(^|[^:])` guard preserves
//   `https://` inside code strings (only a genuine line comment is removed). —
function stripComments(src: string): string {
	return src
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.split('\n')
		.map((line) => line.replace(/(^|[^:])\/\/.*$/, '$1'))
		.join('\n');
}

function webauthnSourceFiles(): string[] {
	const dir = dirname(fileURLToPath(import.meta.url));
	return readdirSync(dir, { recursive: true })
		.map((e) => String(e))
		.filter((p) => p.endsWith('.ts') && !p.endsWith('.test.ts'))
		.map((p) => join(dir, p));
}

describe('AC4b — no invite-token HMAC in edge-TS (the carve-out is core-only, P4)', () => {
	// `hmac` catches `createHmac`/`HMAC`; `subtle` catches `crypto.subtle`/`subtle.sign|importKey`.
	const FORBIDDEN: { readonly label: string; readonly re: RegExp }[] = [
		{ label: 'hmac', re: /hmac/i },
		{ label: 'subtle', re: /subtle/i },
	];

	it('webauthn/** source contains no hmac/subtle crypto', () => {
		const files = webauthnSourceFiles();
		// Non-vacuity: the scan actually found the module (incl. worker-stores.ts) — a broken glob is a bug.
		expect(files.some((f) => f.endsWith('worker-stores.ts'))).toBe(true);

		const offenders: string[] = [];
		for (const file of files) {
			const code = stripComments(readFileSync(file, 'utf8'));
			for (const { label, re } of FORBIDDEN) {
				if (re.test(code)) offenders.push(`${file} → ${label}`);
			}
		}
		expect(offenders, `edge-TS must not implement the invite-token HMAC (it is core-only):\n${offenders.join('\n')}`).toEqual(
			[],
		);
	});
});
