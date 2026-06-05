// POST /api/admin/auth/register — verify a WebAuthn registration response, consume the invite
// (single-use, AC16), revoke prior credentials (recovery; ADR-0016 D4), store the credential (AC20).
//
// Matches the frozen contract (api/openapi.yaml `adminAuthRegister`): success → 200 `AdminAuthResult`
// ({ admin_id }); any failure (verification / UV missing / invite consumed-or-expired) → 400
// `ErrorBody` ({ error_code }). Registration does NOT sign the admin in — they then sign in (the
// state machine: Active → sign-in). Body: { token, response }.

import { json } from '@sveltejs/kit';
import type { RegistrationResponseJSON } from '@simplewebauthn/server';

import { verifyRegistration, WebAuthnError } from '$lib/server/webauthn';
import { CEREMONY_COOKIE, getWebAuthnDeps } from '$lib/server/webauthn-deps';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, url, cookies }) => {
	const deps = getWebAuthnDeps(url);
	const ceremonyKey = cookies.get(CEREMONY_COOKIE);
	if (ceremonyKey === undefined) {
		return json({ error_code: 'ADMIN_WEBAUTHN_CHALLENGE_EXPIRED' }, { status: 400 });
	}

	const body = (await request.json()) as { token?: unknown; response?: unknown };
	if (typeof body.token !== 'string' || typeof body.response !== 'object' || body.response === null) {
		return json({ error_code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' }, { status: 400 });
	}

	try {
		const outcome = await verifyRegistration(deps, {
			ceremonyKey,
			presentedToken: body.token,
			response: body.response as RegistrationResponseJSON,
		});
		cookies.delete(CEREMONY_COOKIE, { path: '/' });
		return json({ admin_id: outcome.adminId });
	} catch (e) {
		cookies.delete(CEREMONY_COOKIE, { path: '/' });
		if (e instanceof WebAuthnError) {
			return json({ error_code: e.code }, { status: 400 });
		}
		throw e;
	}
};
