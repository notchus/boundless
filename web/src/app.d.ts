// SvelteKit ambient types (spec 001 T15). `App.Locals` carries the request-resolved locale (set in
// hooks.server.ts) and, after a successful WebAuthn assertion, the signed-in admin id.

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
		// interface Platform {}
	}
}

export {};
