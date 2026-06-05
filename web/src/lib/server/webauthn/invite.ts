// Admin-invitation lifecycle policy (AC16 / I11 as narrowed by ADR-0015). Pure and
// directly unit-testable: TTL is evaluated against an injected server-time `now` (unix
// seconds), never a device clock. Mirrors the Rust core's `evaluate_onboarding_code` shape
// (a discriminated verdict; "not live" carries the precise error code).

import { WebAuthnError, type WebAuthnErrorCode } from './errors';
import type { InviteRecord, InviteStore } from './ports';

export type InviteVerdict =
  | { readonly status: 'live'; readonly adminId: string; readonly groupId: string; readonly expiresAt: number }
  | { readonly status: 'expired'; readonly code: Extract<WebAuthnErrorCode, 'ADMIN_INVITE_EXPIRED'> }
  | { readonly status: 'consumed'; readonly code: Extract<WebAuthnErrorCode, 'ADMIN_INVITE_CONSUMED'> };

/**
 * Decide whether a presented invitation is usable, at server time `now`.
 *
 * Order (single-use beats TTL when both apply, but both route to `InviteExpired`):
 *   unknown token  → expired (absence is treated as expired; no existence oracle)
 *   already used   → consumed (AC16 single-use)
 *   past TTL        → expired (AC16 server-side TTL)
 *   otherwise       → live
 */
export function evaluateInvite(record: InviteRecord | null, now: number): InviteVerdict {
  if (record == null) {
    return { status: 'expired', code: 'ADMIN_INVITE_EXPIRED' };
  }
  if (record.consumedAt != null) {
    return { status: 'consumed', code: 'ADMIN_INVITE_CONSUMED' };
  }
  if (now >= record.expiresAt) {
    return { status: 'expired', code: 'ADMIN_INVITE_EXPIRED' };
  }
  return { status: 'live', adminId: record.adminId, groupId: record.groupId, expiresAt: record.expiresAt };
}

/** Load + evaluate an invite, throwing the precise `WebAuthnError` if it is not live. */
export async function requireLiveInvite(
  invites: InviteStore,
  presentedToken: string,
  now: number,
): Promise<Extract<InviteVerdict, { status: 'live' }>> {
  const verdict = evaluateInvite(await invites.load(presentedToken), now);
  if (verdict.status !== 'live') {
    throw new WebAuthnError(verdict.code);
  }
  return verdict;
}
