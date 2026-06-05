// Per-request locale resolution (spec 001 T15). Resolves the request locale and stamps `<html lang>`
// + `<html dir>` so RTL locales mirror the layout (a11y bar / AC11b). `locals.locale` is surfaced to
// pages via +layout.server.ts so `t()` renders catalog copy in the right locale.

import type { Handle } from '@sveltejs/kit';

import { direction, resolveLocale } from '$lib/i18n';

export const handle: Handle = async ({ event, resolve }) => {
	const locale = resolveLocale(event.url, event.cookies);
	event.locals.locale = locale;
	const dir = direction(locale);

	return resolve(event, {
		transformPageChunk: ({ html }) =>
			html.replace('lang="en" dir="ltr"', `lang="${locale}" dir="${dir}"`),
	});
};
