// Per-request locale resolution (spec 001 T15). Resolves the request locale and stamps `<html lang>`
// + `<html dir>` so RTL locales mirror the layout (a11y bar / AC11b). `locals.locale` is surfaced to
// pages via +layout.server.ts so `t()` renders catalog copy in the right locale.

import type { Handle, HandleServerError } from '@sveltejs/kit';

import { direction, resolveLocale } from '$lib/i18n';
import { logServerError } from '$lib/server/log';

// Route every uncaught server error through the scrubbed sink (P2/I10, spec 009 T08) so a thrown secret /
// connection string / URL-embedded invite token can never reach a persisted log. Returns void → SvelteKit's
// default client error shape stands (no new user-visible copy — P8); the audit/log line is the side effect.
export const handleError: HandleServerError = ({ error, event, status }) => {
	logServerError({ error, routeId: event.route?.id, status });
};

export const handle: Handle = async ({ event, resolve }) => {
	const locale = resolveLocale(event.url, event.cookies);
	event.locals.locale = locale;
	const dir = direction(locale);

	return resolve(event, {
		transformPageChunk: ({ html }) =>
			html.replace('lang="en" dir="ltr"', `lang="${locale}" dir="${dir}"`),
	});
};
