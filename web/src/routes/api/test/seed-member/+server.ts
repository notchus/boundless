// DEV-ONLY test seam: seed a member into the interim in-memory member backend so the Playwright member
// e2e can list/view/edit it without the Worker. Hard-gated on `dev` (compile-time `false` in any
// production build → 404), namespaced under /api/test/*. Real members are issued via the Worker; this
// route + the in-memory backend are removed when the live deployed BFF→Worker round-trip lands
// (DEFERRED.md → T10).

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
