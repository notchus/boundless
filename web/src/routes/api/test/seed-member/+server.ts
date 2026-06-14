// DEV-ONLY test seam: seed a member into the dev-durable (in-memory) member backend so the Playwright
// member e2e can list/view/edit it without the Worker. Hard-gated on `dev` (compile-time `false` in any
// production build → 404) AND tree-shaken from the prod bundle — proven by tests/build-gates/no-dev-seams.test.ts
// (AC5). Kept under spec 009 T07 Option A. In prod, real members are issued/read via the Worker BFF
// (ADR-0026) — never through this route.

import { dev } from '$app/environment';
import { error, json } from '@sveltejs/kit';

import type { SeedMemberInput } from '$lib/server/members';
import { seedMember } from '$lib/server/members-deps';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
	if (!dev) error(404);

	const body = (await request.json()) as Partial<SeedMemberInput>;
	if (typeof body.name !== 'string' || typeof body.phone !== 'string' || typeof body.address !== 'string') {
		error(400, 'name, phone, address required');
	}

	const summary = seedMember({
		member_id: body.member_id,
		name: body.name,
		phone: body.phone,
		address: body.address,
		roles: body.roles,
		onboarding_status: body.onboarding_status,
	});
	return json({ ok: true, member_id: summary.member_id });
};
