// Admin member-management BFF client (spec 008 T10). The SvelteKit admin tier is a **BFF, not a
// re-implementation** (P4 / ADR-0026): it owns session + presentation and CALLS the Rust Worker
// (`/api/admin/members/*`, T09) with the server-to-server shared secret + the WebAuthn-verified acting
// admin id — it never re-does the crypto/validation/normalization the core owns.
//
// FUNCTIONAL-CORE / IMPERATIVE-SHELL (mirrors `kv-challenge-store.ts`): this module is PURE — types
// derived by hand from the frozen `api/openapi.yaml` (plan §6: hand-rolled-but-derived for v1; no TS
// codegen yet), the `MembersClient` port, the real `WorkerMembersClient` fetch adapter, a seedable
// `InMemoryMembersClient` dev/e2e fake, and the fail-closed `selectMembersClient` decision. It imports
// NO SvelteKit-virtual modules, so it is unit-testable under the bare Vitest config; the imperative
// shell (`members-deps.ts`) supplies the Worker base URL + secret (from `$env/dynamic/private`), the
// in-memory fallback singleton, and `dev`.
//
// This file lives under `src/lib/server/` (SvelteKit never bundles it to the client), so the shared
// secret can never leak into client code. The wire TYPES live in the client-safe `$lib/members-types`
// (so components can type `data.members`); this module imports + re-exports them.

import type {
	AuditEntry,
	AuditField,
	DetailOutcome,
	EditMemberRequest,
	EditOutcome,
	IssueMemberRequest,
	IssueOutcome,
	ListParams,
	MemberDetail,
	MembersClient,
	MemberSummary,
	OnboardingStatus,
	RegenerateOutcome,
	Role,
	SeedMemberInput,
} from '../members-types';

export type {
	AuditEntry,
	AuditField,
	DetailOutcome,
	EditMemberRequest,
	EditOutcome,
	IssuableRole,
	IssueMemberRequest,
	IssueOutcome,
	ListParams,
	MemberDetail,
	MembersClient,
	MemberSummary,
	OnboardingStatus,
	RegenerateOutcome,
	Role,
	SeedMemberInput,
} from '../members-types';

// ── The real Worker adapter (ADR-0026): server-to-server bearer secret + the asserted X-Admin-Id ──

interface ErrorBody {
	readonly error_code: string;
}

export class WorkerMembersClient implements MembersClient {
	constructor(
		private readonly baseUrl: string,
		private readonly secret: string,
	) {}

	private url(path: string): string {
		return `${this.baseUrl.replace(/\/$/, '')}${path}`;
	}

	private headers(adminId: string, json = false): Record<string, string> {
		const h: Record<string, string> = {
			authorization: `Bearer ${this.secret}`,
			'x-admin-id': adminId,
		};
		if (json) h['content-type'] = 'application/json';
		return h;
	}

	/** A non-outcome status (401 BFF-misconfig, or anything unexpected) is an operator fault. Throw a
	 *  value-free error — never echo the response body onto the BFF's own error/log path (P2). */
	private fail(res: Response): Error {
		return new Error(`admin member Worker call failed: HTTP ${res.status}`);
	}

	async list(adminId: string, params: ListParams): Promise<MemberSummary[]> {
		const qs = new URLSearchParams();
		if (params.search) qs.set('search', params.search);
		if (params.role) qs.set('role', params.role);
		if (params.status) qs.set('status', params.status);
		const q = qs.toString();
		const res = await fetch(this.url(`/api/admin/members${q ? `?${q}` : ''}`), {
			headers: this.headers(adminId),
		});
		if (res.status === 200) return ((await res.json()) as { members: MemberSummary[] }).members;
		throw this.fail(res);
	}

