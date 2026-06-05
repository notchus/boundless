// DEV-ONLY test seam: seed a pending-admin invitation into the interim in-memory store so the
// Playwright real-ceremony test can register against it. Hard-gated on `dev` (from $app/environment,
// compile-time `false` in any production build → 404), and namespaced under /api/test/*. The real
// invite is minted by T08's Worker; this route + the in-memory backend are removed when the
// KV/Postgres adapters land (T15-shell, DEFERRED).

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
