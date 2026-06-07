// SvelteKit ambient types (spec 001 T15). `App.Locals` carries the request-resolved locale (set in
// hooks.server.ts) and, after a successful WebAuthn assertion, the signed-in admin id. `App.Platform`
// types the Cloudflare bindings exposed by `@sveltejs/adapter-cloudflare` (T15-shell leg A) — currently
// just the WebAuthn challenge KV namespace; the Hyperdrive binding for the Postgres invite/credential
// stores lands with T15-shell leg B (DEFERRED.md). `import type` keeps the KV type local to this module
// (no global workers-types pollution); `declare global` augments the SvelteKit `App` namespace.

import type { KVNamespace } from '@cloudflare/workers-types';

declare global {
	namespace App {
		interface Locals {
			/** BCP-47 locale resolved per request (hooks.server.ts). */
			locale: string;
			/** Set after a verified admin sign-in assertion (post-assertion session). */
			adminId?: string;
		}
		// interface Error {}
		// interface PageData {}
		// interface PageState {}
		interface Platform {
			/** Cloudflare bindings (adapter-cloudflare). `CHALLENGES`: one-time-use WebAuthn challenge
			 *  KV (ADR-0017 D3); absent under an adapterless/test run, so callers treat it as optional. */
			env?: {
				CHALLENGES?: KVNamespace;
			};
		}
	}
}

export {};
