// GET /api/admin/auth/invite/[token] — resolve a single-use invitation (AC16).
//
// Matches the frozen contract (api/openapi.yaml `adminAuthInvite`): a LIVE invite → 200 with
// `WebAuthnRegistrationOptions` ({ publicKey, challenge_ttl_secs }) + the ceremony-challenge cookie;
// a reused/expired invite → 410 with `InviteExpired` ({ error_code, routes_to }). The admin invite
// token is opaque + single-use + PII-free, so a 410 leaks no member existence (unlike the member
// `/api/auth/*` endpoints, where the 200-body rule avoids an existence oracle). Serves client retries
// + shares `startRegistrationCeremony` with the SSR page (which uses `resolveInviteStatus` for status).

import { json } from '@sveltejs/kit';

import { CHALLENGE_TTL_SECS } from '$lib/server/webauthn';
import { startRegistrationCeremony } from '$lib/server/webauthn-deps';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params, url, cookies, platform }) => {
	const ceremony = await startRegistrationCeremony(url, cookies, params.token, platform);
	if (ceremony.status === 'live') {
		return json({ publicKey: ceremony.options, challenge_ttl_secs: CHALLENGE_TTL_SECS });
	}
	// ADMIN_INVITE_EXPIRED / ADMIN_INVITE_CONSUMED → InviteExpired terminal.
	return json({ error_code: ceremony.code, routes_to: 'InviteExpired' }, { status: 410 });
};
