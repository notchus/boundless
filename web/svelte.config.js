// SvelteKit config (spec 001 T15 / T15-shell leg A). The admin onboarding web app.
//
// Adapter: `@sveltejs/adapter-cloudflare` — the real production target (Cloudflare Workers). Crucially,
// it makes the Cloudflare bindings real LOCALLY with no account: during `vite dev` the adapter calls
// wrangler's `getPlatformProxy()`, so `platform.env.CHALLENGES` is a live Miniflare KV (read from
// wrangler.toml). That is what lets the admin-WebAuthn challenge store run on real KV in dev + the
// Playwright e2e (ADR-0017 D3), not just the in-memory stub. `pnpm build` builds with no account
// (build ≠ deploy); the actual `wrangler deploy` + the Hyperdrive binding for the Postgres
// invite/credential stores are T15-shell leg B (DEFERRED.md). See web/wrangler.toml.

import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
  },
};

export default config;
