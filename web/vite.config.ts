// Vite config for the SvelteKit admin web (spec 001 T15).
//
// Tailwind v4 integrates via `@tailwindcss/vite` (no PostCSS config in v4); it must come before
// `sveltekit()`. Vitest uses the separate `vitest.config.ts` (node env, pure-TS tests), so this
// file is for `vite dev` / `vite build` / `vite preview` only.

import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
});
