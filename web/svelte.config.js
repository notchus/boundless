// SvelteKit config (spec 001 T15 / T15-shell leg A). The admin onboarding web app.
//
// Adapter: `@sveltejs/adapter-cloudflare` — the real production target (Cloudflare Workers). Crucially,
// it makes the Cloudflare bindings real LOCALLY with no account: during `vite dev` the adapter calls
// wrangler's `getPlatformProxy()`, so `platform.env.CHALLENGES` is a live Miniflare KV (read from
// wrangler.toml). That is what lets the admin-WebAuthn challenge store run on real KV in dev + the
// Playwright e2e (ADR-0017 D3), not just the in-memory stub. `pnpm build` builds with no account
// (build ≠ deploy); the actual `wrangler deploy` + the Hyperdrive binding for the Postgres
// invite/credential stores are T15-shell leg B (DEFERRED.md). See web/wrangler.toml.

import { preprocessMeltUI, sequence } from '@melt-ui/pp';
import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // `preprocessMeltUI` rewrites melt-ui's `use:melt={$el}` actions into Svelte attachments (spec 008
  // T10 — the admin member dialogs/menus); it must run AFTER `vitePreprocess`.
  preprocess: sequence([vitePreprocess(), preprocessMeltUI()]),
  kit: {
    adapter: adapter(),
  },
};

export default config;
