// SvelteKit config (spec 001 T15). The admin onboarding web app.
//
// Adapter: `@sveltejs/adapter-node` is used for the *buildable + locally-testable* slice — it
// always builds a previewable server with no platform detection and no `wrangler`, so Playwright's
// `webServer` (and CI) can run the real app. The production target is Cloudflare; swapping in
// `@sveltejs/adapter-cloudflare` + the `wrangler` deploy is the T15-shell (DEFERRED.md), mirroring
// the deferred Rust Worker runtime (T07-shell-B). Playwright drives the app via `vite dev`, which is
// adapter-independent, so this choice does not affect the tests.

import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
  },
};

export default config;
