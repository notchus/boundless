// In-memory port implementations for testing the WebAuthn verification logic — the web
// analogue of the Rust core's in-memory `MemStore` test double. Used by both the Vitest
// unit tests and the Playwright real-ceremony test. NOT production code (the real KV/Postgres
// implementations are the shell, T15). Extra inspector methods (beyond the port interface)
// exist for assertions, exactly like the Rust `MemStore` helpers.

import type {
  ChallengeStore,
  Clock,
  CredentialStore,
  InviteRecord,
  InviteStore,
  StoredCredential,
} from '../ports';

/** A settable clock (unix seconds) so TTL/expiry are driven by the test, not wall time. */
export class FixedClock implements Clock {
  private current: number;
  constructor(initial: number) {
    this.current = initial;
  }
  now(): number {
    return this.current;
  }
  set(value: number): void {
    this.current = value;
  }
  advance(seconds: number): void {
    this.current += seconds;
  }
}

/** Consume-once challenge store, TTL evaluated against the injected clock. */
export class MemoryChallengeStore implements ChallengeStore {
  private readonly entries = new Map<string, { challenge: string; expiresAt: number }>();
  constructor(private readonly clock: Clock) {}

  async put(key: string, challenge: string, ttlSecs: number): Promise<void> {
    this.entries.set(key, { challenge, expiresAt: this.clock.now() + ttlSecs });
  }

  async take(key: string): Promise<string | null> {
    const entry = this.entries.get(key);
    if (entry === undefined) {
      return null;
    }
    // One-time use: delete on read regardless of expiry.
    this.entries.delete(key);
    if (this.clock.now() >= entry.expiresAt) {
      return null;
    }
    return entry.challenge;
  }

  /** Test inspector: is a challenge still outstanding for `key`? */
  has(key: string): boolean {
    return this.entries.has(key);
  }
}

export interface SeedInvite {
  readonly adminId: string;
  readonly groupId: string;
  readonly expiresAt: number;
  readonly consumedAt?: number | null;
}

export class MemoryInviteStore implements InviteStore {
  private readonly rows = new Map<string, { adminId: string; groupId: string; expiresAt: number; consumedAt: number | null }>();

  async load(presentedToken: string): Promise<InviteRecord | null> {
    const row = this.rows.get(presentedToken);
    return row === undefined ? null : { ...row };
  }

  async markConsumed(presentedToken: string, now: number): Promise<void> {
    const row = this.rows.get(presentedToken);
    if (row !== undefined && row.consumedAt === null) {
      row.consumedAt = now;
    }
  }

  /** Test seeding: register a pending-admin invitation row. */
  add(token: string, invite: SeedInvite): void {
    this.rows.set(token, {
      adminId: invite.adminId,
      groupId: invite.groupId,
      expiresAt: invite.expiresAt,
      consumedAt: invite.consumedAt ?? null,
    });
  }

  /** Test inspector: is this invite still unconsumed? */
  isConsumed(token: string): boolean {
    return this.rows.get(token)?.consumedAt != null;
  }
}

export class MemoryCredentialStore implements CredentialStore {
  private readonly creds: StoredCredential[] = [];

  async listActiveByAdmin(adminId: string): Promise<StoredCredential[]> {
    return this.creds.filter((c) => c.adminId === adminId && c.revokedAt === null).map((c) => ({ ...c }));
  }

  async findActive(credentialId: string): Promise<StoredCredential | null> {
    const found = this.creds.find((c) => c.credentialId === credentialId && c.revokedAt === null);
    return found === undefined ? null : { ...found };
  }

  async insert(credential: StoredCredential): Promise<void> {
    this.creds.push({ ...credential });
  }

  async revokeAllForAdmin(adminId: string, now: number): Promise<void> {
    for (const [i, c] of this.creds.entries()) {
      if (c.adminId === adminId && c.revokedAt === null) {
        this.creds[i] = { ...c, revokedAt: now };
      }
    }
  }

  async bumpSignCount(credentialId: string, newCount: number, _now: number): Promise<void> {
    for (const [i, c] of this.creds.entries()) {
      if (c.credentialId === credentialId) {
        this.creds[i] = { ...c, signCount: newCount };
      }
    }
  }

  /** Test inspector: every stored credential (incl. revoked), in insertion order. */
  all(): StoredCredential[] {
    return this.creds.map((c) => ({ ...c }));
  }
}
