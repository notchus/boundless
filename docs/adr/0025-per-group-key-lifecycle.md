# ADR-0025: Per-Group field-encryption key lifecycle — secretbox key, KEK-wrapped in Secrets Store, generated at Group bootstrap

- **Status:** Accepted
- **Date:** 2026-06-10
- **Author:** notch
- **Deciders:** notch
- **Relates to:** I1 (addresses encrypted at rest), I2/P3 (plaintext only during matching), P2 (no PII / no keys in logs), P12 (operability / runbooks); ADR-0014 (Secrets Store / server-driven config); `docs/architecture.md` §5–§6 (`delegated_keys`, DO Per-Group key); `docs/stack-matrix.md` (dryoc); spec 008 (admin member-management / issuance)
- **Was:** an unstated assumption flagged by spec 008's `/clarify` pass ("when/where is the per-Group key generated — does it need an ADR?"). Yes; this is it.

## Context

I1 mandates: "Every persisted `Address` value is stored encrypted with a **per-Group key**.
The Group key is itself encrypted with the **KEK** (Key Encryption Key) stored in
Cloudflare Secrets Store." `docs/architecture.md` names the supporting machinery — a
`delegated_keys` table ("per-Group encryption keys, themselves encrypted with KEK") and a
DO that holds the "Per-Group key (encrypted at rest in DO storage, decrypted via Secrets
Store on init)" — and `docs/stack-matrix.md` assigns "per-Group sealed-box/secretbox PII
encryption (I1)" to **dryoc**, **at spec 008**.

But nothing was decided about the key's **lifecycle**: which primitive (the stack matrix
hedged "sealed-box/secretbox" with a slash — they are *different* libsodium APIs), **when
and where the key is generated**, how the KEK wraps it, and how rotation works. Spec 008
is the first spec that must encrypt a member's address and name at rest (I1, and name by
extension), so it cannot proceed without a real key — and a key cannot exist without
deciding who creates it and when. None of the existing migrations create a `delegated_keys`
table or a Group key, and no spec currently bootstraps the single-install Group row.

This ADR pins the lifecycle so spec 008's AC2 (`i1_addresses_encrypted`) and AC12 (Group
bootstrap) become writable.

## Decision

**The per-Group field-encryption key is a symmetric libsodium *secretbox* key, generated
once at Group bootstrap, stored only KEK-wrapped in `delegated_keys`, and used for
field-level PII-at-rest encryption (address, name, and any future tainted field).**

### Primitive — secretbox (symmetric), not sealed-box (asymmetric)

- The Group key is a **32-byte secretbox key** (dryoc's `crypto_secretbox`,
  XSalsa20-Poly1305). Field encryption = `secretbox(plaintext, nonce, group_key)`; the
  stored `bytea` is `nonce ‖ ciphertext` (a fresh random nonce per encryption, stored
  alongside — never reused).
