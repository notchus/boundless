import { describe, expect, it } from 'vitest';

import type { StoredCredential } from './ports';
import { MemoryCredentialStore } from './testing/memory-stores';

const ADMIN = 'admin-1';

function cred(credentialId: string): StoredCredential {
  return { credentialId, adminId: ADMIN, publicKey: new Uint8Array([1]), signCount: 0, revokedAt: null };
}

// AC20: more than one credential per admin; ADR-0016 D4: a Developer re-invite registration
// revokes the prior credential(s). The store contract that backs both, exercised directly.
// NOTE: the store genuinely supports >1 active credential per admin (proven below). The
// *invite-gated* verifyRegistration path is revoke-and-replace by design (initial/recovery, D4);
// the additive backup-key *enrollment* orchestration (authenticated, no-revoke) is the deferred
// shell (T15 — needs the admin session). See DEFERRED.md → Server / admin-WebAuthn (T09).
describe('admin credential store (AC20 multi-cred + ADR-0016 D4 recovery)', () => {
  it('supports more than one active credential per admin', async () => {
    const store = new MemoryCredentialStore();
    await store.insert(cred('passkey'));
    await store.insert(cred('backup-key'));
    expect((await store.listActiveByAdmin(ADMIN)).map((c) => c.credentialId).sort()).toEqual([
      'backup-key',
      'passkey',
    ]);
  });

  it('revokeAllForAdmin marks every prior credential revoked (lost-key recovery)', async () => {
    const store = new MemoryCredentialStore();
    await store.insert(cred('passkey'));
    await store.insert(cred('backup-key'));

    await store.revokeAllForAdmin(ADMIN, 1234);

    expect(await store.listActiveByAdmin(ADMIN)).toHaveLength(0);
    expect(await store.findActive('passkey')).toBeNull();
    expect(store.all().every((c) => c.revokedAt === 1234)).toBe(true);
  });

  it('bumpSignCount updates the stored counter', async () => {
    const store = new MemoryCredentialStore();
    await store.insert(cred('passkey'));
    await store.bumpSignCount('passkey', 7, 1234);
    expect((await store.findActive('passkey'))?.signCount).toBe(7);
  });
});