	async issue(adminId: string, req: IssueMemberRequest): Promise<IssueOutcome> {
		const res = await fetch(this.url('/api/admin/members'), {
			method: 'POST',
			headers: this.headers(adminId, true),
			body: JSON.stringify(req),
		});
		if (res.status === 201) {
			const b = (await res.json()) as { member: MemberSummary; onboarding_code?: string; code_expires_at: number };
			return { kind: 'issued', member: b.member, onboarding_code: b.onboarding_code, code_expires_at: b.code_expires_at };
		}
		if (res.status === 409) {
			const b = (await res.json()) as { existing: MemberSummary };
			return { kind: 'duplicate', existing: b.existing };
		}
		if (res.status === 400) return { kind: 'rejected', code: ((await res.json()) as ErrorBody).error_code };
		if (res.status === 503) return { kind: 'group_key_missing' };
		throw this.fail(res);
	}

	async detail(adminId: string, id: string): Promise<DetailOutcome> {
		const res = await fetch(this.url(`/api/admin/members/${encodeURIComponent(id)}`), {
			headers: this.headers(adminId),
		});
		if (res.status === 200) return { kind: 'detail', detail: (await res.json()) as MemberDetail };
		if (res.status === 404) return { kind: 'not_found' };
		if (res.status === 503) return { kind: 'group_key_missing' };
		throw this.fail(res);
	}

	async edit(adminId: string, id: string, req: EditMemberRequest): Promise<EditOutcome> {
		const res = await fetch(this.url(`/api/admin/members/${encodeURIComponent(id)}`), {
			method: 'PATCH',
			headers: this.headers(adminId, true),
			body: JSON.stringify(req),
		});
		if (res.status === 200) return { kind: 'updated', detail: (await res.json()) as MemberDetail };
		if (res.status === 409) return { kind: 'stale' };
		if (res.status === 400) return { kind: 'rejected', code: ((await res.json()) as ErrorBody).error_code };
		if (res.status === 404) return { kind: 'not_found' };
		if (res.status === 503) return { kind: 'group_key_missing' };
		throw this.fail(res);
	}

	async regenerateCode(adminId: string, id: string): Promise<RegenerateOutcome> {
		const res = await fetch(this.url(`/api/admin/members/${encodeURIComponent(id)}/regenerate-code`), {
			method: 'POST',
			headers: this.headers(adminId),
		});
		if (res.status === 200) {
			const b = (await res.json()) as { onboarding_code?: string; code_expires_at: number };
			return { kind: 'regenerated', onboarding_code: b.onboarding_code, code_expires_at: b.code_expires_at };
		}
		if (res.status === 404) return { kind: 'not_found' };
		throw this.fail(res);
	}

	async auditLog(adminId: string, params: { memberId?: string }): Promise<AuditEntry[]> {
		const q = params.memberId ? `?member_id=${encodeURIComponent(params.memberId)}` : '';
		const res = await fetch(this.url(`/api/admin/audit-log${q}`), { headers: this.headers(adminId) });
		if (res.status === 200) return ((await res.json()) as { entries: AuditEntry[] }).entries;
		throw this.fail(res);
	}
}

// ── In-memory fake (dev + e2e). Mirrors the Worker's contract — the show-once code, the audited detail
//    read, the duplicate-phone 409 link, optimistic-concurrency staleness — so the UI is fully driveable
//    locally with no Worker/account. It holds plaintext (it is the dev backend, never a deploy target —
//    the fail-closed `selectMembersClient` refuses it outside dev). ──

interface StoredMember {
	member_id: string;
	name: string;
	phone: string;
	address: string;
	roles: Role[];
	onboarding_status: OnboardingStatus;
	updated_at: number;
	onboarding_code: string;
	code_expires_at: number;
}

const CODE_TTL_SECS = 72 * 60 * 60;

export class InMemoryMembersClient implements MembersClient {
	private readonly members: StoredMember[] = [];
	private readonly audit: AuditEntry[] = [];
	/** Monotonic non-colliding `updated_at`/timestamp source — so a stale token always differs and the
	 *  optimistic-concurrency reject is exercisable even within one wall-clock second. */
	private stamp = Math.floor(Date.now() / 1000);

