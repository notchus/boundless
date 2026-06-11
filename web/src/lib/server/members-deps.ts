// Composition root for the admin member-management routes (spec 008 T10) — the `webauthn-deps.ts`
// analog. The BFF LOGIC (the port + adapters) lives in the pure `$lib/server/members`; this shell wires
// it: it selects the real Rust-Worker client (ADR-0026) when the Worker base URL + shared secret are
// configured (the deployed edge), else the in-memory fake (dev/e2e), else fails closed.
//
// `ADMIN_WORKER_BASE` (the Worker origin, e.g. https://boundless-worker.<sub>.workers.dev) and
// `ADMIN_API_SECRET` (the server-to-server shared secret) come from `$env/dynamic/private` — on the
// edge these are the wrangler-set secrets; in `vite dev`/Playwright they are unset, so the fake backs
// the UI (the live deployed BFF→Worker round-trip is the deferred shell — DEFERRED.md → T10). The
// in-memory fake is a module-level singleton (`let`) so the dev-only `/api/test/*` seed/reset seams can
// seed + swap it; there is no production deploy yet, so the fake can never serve real traffic.

import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';

import {
	InMemoryMembersClient,
	selectMembersClient,
	type MembersClient,
	type SeedMemberInput,
} from '$lib/server/members';

// Interim in-memory backend — the FALLBACK used when no Worker binding is present (dev/e2e). `let` so
// the dev `/api/test/reset` seam can swap it for per-test isolation.
let fake = new InMemoryMembersClient();

/** The members client for this request: the real Worker client when configured, else the dev fake,
 *  else fail closed (mirrors `webauthn-deps.ts::challengeStore`). */
export function getMembersClient(): MembersClient {
	return selectMembersClient(env.ADMIN_WORKER_BASE, env.ADMIN_API_SECRET, fake, dev);
}

// — Dev-only test seams (the `/api/test/*` routes guard these behind `dev`) —

/** Seed a member into the interim in-memory backend (dev/test only — real members are issued via the
 *  Worker). Returns the seeded summary (its `member_id` lets a test deep-link to detail/edit). */
export function seedMember(input: SeedMemberInput): { member_id: string } {
	return fake.seed(input);
}

/** Reset the interim in-memory member backend (dev/test only) for per-test isolation. */
export function resetMembers(): void {
	fake = new InMemoryMembersClient();
}
