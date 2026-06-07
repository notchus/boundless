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

// The script shells out and scans every lock file (~6s wall as the dep tree grew across T15/T16),
// which exceeds vitest's 5s default and flakes under load. Give it a generous fixed timeout — this is
// a synchronous gate, not a perf test, so a wide margin is correct (the script's own exit code, not
// wall-clock, is what AC13 asserts).
test(
	'ac13_onboarding_adds_no_third_party — network allow-list passes across all lock files (I8)',
	() => {
		const output = execFileSync('bash', ['scripts/check-network-allowlist.sh'], {
			cwd: repoRoot,
			encoding: 'utf8',
		});
		expect(output).toContain('no forbidden trackers');
	},
	30_000,
);
