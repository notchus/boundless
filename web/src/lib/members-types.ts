// Wire types for the admin member-management surface (spec 008 T10), derived by hand from the frozen
// `api/openapi.yaml` admin schemas (plan §6: hand-rolled-but-derived for v1; pinned by the T08 contract
// test). Kept OUT of `$lib/server/` so client components may `import type` them to type `data.members`
// etc. — these are pure types (no logic, no secrets); the server-only Worker client lives in
// `$lib/server/members.ts` and imports + re-exports them.

/** A role an EXISTING member may hold (display). Mirrors `Role` (includes `admin`). */
export type Role = 'rider' | 'driver' | 'admin';
/** A role a member may be ISSUED with — Rider/Driver only; `admin` is unrepresentable (I11/AC10). */
export type IssuableRole = 'rider' | 'driver';
/** Where a member is in the issue → onboard lifecycle. Mirrors `core::server::OnboardingStatus`. */
export type OnboardingStatus =
	| 'issued_not_onboarded'
	| 'onboarded'
	| 'code_expired_or_lost'
	| 'needs_reonboarding';

/** The PII-free member-LIST projection (AC8) — no phone/address; listing is not an audited read. */
export interface MemberSummary {
	readonly member_id: string;
	readonly name: string;
	readonly roles: Role[];
	readonly onboarding_status: OnboardingStatus;
}

/** The audited member-DETAIL view (decrypted PII). TLS-only; NEVER logged (P2). */
export interface MemberDetail {
	readonly member_id: string;
	readonly name: string;
	readonly phone: string;
	readonly address: string;
	readonly roles: Role[];
	readonly onboarding_status: OnboardingStatus;
	/** The optimistic-concurrency token (server-time epoch seconds) echoed back on edit (AC11). */
	readonly updated_at: number;
}

export interface IssueMemberRequest {
	readonly name: string;
	readonly phone: string;
	readonly address: string;
	readonly roles: IssuableRole[];
}

export interface EditMemberRequest {
	readonly name?: string;
	readonly phone?: string;
	readonly address?: string;
	readonly roles?: IssuableRole[];
	readonly expected_updated_at: number;
}

export type AuditField = 'name' | 'phone' | 'address';

/** One I5 PII-read audit record — field NAMES, never values (AC9). */
export interface AuditEntry {
	readonly timestamp: number;
	readonly admin_id: string;
	readonly member_id: string;
	readonly fields: AuditField[];
	readonly request_id: string;
}

export interface ListParams {
	readonly search?: string;
	readonly role?: Role;
	readonly status?: OnboardingStatus;
}

// — Outcomes (HTTP status → a typed result the page/action consumes; the raw Response never escapes) —

export type IssueOutcome =
	| { readonly kind: 'issued'; readonly member: MemberSummary; readonly onboarding_code?: string; readonly code_expires_at: number }
	| { readonly kind: 'duplicate'; readonly existing: MemberSummary }
	| { readonly kind: 'rejected'; readonly code: string }
	| { readonly kind: 'group_key_missing' };

export type DetailOutcome =
	| { readonly kind: 'detail'; readonly detail: MemberDetail }
	| { readonly kind: 'not_found' }
	| { readonly kind: 'group_key_missing' };

export type EditOutcome =
	| { readonly kind: 'updated'; readonly detail: MemberDetail }
	| { readonly kind: 'stale' }
	| { readonly kind: 'rejected'; readonly code: string }
	| { readonly kind: 'not_found' }
	| { readonly kind: 'group_key_missing' };

export type RegenerateOutcome =
	| { readonly kind: 'regenerated'; readonly onboarding_code?: string; readonly code_expires_at: number }
	| { readonly kind: 'not_found' };

/** The port both the real Worker adapter and the in-memory fake implement. Every method carries the
 *  acting (WebAuthn-verified) admin id — it becomes the `X-Admin-Id` actor on the I5 audit row. */
export interface MembersClient {
	list(adminId: string, params: ListParams): Promise<MemberSummary[]>;
	issue(adminId: string, req: IssueMemberRequest): Promise<IssueOutcome>;
	detail(adminId: string, id: string): Promise<DetailOutcome>;
	edit(adminId: string, id: string, req: EditMemberRequest): Promise<EditOutcome>;
	regenerateCode(adminId: string, id: string): Promise<RegenerateOutcome>;
	auditLog(adminId: string, params: { readonly memberId?: string }): Promise<AuditEntry[]>;
}

/** What a dev/e2e seam may seed (all optional but name/phone/address sensible). */
export interface SeedMemberInput {
	readonly member_id?: string;
	readonly name: string;
	readonly phone: string;
	readonly address: string;
	readonly roles?: Role[];
	readonly onboarding_status?: OnboardingStatus;
}
