// SSR load for the admin registration landing (spec 001 T15). Resolves only invite liveness
// server-side (no ceremony build / challenge / cookie — that is minted lazily by the first register
// click via the GET endpoint), so the initial render is already the register prompt OR the
// InviteExpired terminal with no client loading flash, and the whole screen is axe-scannable on
// first paint.

import { resolveInviteStatus } from '$lib/server/webauthn-deps';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
	const status = await resolveInviteStatus(params.token);
	return { token: params.token, status };
};
