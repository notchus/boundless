// Ambient types for the `cloudflare:test` worker env — the bindings declared in wrangler.toml.
// (The Worker itself is Rust, so there is no generated Env type; these mirror the bindings used by
// the test.) Runtime types come from @cloudflare/vitest-pool-workers.
declare module 'cloudflare:test' {
	interface ProvidedEnv {
		MANIFEST: KVNamespace;
		ADMIN_ALERTS: Queue;
		GROUP_HUB: DurableObjectNamespace;
		// Hyperdrive → Postgres (emulated by miniflare against a local PG; see vitest.config.ts) +
		// the auth config the Worker reads via env.var. Not exercised by the test directly (the Worker
		// uses them), declared for completeness of the bindings the worker env carries.
		HYPERDRIVE: Hyperdrive;
		HMAC_KEY: string;
		GROUP_ID: string;
	}
}
