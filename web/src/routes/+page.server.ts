// The admin web has no public landing — the only entry points are the developer-minted invite link
// (`/admin/onboard/[token]`) and admin sign-in (`/admin/signin`). The root redirects to sign-in;
// there is no signup surface anywhere (AC1(b) / I11).

import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => {
	redirect(307, '/admin/signin');
};
