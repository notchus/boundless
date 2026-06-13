// Admin sign-in — WebAuthn assertion only, no password (AC2).
//   GET  /api/admin/auth/signin  → assertion options ({ publicKey }) + ceremony cookie. (Options
//        retrieval is a shell detail the frozen contract leaves to T15; the verify POST is contracted.)
//   POST /api/admin/auth/signin  → verify the assertion. Matches the frozen contract
//        (`adminAuthSignIn`): success → 200 `AdminAuthResult` ({ admin_id }) + the §10-F session
//        cookie; failure (verification / UV missing / unknown credential) → 400 `ErrorBody`
//        ({ error_code }). Body (POST): { response }.

import { json } from '@sveltejs/kit';
import type { AuthenticationResponseJSON } from '@simplewebauthn/server';

import { buildAuthenticationOptions, verifyAuthentication, WebAuthnError } from '$lib/server/webauthn';
import { ADMIN_SESSION_COOKIE, createSession, SESSION_COOKIE_OPTIONS } from '$lib/server/session';
import { CEREMONY_COOKIE, CEREMONY_COOKIE_OPTIONS, getWebAuthnDeps, newCeremonyKey } from '$lib/server/webauthn-deps';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url, cookies, platform }) => {
	const deps = getWebAuthnDeps(url, platform);
	const ceremonyKey = newCeremonyKey();
	const options = await buildAuthenticationOptions(deps, { ceremonyKey });
	cookies.set(CEREMONY_COOKIE, ceremonyKey, CEREMONY_COOKIE_OPTIONS);
	return json({ publicKey: options });
};

export const POST: RequestHandler = async ({ request, url, cookies, platform }) => {
	const deps = getWebAuthnDeps(url, platform);
	const ceremonyKey = cookies.get(CEREMONY_COOKIE);
	if (ceremonyKey === undefined) {
		return json({ error_code: 'ADMIN_WEBAUTHN_CHALLENGE_EXPIRED' }, { status: 400 });
	}

	const body = (await request.json()) as { response?: unknown };
	if (typeof body.response !== 'object' || body.response === null) {
		return json({ error_code: 'ADMIN_WEBAUTHN_VERIFICATION_FAILED' }, { status: 400 });
	}

	try {
		const outcome = await verifyAuthentication(deps, {
			ceremonyKey,
			response: body.response as AuthenticationResponseJSON,
		});
		cookies.delete(CEREMONY_COOKIE, { path: '/' });
		// Post-assertion session (§10-F): httpOnly + Secure + SameSite=Strict.
		cookies.set(ADMIN_SESSION_COOKIE, await createSession(outcome.adminId, platform), SESSION_COOKIE_OPTIONS);
		return json({ admin_id: outcome.adminId });
	} catch (e) {
		cookies.delete(CEREMONY_COOKIE, { path: '/' });
		if (e instanceof WebAuthnError) {
			return json({ error_code: e.code }, { status: 400 });
		}
		throw e;
	}
};
