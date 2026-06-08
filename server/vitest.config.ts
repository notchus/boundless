// Dev-only test harness config (spec 001 T07-shell-B slice 1). `@cloudflare/vitest-pool-workers`
// (the vitest-4 line, 0.16.x) runs the Worker inside miniflare/workerd, reading the bindings (KV /
// Durable Objects / Queues) from wrangler.toml and emulating them in-process — NO Cloudflare
// account. The Rust→wasm Worker is pre-built by the `build` script (`worker-build --release --features
// scaffold`) before `vitest run` (the pool does not run wrangler's `[build]` command itself), and the
// pool loads
// `main = build/worker/shim.mjs`.
//
// Vitest 4 reworked pools: the old `test.poolOptions.workers` object is now the argument to the
// `cloudflareTest()` Vite plugin (per the package's v3→v4 codemod).
import { cloudflareTest } from '@cloudflare/vitest-pool-workers';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [cloudflareTest({ wrangler: { configPath: './wrangler.toml' } })],
	// The tests import the golden wire fixtures from the repo root (`../../fixtures/auth/*.json`) to
	// assert contract conformance — allow Vite to read above the `server/` root.
	server: { fs: { allow: ['..'] } },
});
