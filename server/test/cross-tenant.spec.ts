// Cross-tenant isolation proof for the spec-008 **T11** admin member endpoints (AC16 / sec-audit F5).
//
// This is the Worker-HTTP-level precursor to the live deployed-edge smoke (scripts/smoke-deployed-edge.sh
// cross-tenant block + docs/runbooks/deploy-worker.md). It runs the SAME real Rust→wasm Worker over the
// SAME local Postgres as server/test/admin-members.spec.ts — connecting as the non-superuser, NON-
// `BYPASSRLS` `boundless_app` role (scripts/setup-worker-test-db.sh), so RLS genuinely applies and the W2
// `ensure_least_privilege` guard accepts the role. The Worker is RLS-scoped to its single `GROUP_ID`
// binding (Group A = …0001, the install's tenant); `X-Admin-Id` is only the I5 audit actor, NOT a tenant
// selector.
//
// The setup script seeds a SECOND tenant — Group B (…2000…) + one Group-B member
// (CROSS_TENANT_MEMBER_ID) — directly in the same database (as superuser, bypassing RLS). Group B has NO
// `delegated_keys`/KEK and the member's PII columns are NULL: Worker-A must never even SELECT the row, let
// alone decrypt it. So a Group-A admin (the only admin this single-install Worker can act as) must be
// unable to list, read, edit, or regenerate-code that Group-B member. That is the database-level isolation
// (RLS + the locked-down role) proven THROUGH the real Worker code path — strictly stronger than the
// store-level `rls_isolates_member_reads_by_tenant` (T07), which runs as a superuser with `SET ROLE`.
//
// The genuinely-live version (this same proof against the DEPLOYED edge, with ≥2 Groups seeded in Neon)
// is the operator-gated shell — see DEFERRED.md → "Server / Worker — cross-tenant deployed-edge (T11)".

import { SELF } from 'cloudflare:test';
import { describe, expect, it } from 'vitest';

const BASE = 'https://worker.example';
// Must match `ADMIN_API_SECRET` in vitest.config.ts.
const SECRET = 'test-admin-shared-secret-do-not-deploy';
// The BFF-asserted acting admin id (any uuid — the I5 audit actor). NOT a tenant selector.
const ADMIN_ID = '00000000-0000-0000-0000-0000000000aa';

// The Group-B member seeded by scripts/setup-worker-test-db.sh (group_id = 2000…). The Worker is scoped
// to Group A (GROUP_ID = …0001), so this id is invisible to it. Keep in lock-step with the setup script's
// XTENANT_MEMBER_ID.
const CROSS_TENANT_MEMBER_ID = '2b000000-0000-0000-0000-000000000000';
// The fixed `updated_at` (epoch) that scripts/setup-worker-test-db.sh seeds on the Group-B member, so the
// edit case can pass the member's REAL timestamp — see the edit step below.
const CROSS_TENANT_UPDATED_AT = 1700000000;

function adminHeaders(extra: Record<string, string> = {}): Record<string, string> {
	return {
		'content-type': 'application/json',
		authorization: `Bearer ${SECRET}`,
		'x-admin-id': ADMIN_ID,
		...extra,
	};
}

// A per-run-unique E.164 (random base + counter) so re-running `pnpm test` without a DB reset does not
// collide on the (group_id, phone_lookup_hash) index (mirrors admin-members.spec.ts).
const RUN_BASE = String(Math.floor(Math.random() * 9000) + 1000);
let phoneCounter = 0;
function uniquePhone(): string {
	phoneCounter += 1;
	return `+1556${RUN_BASE}${String(phoneCounter).padStart(3, '0')}`;
}

/* eslint-disable @typescript-eslint/no-explicit-any */
describe('cross-tenant isolation (spec 008 T11 — AC16 / sec-audit F5)', () => {
	it('worker_cross_tenant_admin_cannot_read_other_group: a Group-A admin cannot list/read/edit/regenerate a Group-B member', async () => {
		// 1. READ by id — RLS hides the Group-B member ⇒ 404, never the row.
		const detail = await SELF.fetch(`${BASE}/api/admin/members/${CROSS_TENANT_MEMBER_ID}`, {
			headers: adminHeaders(),
		});
		expect(detail.status).toBe(404);
		expect(((await detail.json()) as any).error_code).toBe('ADMIN_MEMBER_NOT_FOUND');

		// 2. LIST — issue a Group-A member first (so the list is non-empty and we prove the list works),
		// then assert the Group-B member id is absent from the response.
		const issued = await SELF.fetch(`${BASE}/api/admin/members`, {
			method: 'POST',
			headers: adminHeaders(),
			body: JSON.stringify({
				name: 'Maria',
				phone: uniquePhone(),
				address: '12 Olive St',
				roles: ['rider'],
			}),
		});
		expect(issued.status).toBe(201);
		const myId = ((await issued.json()) as any).member.member_id as string;

		const listRes = await SELF.fetch(`${BASE}/api/admin/members`, { headers: adminHeaders() });
		expect(listRes.status).toBe(200);
		const listRaw = await listRes.text();
		expect(listRaw).toContain(myId); // the Group-A member IS listed (the list is live)…
		expect(listRaw).not.toContain(CROSS_TENANT_MEMBER_ID); // …the Group-B member is NOT (RLS).

		// 3. EDIT — a cross-tenant write cannot reach the row. We pass the member's REAL updated_at (the
		// fixed value seeded for Group B), so the optimistic-concurrency `WHERE id=$1 AND
		// floor(epoch(updated_at))=$expected` WOULD match the row if it were visible — a 409 (0 rows)
		// therefore means RLS hid an otherwise-matching row, NOT merely a stale-timestamp miss (the
		// non-vacuous form: with broken RLS the row matches → the edit proceeds → the read-back decrypt
		// fails → 5xx, never this clean 409). A role-only edit needs no Group key, so the 409 is a pure
		// RLS signal.
		const editRes = await SELF.fetch(`${BASE}/api/admin/members/${CROSS_TENANT_MEMBER_ID}`, {
			method: 'PATCH',
			headers: adminHeaders(),
			body: JSON.stringify({ roles: ['rider'], expected_updated_at: CROSS_TENANT_UPDATED_AT }),
		});
		expect(editRes.status).toBe(409);
		const editRaw = await editRes.text();
		expect(editRaw).toContain('ADMIN_MEMBER_EDIT_STALE');
		// The edit returned no member detail (a successful read-back would carry `updated_at`/`name`).
		expect(editRaw).not.toContain('updated_at');
		expect(editRaw).not.toContain('"name"');

		// 4. REGENERATE-CODE — the member is not visible in this tenant ⇒ NotFound (404), no code minted.
		const regen = await SELF.fetch(
			`${BASE}/api/admin/members/${CROSS_TENANT_MEMBER_ID}/regenerate-code`,
			{ method: 'POST', headers: adminHeaders() },
		);
		expect(regen.status).toBe(404);
		const regenBody = (await regen.json()) as any;
		expect(regenBody.error_code).toBe('ADMIN_MEMBER_NOT_FOUND');
		expect(regenBody.onboarding_code).toBeUndefined(); // no code for a cross-tenant member
	});
});
