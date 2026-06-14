// applySecurityHeaders — the response-header contract (spec 009 T12, AC11 / F13).
//
// The pure core is relative-import + bare-Vitest testable; `hooks.server` applies it on every resolved
// response (a one-liner shell), and the edge smoke + the deployed Playwright leg re-assert it live.

import { describe, expect, it } from 'vitest';

import { applySecurityHeaders } from './security-headers';

describe('applySecurityHeaders (AC11 / F13)', () => {
	it('sets Referrer-Policy: no-referrer (the URL-embedded invite-token leak guard)', () => {
		const response = applySecurityHeaders(new Response('x'));
		expect(response.headers.get('referrer-policy')).toBe('no-referrer');
	});

	it('preserves the response status + body', async () => {
		const response = applySecurityHeaders(new Response('signin', { status: 200 }));
		expect(response.status).toBe(200);
		expect(await response.text()).toBe('signin');
		expect(response.headers.get('referrer-policy')).toBe('no-referrer');
	});
});