- Decryption requires the **unwrapped** Group key, so `Address::from_db(bytes, &GroupKey)`
  (I1's stated shape) is the only way to recover plaintext.
- The exact dryoc function names / nonce-length constants are pinned at implementation time
  via `docs-researcher` against the locked dryoc version (anti-hallucination); this ADR
  fixes the *primitive class* (symmetric AEAD secretbox), not the call signatures.

### Wrapping — KEK in Cloudflare Secrets Store

- The Group key is **never persisted in plaintext**. It is wrapped (encrypted) by a **KEK**
  held in **Cloudflare Secrets Store** (ADR-0014's secret home), and the wrapped blob lives
  in `delegated_keys` (`bytea`). The KEK never leaves the server tier and never enters a
  log (P2).
- At runtime the server/DO unwraps the Group key via the KEK and **caches the plaintext key
  in memory only** (architecture §5: "decrypted via Secrets Store on init"). The plaintext
  key is never written to durable storage and is dropped on eviction.

### Generation — at Group bootstrap, owned by spec 008

- Boundless is single-tenant (one install = one Group). **Spec 008 bootstraps the Group**:
  in one transaction it creates the single `groups` row and generates the Group key from a
  CSPRNG (the injected RNG discipline — ADR-0021's `RngSecretSource`, no ambient
  randomness in the core), wraps it with the KEK, and writes the `delegated_keys` row.
- **Issuance fails closed** if no Group key exists (spec 008 AC12) — an address is never
  stored unencrypted as a fallback.

### Rotation — runbook-driven, not automatic

- **KEK rotation** re-wraps the existing Group key (decrypt-with-old-KEK,
  encrypt-with-new-KEK); the plaintext Group key and all PII ciphertext are unchanged.
- **Group-key rotation** requires re-encrypting every PII row under the new key — a
  maintenance Workflow. This is **manual / runbook-driven**, not automatic, and is
  **deferred** (no rotation trigger ships in spec 008). A **key-management runbook**
  (`docs/runbooks/`, P12) documents both procedures and KEK access.

### Scope — field-level at-rest, deliberately *not* E2E

- This is field-level encryption **at rest**. The server **can** decrypt (it holds the KEK
  → the Group key) — this is **required**, not a weakness: matching must decrypt addresses
  into an ephemeral `MatchingContext` and drop them (P3/I2), and the Admin must read PII
  through audited endpoints (I5). The Optional Live Tracker's true E2E encryption (I9) is a
  separate mechanism with a different key model and is out of scope here.

## Considered alternatives

### Option B — sealed boxes (asymmetric, `crypto_box_seal`)

**Rejected.** A sealed box encrypts to a Group *public* key and decrypts with the Group
*secret* key. But the **secret key still lives server-side** (the server both writes and
reads PII at rest), so asymmetry buys **no** confidentiality benefit over symmetric
secretbox — it only adds a keypair to generate, store, and wrap, and is slower. Sealed
boxes are the right tool when the encryptor must *not* be able to decrypt (e.g. I9's live
tracker, where the Driver encrypts to the Rider's public key and the server cannot read);
that is not the at-rest case. Symmetric secretbox is the minimal correct primitive for I1.

### Option C — one global key for all Groups (no per-Group key)

**Rejected.** It contradicts I1 ("a **per-Group** key") and collapses the blast radius:
a single key compromise would expose **every** Group's PII. Even though one install is one
Group *today*, the architecture keeps PII strictly per-Group, and the future federation /
multi-instance considerations (architecture "Future considerations") and simple
defense-in-depth all argue for per-Group isolation from day one. The `delegated_keys` table
is per-Group by design.

### Option D — store the KEK in an env var / Worker config instead of Secrets Store

**Rejected.** I1 names Cloudflare **Secrets Store** as the KEK's home, and
`forbidden-patterns` forbids hardcoded secrets / secrets in config. Secrets Store is the
audited, rotatable, first-party-bound home (ADR-0014); an env var is none of those.

## Consequences

### Positive

- **AC2 (`i1_addresses_encrypted`) and AC12 (bootstrap) become writable** — the key exists,
  has a generation point, and a defined storage shape.
- **Per-Group blast-radius containment** + the standard KEK-wraps-DEK envelope pattern,
  matching the architecture's `delegated_keys` + DO-cache design exactly.
- **Symmetric = simple and fast** — one 32-byte key, one AEAD call per field; no keypair
  management.
- **KEK rotation is cheap** (re-wrap only); the expensive Group-key rotation is isolated
  behind a deliberate, documented maintenance procedure.

### Negative / costs

- **The server can decrypt PII** (not E2E). Required by P3/I2 + I5, but it means the at-rest
  encryption defends against a *storage* breach (stolen DB / backups), **not** a compromised
  server tier. This is the intended threat model (I1 is about at-rest), stated plainly.
- **Nonce-uniqueness is a footgun** — a reused secretbox nonce is catastrophic. Mitigated by
  a **single core encryption function** that draws a fresh CSPRNG nonce per call and stores
  `nonce ‖ ciphertext`; no caller hand-rolls nonces. To be enforced by the implementing
  code + test.
- **Group-key rotation is deferred** — there is no automatic rotation in spec 008; a
  long-lived key relies on KEK protection until the rotation Workflow + runbook are built.
  Recorded as a follow-up, not pretended-done.

### Neutral / follow-ups

- **Spec 008 owns:** the `delegated_keys` migration (+ the `groups`-row bootstrap), the
  `address_encrypted` / `name_encrypted` columns, the core encryption function, and the
  `i1_addresses_encrypted` test.
- **`docs/runbooks/` key-management runbook** (P12) — KEK access, KEK rotation (re-wrap),
  Group-key rotation (re-encrypt Workflow, deferred).
- **`docs/stack-matrix.md`** — the dryoc row's "sealed-box/secretbox" hedge is resolved to
  **secretbox for field-level PII at rest** (sealed boxes remain reserved for I9's live
  tracker).
- **`DEFERRED.md`** — the spec-008-tagged "per-Group sealed-box PII encryption (I1)" + "per-
  Group key/KEK columns" items are now governed by this ADR.

## References

- `docs/privacy-invariants.md` I1 (per-Group key + KEK; `from_db(bytes, GroupKey)`), I2/I9
- `.specify/memory/constitution.md` P2, P3, P12
- `docs/architecture.md` §5 (DO Per-Group key, decrypted via Secrets Store on init), §6
  (`delegated_keys` table), trust-boundary "DO → Secrets Store"
- `docs/stack-matrix.md` (dryoc — "per-Group sealed-box/secretbox PII encryption (I1) …
  spec 008"); ADR-0014 (Secrets Store); ADR-0021 (`RngSecretSource` / no ambient randomness)
- spec 008 (`specs/008-admin-member-management/spec.md`, AC2/AC3/AC12)
- dryoc `crypto_secretbox` API to be version-pinned via `docs-researcher` at implementation
  (lock = ground truth)
