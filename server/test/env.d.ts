// Ambient types for the `cloudflare:test` worker env — the bindings declared in wrangler.toml.
// (The Worker itself is Rust, so there is no generated Env type; these mirror the bindings used by
// the test.) Runtime types come from @cloudflare/vitest-pool-workers.
declare module 'cloudflare:test' {
	interface ProvidedEnv {
		MANIFEST: KVNamespace;
		ADMIN_ALERTS: Queue;
		GROUP_HUB: DurableObjectNamespace;
	}
}
