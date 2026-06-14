// Scrubbed server-log sink for the admin web (spec 009 T08, P2/I10). Every server-side log line funnels
// through `emit()`, which runs the text through `scrub()` BEFORE it reaches stdout/Logpush — so an
// operator secret, a Postgres connection string, or the URL-embedded single-use invite token (R13) can
// never survive into a persisted log. This is the web-tier counterpart of the Rust core's
// `boundless::logging` scrubber; the detector is deliberately lean (the known web egress is operator
// strings + secrets + the invite token), with residual gaps mirrored to the Rust scrubber's DEFERRED list.
//
// FUNCTIONAL CORE: this module is PURE (no `$app`/`$env`/SvelteKit virtuals), so it is unit-testable under
// the bare Vitest config. The shell (`hooks.server.ts`) wires `logServerError` into SvelteKit's
// `handleError`. `console` is called ONLY here — the no-raw-`console` lint (web-no-raw-console.test.ts)
// allow-lists this one file, so all output is guaranteed to pass through `scrub()`.

const REDACTED = '[redacted]';

// Order matters: redact the structured secrets first (Bearer / connection string), THEN the generic
// high-entropy blob. The single-use invite token is opaque and ≥32-char high-entropy (64-hex / UUID /
// base64url), so the blob rule catches it ANYWHERE — including embedded in a URL path (R13) — without a
// dedicated URL-segment rule that would false-positive on route literals (`/invite/resolve`). The handler
// also logs the route PATTERN (`/admin/onboard/[token]`), never the live `url.pathname`, as the first line
// of defense; the B1 token rides in the POST body (ADR-0027), never the URL.
const SCRUBBERS: readonly { readonly re: RegExp; readonly replace: string }[] = [
	// `authorization: Bearer <secret>` (the ADR-0026 BFF shared secret on the BFF→Worker hop).
	{ re: /(Bearer\s+)[A-Za-z0-9._~+/=-]+/gi, replace: `$1${REDACTED}` },
	// A Postgres connection string (a real Neon URL accidentally on a log path carries the DB password).
	{ re: /postgres(?:ql)?:\/\/[^\s"']+/gi, replace: `postgres://${REDACTED}` },
	// A generic high-entropy blob: a ≥32-char run of token charset carrying BOTH a letter and a digit
	// (so a long hex/base64url/UUID token/secret is caught while an ordinary long word/identifier is spared).
	{ re: /[A-Za-z0-9_-]{32,}/g, replace: REDACTED },
];

/**
 * Redact secrets / PII-shaped values from a log string before it is persisted (P2/I10). Idempotent and
 * total — never throws. Only the high-entropy blob rule needs the letter+digit guard (below).
 */
export function scrub(input: string): string {
	let out = input;
	for (const { re, replace } of SCRUBBERS) {
		out =
			re.source === '[A-Za-z0-9_-]{32,}'
				? out.replace(re, (m) => (/[A-Za-z]/.test(m) && /[0-9]/.test(m) ? replace : m))
				: out.replace(re, replace);
	}
	return out;
}

export type LogLevel = 'error' | 'warn' | 'info';

function safeStringify(fields: Record<string, unknown>): string {
	try {
		return JSON.stringify(fields);
	} catch {
		return '"[unserializable fields]"';
	}
}

/**
 * Emit one scrubbed server log line. The SOLE sanctioned `console` call site in `web/src` — every other
 * server log path routes through here so it is scrubbed (P2/I10). On the Cloudflare Worker runtime
 * `console.*` is the Logpush/tail sink.
 */
export function emit(level: LogLevel, message: string, fields?: Record<string, unknown>): void {
	const line = scrub(fields ? `[${level}] ${message} ${safeStringify(fields)}` : `[${level}] ${message}`);
	// The one sanctioned `console` call site (the no-raw-console lint allow-lists this file); `line` is
	// already scrubbed. Level maps to the matching console method — info uses `console.info`, never the bare
	// `log` debug method the pre-commit hook forbids everywhere; info/warn/error are real structured logging.
	const sink = level === 'error' ? console.error : level === 'warn' ? console.warn : console.info;
	sink(line);
}

/**
 * Log an uncaught server error through the scrubbed sink. Called by the `handleError` hook. Logs the
 * route PATTERN (`/admin/onboard/[token]`), never `url.pathname` (which carries the live token); `scrub`
 * is the defense-in-depth backstop on the error message itself.
 */
export function logServerError(args: { error: unknown; routeId?: string | null; status?: number }): void {
	emit('error', 'unhandled server error', {
		status: args.status ?? 500,
		route: args.routeId ?? null,
		detail: args.error instanceof Error ? args.error.message : String(args.error),
	});
}
