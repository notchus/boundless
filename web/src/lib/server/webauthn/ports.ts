// The I/O boundary for admin-WebAuthn verification — the TypeScript-edge analogue of the
// Rust core's store ports (AuthStore/DeviceStore). The pure verification logic
// (register.ts / authenticate.ts) depends only on these interfaces; production wires them
// to Cloudflare KV (challenges) + Postgres via the Worker (invites/credentials), and the
// tests wire the in-memory fakes in ./testing. Keeping persistence behind ports is what
// lets T09 ship + fully test the logic before the deployable SvelteKit shell (T15) exists.

import type { AuthenticatorTransportFuture, WebAuthnCredential } from '@simplewebauthn/server';

/** Injected clock — unix **seconds**. No `Date.now()` in the verification logic (T04/T05 discipline). */
export interface Clock {
  now(): number;
}

/**
 * Per-ceremony WebAuthn challenge store (Cloudflare KV in production; ADR-0017 D3).
 * `take` is **consume-once**: it returns the stored challenge and atomically deletes it, and
 * yields `null` if the key is absent or past its TTL — so a replayed/expired challenge fails.
 */
export interface ChallengeStore {
  put(key: string, challenge: string, ttlSecs: number): Promise<void>;
  take(key: string): Promise<string | null>;
}

/** A pending-admin invitation row as the store sees it (admin_invitations; T06/T08). */
export interface InviteRecord {
  readonly adminId: string;
  readonly groupId: string;
  /** Server-side TTL instant, unix seconds (admin_invitations.expires_at). */
  readonly expiresAt: number;
  /** Single-use marker (admin_invitations.consumed_at); null while live. */
  readonly consumedAt: number | null;
}

/**
 * Admin invitation store. `load` resolves the **presented** registration token to its row
 * (production hashes the token with the per-instance HMAC and compares against
 * `token_hash` via the core's `admin_invitation_token_matches` — that crypto stays
 * server-side per ADR-0017's P4 carve-out; here it is abstracted behind the port).
 * `markConsumed` stamps `consumed_at` (single-use, AC16). TTL/consumed policy is evaluated
 * by `evaluateInvite` (invite.ts), not the store.
 */
export interface InviteStore {
  load(presentedToken: string): Promise<InviteRecord | null>;
  markConsumed(presentedToken: string, now: number): Promise<void>;
}

/** A stored admin WebAuthn credential (admin_webauthn_credentials; T06). Multiple per admin (AC20). */
export interface StoredCredential {
  /** WebAuthn credential id, base64url (DB: credential_id bytea). */
  readonly credentialId: string;
  readonly adminId: string;
  /** COSE public key bytes (DB: public_key bytea). Not PII. Typed off the library's own
   * `WebAuthnCredential` so the registration output and the assertion-verify input agree. */
  readonly publicKey: WebAuthnCredential['publicKey'];
  /** WebAuthn signature counter (DB: sign_count bigint). */
  readonly signCount: number;
  readonly transports?: AuthenticatorTransportFuture[];
  readonly aaguid?: string;
  /** Revocation instant, unix seconds; null while active (DB: revoked_at). */
  readonly revokedAt: number | null;
}

/**
 * Admin WebAuthn credential store. Reads return **active** (non-revoked) credentials only.
 * `revokeAllForAdmin` is the ADR-0016 D4 recovery primitive (a Developer re-invite
 * registration revokes the admin's prior credentials). Multiple active credentials per
 * admin are supported — `insert` never replaces (AC20).
 */
export interface CredentialStore {
  listActiveByAdmin(adminId: string): Promise<StoredCredential[]>;
  findActive(credentialId: string): Promise<StoredCredential | null>;
  insert(credential: StoredCredential): Promise<void>;
  revokeAllForAdmin(adminId: string, now: number): Promise<void>;
  bumpSignCount(credentialId: string, newCount: number, now: number): Promise<void>;
}

/** Everything the verification functions need, wired by the shell (or the test harness). */
export interface WebAuthnDeps {
  readonly rp: import('./config').RpConfig;
  readonly clock: Clock;
  readonly challenges: ChallengeStore;
  readonly invites: InviteStore;
  readonly credentials: CredentialStore;
}
