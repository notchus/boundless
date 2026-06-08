// Dev-only test harness config (spec 001 T07-shell-B). `@cloudflare/vitest-pool-workers` (the
// vitest-4 line, 0.16.x) runs the Worker inside miniflare/workerd, reading the bindings (KV / Durable
// Objects / Queues / Hyperdrive) from wrangler.toml and emulating them in-process — NO Cloudflare
// account. The Rust→wasm Worker is pre-built by the `build` script (`worker-build --release`) before
// `vitest run` (the pool does not run wrangler's `[build]` command itself), and the pool loads
// `main = build/worker/shim.mjs`.
//
// Two test-only overrides via `miniflare` (these never touch the committed deploy config):
//   * `hyperdrives.HYPERDRIVE` — the LOCAL Postgres the emulated Hyperdrive Socket connects to. The
//     Worker connects as the non-superuser `boundless_app` role (provisioned by
//     scripts/setup-worker-test-db.sh) so the W2 least-privilege guard accepts it and RLS applies.
//     Per-environment via WORKER_TEST_PG (local default :55432; CI sets :5432).
//   * `bindings.HMAC_KEY` — a TEST-ONLY 32-byte (hex) per-instance key (I3). Injected here, NOT in
//     wrangler.toml, so no key is ever in the deploy config — at deploy HMAC_KEY is a `wrangler
//     secret`. (GROUP_ID is a non-secret committed default in wrangler.toml `[vars]`.) Cf. the
//     committed test key in server/store/tests/common/mod.rs.
import { cloudflareTest } from '@cloudflare/vitest-pool-workers';
import { defineConfig } from 'vitest/config';

const WORKER_TEST_PG =
	process.env.WORKER_TEST_PG ??
	'postgresql://boundless_app:boundless_app@localhost:55432/boundless_test';

// Obviously-a-test value (32 bytes of 0xAB as hex). Never deployed — deploy uses `wrangler secret`.
const TEST_HMAC_KEY_HEX = 'ab'.repeat(32);

export default defineConfig({
	plugins: [
		cloudflareTest({
			wrangler: { configPath: './wrangler.toml' },
			miniflare: {
				hyperdrives: { HYPERDRIVE: WORKER_TEST_PG },
				bindings: { HMAC_KEY: TEST_HMAC_KEY_HEX },
			},
		}),
	],
	// The tests import the golden wire fixtures from the repo root (`../../fixtures/auth/*.json`) to
	// assert contract conformance — allow Vite to read above the `server/` root.
	server: { fs: { allow: ['..'] } },
});
