// Surfaces the request-resolved locale (hooks.server.ts) to every page so `t()` renders catalog copy
// in the active locale. Cascades to all child routes' `data`.

import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = ({ locals }) => {
	return { locale: locals.locale };
};
