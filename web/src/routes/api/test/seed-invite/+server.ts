// DEV-ONLY test seam: seed a pending-admin invitation into the dev-durable (in-memory) invite store so the
// Playwright onboarding e2e can register against it. Hard-gated on `dev` (from $app/environment,
// compile-time `false` in any production build → 404) AND tree-shaken from the prod bundle — proven by
// tests/build-gates/no-dev-seams.test.ts (AC5). Kept under spec 009 T07 Option A (an invite-seed is unavoidable
// for the dev onboarding e2e; AC5 is the I11 guarantee). In prod the real invite is operator-seeded into
// Postgres and resolved via the Worker's B1 endpoint (ADR-0027) — never through this route.

import { dev } from '$app/environment';
import { error, json } from '@sveltejs/kit';

import { seedInvite, type SeedInviteInput } from '$lib/server/webauthn-deps';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
	if (!dev) error(404);

	const body = (await request.json()) as Partial<SeedInviteInput>;
	if (
		typeof body.token !== 'string' ||
		typeof body.adminId !== 'string' ||
		typeof body.groupId !== 'string' ||
		typeof body.expiresAt !== 'number'
	) {
		error(400, 'token, adminId, groupId, expiresAt required');
	}

	seedInvite({
		token: body.token,
		adminId: body.adminId,
		groupId: body.groupId,
		expiresAt: body.expiresAt,
		consumedAt: body.consumedAt ?? null,
	});
	return json({ ok: true });
};