	private nextStamp(): number {
		this.stamp += 1;
		return this.stamp;
	}

	private mintCode(): string {
		return `BNDL-${globalThis.crypto.randomUUID().replace(/-/g, '').slice(0, 8).toUpperCase()}`;
	}

	private summary(m: StoredMember): MemberSummary {
		return { member_id: m.member_id, name: m.name, roles: m.roles, onboarding_status: m.onboarding_status };
	}

	private record(adminId: string, memberId: string, fields: AuditField[]): void {
		this.audit.push({
			timestamp: this.nextStamp(),
			admin_id: adminId,
			member_id: memberId,
			fields,
			request_id: globalThis.crypto.randomUUID(),
		});
	}

	/** Light validation mirroring the core's outcomes (the real normalization/hash lives in the core). */
	private validate(name: string, phone: string, address: string, roles: string[]): { code: string } | null {
		if (roles.length === 0) return { code: 'ADMIN_MEMBER_ROLES_REQUIRED' };
		if (roles.some((r) => r === 'admin')) return { code: 'ADMIN_MEMBER_ROLE_FORBIDDEN' };
		if (address.trim() === '') return { code: 'ADMIN_MEMBER_ADDRESS_INVALID' };
		const digits = phone.replace(/[^0-9]/g, '');
		if (!/^\+?[0-9 ()-]+$/.test(phone) || digits.length < 7) return { code: 'ADMIN_MEMBER_PHONE_INVALID' };
		return null;
	}

	/** Normalize a phone the way the duplicate check compares (digits only — the fake's stand-in for
	 *  the core's E.164 `normalize_phone`). */
	private normalize(phone: string): string {
		return phone.replace(/[^0-9]/g, '');
	}

	seed(input: SeedMemberInput): MemberSummary {
		const m: StoredMember = {
			member_id: input.member_id ?? globalThis.crypto.randomUUID(),
			name: input.name,
			phone: input.phone,
			address: input.address,
			roles: input.roles ?? ['rider'],
			onboarding_status: input.onboarding_status ?? 'issued_not_onboarded',
			updated_at: this.nextStamp(),
			onboarding_code: this.mintCode(),
			code_expires_at: Math.floor(Date.now() / 1000) + CODE_TTL_SECS,
		};
		this.members.push(m);
		return this.summary(m);
	}

	async list(_adminId: string, params: ListParams): Promise<MemberSummary[]> {
		const search = params.search?.trim().toLowerCase();
		return this.members
			.filter((m) => (search ? m.name.toLowerCase().includes(search) : true))
			.filter((m) => (params.role ? m.roles.includes(params.role) : true))
			.filter((m) => (params.status ? m.onboarding_status === params.status : true))
			.map((m) => this.summary(m));
	}

	async issue(adminId: string, req: IssueMemberRequest): Promise<IssueOutcome> {
		const rejected = this.validate(req.name, req.phone, req.address, req.roles);
		if (rejected) return { kind: 'rejected', code: rejected.code };

		const existing = this.members.find((m) => this.normalize(m.phone) === this.normalize(req.phone));
		if (existing) {
			// Duplicate disclosure is an admin-only, I5-audited read (name only — R9).
			this.record(adminId, existing.member_id, ['name']);
			return { kind: 'duplicate', existing: this.summary(existing) };
		}

		const m: StoredMember = {
			member_id: globalThis.crypto.randomUUID(),
			name: req.name,
			phone: req.phone,
			address: req.address,
			roles: [...req.roles],
			onboarding_status: 'issued_not_onboarded',
			updated_at: this.nextStamp(),
			onboarding_code: this.mintCode(),
			code_expires_at: Math.floor(Date.now() / 1000) + CODE_TTL_SECS,
		};
		this.members.push(m);
		return { kind: 'issued', member: this.summary(m), onboarding_code: m.onboarding_code, code_expires_at: m.code_expires_at };
	}

