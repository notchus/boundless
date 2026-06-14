// Response security headers (spec 009 T12, AC11). Pure core — no `$app`/`$env`/`$lib` imports — so the
// header contract is unit-testable under bare Vitest (the house functional-core + thin-shell pattern);
// the `hooks.server` `handle` shell just calls this on the resolved response.

/**
 * Apply Boundless's app-wide response security headers and return the same response.
 *
 * `Referrer-Policy: no-referrer` (AC11 / F13): the single-use admin invite token rides in the URL path
 * (`/admin/onboard/<token>`, `/api/admin/auth/invite/<token>`), so the browser must never leak it via the
 * `Referer` header on a sub-resource load or onward navigation. A blanket no-referrer is a safe default
 * for the whole admin surface — no third-party referer is ever wanted — and covers every token-bearing
 * URL without per-route plumbing.
 */
export function applySecurityHeaders(response: Response): Response {
	response.headers.set('Referrer-Policy', 'no-referrer');
	return response;
}
