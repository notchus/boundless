// Miniflare integration tests for the boundless-worker (spec 001 T07-shell-B, PgAuthStore slice).
//
// Runs the real Rust→wasm Worker in workerd via @cloudflare/vitest-pool-workers, with KV / Durable
// Objects / Queues AND a real **Hyperdrive → Postgres** binding all emulated in-process — NO
// Cloudflare account. The Hyperdrive Socket connects to the local Postgres provisioned by
// scripts/setup-worker-test-db.sh (the non-superuser `boundless_app` role; see vitest.config.ts).
//
// What this proves that slice 1 could not: the **transport** — `connect_raw` over the wasm
// `worker::Socket` + the `spawn_local` connection driver + the W2 least-privilege guard + a real
// query, end-to-end inside workerd — and that the real `AuthService::sign_in` runs over the real
// `PgAuthStore` (RLS-scoped). Business-logic correctness over real Postgres (member_matched,
// onboarding consume, rotate-vs-replay, …) is proven natively in server/store/tests/service_pg.rs;
// here the DB is UNSEEDED, so sign-in returns phone_not_on_file through the real store/transport.

import { env, SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';

// Replay the GOLDEN wire fixtures (the same files the platform clients are built against) — so a
// drift between the Worker's wire shape and the frozen contract fails CI, not a hand-copied literal.
import belowMinVersion from '../../fixtures/auth/below_min_version.json';
import phoneNotOnFile from '../../fixtures/auth/phone_not_on_file.json';

const BASE = 'https://worker.example';

async function signin(phone: string, app_version: string, platform = 'ios') {
	const res = await SELF.fetch(`${BASE}/api/auth/signin`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ phone, reported: { platform, app_version } }),
	});
	return { res, body: (await res.json()) as Record<string, unknown> };
}

describe('boundless-worker (T07-shell-B, PgAuthStore over Hyperdrive)', () => {
	it('GET /healthz carries the version handshake on every response (AC7/O4)', async () => {
		const res = await SELF.fetch(`${BASE}/healthz`);
		expect(res.status).toBe(200);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.status).toBe('ok');
		expect(body.client_min_version).toBe('1.0.0');
		expect(body.client_recommended_version).toBe('1.2.0');
	});

	it('GET /readyz probes Postgres over Hyperdrive → db:ok (connect_raw + spawn_local + W2 guard)', async () => {
		// "ok" means: the Worker got a `worker::Socket` from the Hyperdrive binding, drove a
		// tokio-postgres `connect_raw` connection via spawn_local, and the W2 least-privilege guard
		// ran a real query and accepted the non-superuser role. The whole transport, inside workerd.
		// (Liveness `/healthz` is dependency-free; the DB probe is on readiness `/readyz`.)
		const res = await SELF.fetch(`${BASE}/readyz`);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.db).toBe('ok');
	});

	it('KV MANIFEST binding round-trips and /healthz reflects it', async () => {
		// Seed via the binding directly (the pool exposes the worker env), then read through the route.
		await env.MANIFEST.put('manifest:v1:index', '{"v":1}');
		expect(await env.MANIFEST.get('manifest:v1:index')).toBe('{"v":1}');
		const res = await SELF.fetch(`${BASE}/healthz`);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.manifest_present).toBe(true);
	});

	it('sign-in → phone_not_on_file via the real PgAuthStore over Hyperdrive (empty db, no existence leak)', async () => {
		// The real AuthService::sign_in runs find_member_by_phone in an RLS-scoped transaction over the
		// real Postgres; the db is unseeded so any phone misses, matching the golden fixture exactly.
		const { res, body } = await signin('+15559999999', '1.2.0');
		expect(res.status).toBe(200);
		expect(body).toEqual(phoneNotOnFile);
	});

	it('sign-in below client_min_version degrades + fans out one admin alert to the Queue (AC8/§10-E)', async () => {
		// A below-min handshake collapses to BelowMinVersion regardless of phone; the Queue send must
		// succeed (a missing ADMIN_ALERTS binding would 500), proving the alert-fanout binding. The
		// fixture's reported_client_version is {ios, 0.9.0}, so drive exactly that.
		const { res, body } = await signin('+15555550100', '0.9.0');
		expect(res.status).toBe(200);
		expect(body).toEqual(belowMinVersion);
	});

	it('GroupHub DO persists via state.storage() and locks after the §10-E window (AC17)', async () => {
		// Server time is read inside the DO (never client-supplied), so the 6 rapid calls all fall in
		// one real 15-minute window bucket.
		const member = '00000000-0000-0000-0000-000000000042';
		let last: Record<string, unknown> = {};
		for (let i = 0; i < 6; i++) {
			const res = await SELF.fetch(`${BASE}/api/auth/bind-device`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ member }),
			});
			expect(res.status).toBe(200);
			last = (await res.json()) as Record<string, unknown>;
			// The durable "served" counter increments across requests → state.storage() round-trips.
			expect(last.served).toBe(i + 1);
		}
		// prior attempts 0..4 are allowed; the 6th call sees prior=5 → locked (5 per 15 min, AC17).
		expect(last.locked).toBe(true);
		expect(last.error_code).toBe('AUTH_ONBOARDING_CODE_RATE_LIMITED');
	});
});