	async detail(adminId: string, id: string): Promise<DetailOutcome> {
		const m = this.members.find((x) => x.member_id === id);
		if (!m) return { kind: 'not_found' };
		this.record(adminId, m.member_id, ['name', 'phone', 'address']);
		return {
			kind: 'detail',
			detail: {
				member_id: m.member_id,
				name: m.name,
				phone: m.phone,
				address: m.address,
				roles: m.roles,
				onboarding_status: m.onboarding_status,
				updated_at: m.updated_at,
			},
		};
	}

	async edit(adminId: string, id: string, req: EditMemberRequest): Promise<EditOutcome> {
		const m = this.members.find((x) => x.member_id === id);
		if (!m) return { kind: 'not_found' };
		if (m.updated_at !== req.expected_updated_at) return { kind: 'stale' };

		const name = req.name ?? m.name;
		const phone = req.phone ?? m.phone;
		const address = req.address ?? m.address;
		const roles = req.roles ?? m.roles;
		const rejected = this.validate(name, phone, address, roles);
		if (rejected) return { kind: 'rejected', code: rejected.code };
		// A phone change onto a number already enrolled is an opaque store conflict on the real Worker
		// (T09 register edit-into-duplicate carry-forward); the fake mirrors the happy path only.

		m.name = name;
		m.phone = phone;
		m.address = address;
		m.roles = [...roles];
		m.updated_at = this.nextStamp();
		// PATCH returns MemberDetail (PII) → an audited read (x-requires-audit, contract).
		this.record(adminId, m.member_id, ['name', 'phone', 'address']);
		return {
			kind: 'updated',
			detail: {
				member_id: m.member_id,
				name: m.name,
				phone: m.phone,
				address: m.address,
				roles: m.roles,
				onboarding_status: m.onboarding_status,
				updated_at: m.updated_at,
			},
		};
	}

	async regenerateCode(_adminId: string, id: string): Promise<RegenerateOutcome> {
		const m = this.members.find((x) => x.member_id === id);
		if (!m) return { kind: 'not_found' };
		m.onboarding_code = this.mintCode();
		m.code_expires_at = Math.floor(Date.now() / 1000) + CODE_TTL_SECS;
		return { kind: 'regenerated', onboarding_code: m.onboarding_code, code_expires_at: m.code_expires_at };
	}

	async auditLog(_adminId: string, params: { memberId?: string }): Promise<AuditEntry[]> {
		return params.memberId ? this.audit.filter((e) => e.member_id === params.memberId) : [...this.audit];
	}
}

// ── Fail-closed selection (mirrors `selectChallengeStore`) ──

/**
 * Pick the members client for a request. Returns the real `WorkerMembersClient` (ADR-0026) when BOTH the
 * Worker base URL and the shared secret are configured (the deployed edge). When either is absent it
 * falls back to the in-memory fake ONLY when `allowInMemoryFallback` is true (dev/e2e) — otherwise it
 * **fails closed** by throwing, so a production build with no Worker binding refuses to serve a fake
 * member roster rather than degrade silently.
 *
 * Pure (no SvelteKit-virtual imports) so it is unit-testable; the imperative shell (`members-deps.ts`)
 * supplies the env values, the fallback singleton, and `dev`.
 */
export function selectMembersClient(
	baseUrl: string | undefined,
	secret: string | undefined,
	fallback: MembersClient,
	allowInMemoryFallback: boolean,
): MembersClient {
	if (baseUrl && secret) return new WorkerMembersClient(baseUrl, secret);
	if (allowInMemoryFallback) return fallback;
	throw new Error(
		'Admin member backend unavailable: ADMIN_WORKER_BASE / ADMIN_API_SECRET are not configured. ' +
			'Refusing the in-memory fallback outside dev — it serves an empty/fake member roster and would ' +
			'mask a deploy misconfiguration (ADR-0026).',
	);
}
