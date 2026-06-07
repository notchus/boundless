// Miniflare integration tests for the boundless-worker skeleton (spec 001 T07-shell-B slice 1).
//
// Runs the real Rust→wasm Worker in workerd via @cloudflare/vitest-pool-workers — KV / Durable
// Objects / Queues all emulated in-process, NO Cloudflare account. Proves the plumbing the slice
// stands up: the Router + version handshake (AC7/O4), the real core sign-in over the scaffold store
// (matched / not-on-file / below-min wire shapes, no existence leak), the KV binding, the Queue
// binding (the below-min alert fanout, §10-E), and the GroupHub Durable Object's state.storage()
// round-trip + §10-E rate-limit window (AC17).

import { env, SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';

// Replay the GOLDEN wire fixtures (the same files the platform clients are built against) — so a
// drift between the Worker's wire shape and the frozen contract fails CI, not a hand-copied literal.
import belowMinVersion from '../../fixtures/auth/below_min_version.json';
import phoneNotOnFile from '../../fixtures/auth/phone_not_on_file.json';
import signinOk from '../../fixtures/auth/signin_ok.json';

const BASE = 'https://worker.example';

async function signin(phone: string, app_version: string, platform = 'ios') {
	const res = await SELF.fetch(`${BASE}/api/auth/signin`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ phone, reported: { platform, app_version } }),
	});
	return { res, body: (await res.json()) as Record<string, unknown> };
}

describe('boundless-worker skeleton (T07-shell-B slice 1)', () => {
	it('GET /healthz carries the version handshake on every response (AC7/O4)', async () => {
		const res = await SELF.fetch(`${BASE}/healthz`);
		expect(res.status).toBe(200);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.status).toBe('ok');
		expect(body.client_min_version).toBe('1.0.0');
		expect(body.client_recommended_version).toBe('1.2.0');
	});

	it('KV MANIFEST binding round-trips and /healthz reflects it', async () => {
		// Seed via the binding directly (the pool exposes the worker env), then read through the route.
		await env.MANIFEST.put('manifest:v1:index', '{"v":1}');
		expect(await env.MANIFEST.get('manifest:v1:index')).toBe('{"v":1}');
		const res = await SELF.fetch(`${BASE}/healthz`);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.manifest_present).toBe(true);
	});

	it('sign-in → member_matched for the seeded phone, matching the golden fixture byte-for-byte', async () => {
		const { res, body } = await signin('+15551230000', '1.2.0');
		expect(res.status).toBe(200);
		expect(body).toEqual(signinOk);
	});

	it('sign-in → phone_not_on_file for an unknown phone (no existence leak), matching the fixture', async () => {
		const { res, body } = await signin('+15559999999', '1.2.0');
		expect(res.status).toBe(200);
		expect(body).toEqual(phoneNotOnFile);
	});

	it('sign-in below client_min_version degrades + fans out one admin alert to the Queue (AC8/§10-E)', async () => {
		// A below-min handshake collapses to BelowMinVersion regardless of phone; the Queue send must
		// succeed (a missing ADMIN_ALERTS binding would 500), proving the alert-fanout binding. The
		// fixture's reported_client_version is {ios, 0.9.0}, so drive exactly that.
		const { res, body } = await signin('+15551230000', '0.9.0');
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
