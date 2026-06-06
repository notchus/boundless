// AC13 (I8) — the named test wrapper for the network allow-list / no-tracker gate. Spec 001 T16.
//
// The live gate is `scripts/check-network-allowlist.sh` (scans every lock file for forbidden
// analytics/tracker/crash SDKs) — run in the CI `network-allowlist` job and the pre-push hook.
// This test invokes the same script so AC13 has a named, traceable test in a suite (the spec's
// `ac13_onboarding_adds_no_third_party`). `execFileSync` throws on a non-zero exit, so a tracker
// creeping into any lock fails the test; we also assert the success line to guard against the
// script silently short-circuiting.

import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';

const repoRoot = fileURLToPath(new URL('../../../', import.meta.url));

test('ac13_onboarding_adds_no_third_party — network allow-list passes across all lock files (I8)', () => {
	const output = execFileSync('bash', ['scripts/check-network-allowlist.sh'], {
		cwd: repoRoot,
		encoding: 'utf8',
	});
	expect(output).toContain('no forbidden trackers');
});
