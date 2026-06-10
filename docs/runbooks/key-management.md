# Runbook: per-Group encryption key management (KEK + Group key)

> The lifecycle of the keys behind **I1** (addresses and names encrypted at rest). Pinned by
> **ADR-0025**. This runbook covers: how the per-Group key is generated at Group bootstrap, where the
> **KEK** lives and who may reach it, how to rotate the KEK (cheap re-wrap), and the deferred
> procedure for rotating the Group key itself (expensive re-encrypt).
>
> **Threat model (state it, never assume it — ADR-0025 §scope):** this is field-level encryption
> **at rest**. It defends a **storage** breach (stolen DB, backups, a `delegated_keys` row) — the
> ciphertext is useless without the KEK→Group key. It does **not** make PII opaque to the running
> server: the server holds the KEK, unwraps the Group key, and **can** decrypt — by design, because
> matching must decrypt addresses into an ephemeral `MatchingContext` (P3/I2) and Admins read PII
> through audited endpoints (I5). The true-E2E mechanism (the Optional Live Tracker, I9) is separate
> and out of scope here.

---

## The envelope, in one picture

```
Cloudflare Secrets Store ──┐
   KEK (32-byte secretbox key, never leaves the server tier, never logged)
                           │ unwrap on DO init
                           ▼
delegated_keys.wrapped_key  ── secretbox(GroupKey, kek_nonce, KEK) = nonce ‖ ciphertext
                           │
                           ▼
   GroupKey (32-byte secretbox key, plaintext in DO memory ONLY, zeroized on drop/evict)
                           │ encrypt_field(plaintext, GroupKey, fresh_nonce) = nonce ‖ ciphertext
                           ▼
members.address_encrypted / members.name_encrypted   (bytea, at rest)
```

Two keys, two layers (the standard **KEK-wraps-DEK** envelope):

- **KEK** (Key-Encryption-Key) — one per install, held in **Cloudflare Secrets Store**. Wraps the
  Group key. Never persisted in the database, never in `[vars]`/`wrangler.toml`, never logged (P2).
- **Group key** (the data-encryption key, "DEK") — one **per Group** (ADR-0025 rejects a global
  key). Encrypts the PII fields. Persisted **only** KEK-wrapped, in `delegated_keys.wrapped_key`. The
  plaintext exists only in `GroupHub` Durable Object memory after an unwrap, and is zeroized on drop.
  The DO persists **no** key copy in its own storage — `delegated_keys.wrapped_key` is the **sole**
  at-rest home of the (wrapped) key. (`docs/architecture.md`'s "Per-Group key, encrypted at rest in DO
  storage" refers to that single wrapped row decrypted on init, **not** a second DO-storage key copy;
  there must never be one.)

---

## 1 — KEK access (Cloudflare Secrets Store)

- The KEK is created and stored as a **Cloudflare Secrets Store** secret and bound to the Worker as a
  Secrets-Store binding (`KEK`) — **not** an `env.var`/`[vars]` plaintext binding (ADR-0025 rejects
  that; `forbidden-patterns` forbids secrets in config). The HMAC key (I3) and the KEK are different
  secrets with different blast radii; do not conflate them.
- **Who may reach it:** only the Developer/operator, through Cloudflare's Secrets Store access
  controls. The agent never reads or writes it (Cloudflare MCP is read-only; mutations are the human
  `wrangler`/dashboard gate).
- **Boot fails closed without it:** the Worker (spec 008 T09) refuses to serve member-management if
  the `KEK` binding is absent — issuance can never silently fall back to storing plaintext.
- **Generate a KEK** (32 bytes, hex): `openssl rand -hex 32`, then store it via Secrets Store (the
  exact `wrangler`/dashboard command is pinned in the spec 008 T09 deploy notes — Secrets Store, not
  `wrangler secret put`, which is the `[vars]`-style path used for `HMAC_KEY`).

> **Committed-credential gate:** the `wrangler.toml` grep gate (DEFERRED.md → T09) is extended to
> assert no `KEK`/`GROUP_KEY` value ever lands in `server/wrangler.toml` or a `[vars]` block.

---

## 2 — Group-key generation (at Group bootstrap, once per install)

Boundless is single-tenant (one install = one Group). The Group and its key are created **once**, at
bootstrap, by the operator — **not** through a tenant-scoped API (a bootstrap endpoint would need the
still-deferred Developer hardware-key WebAuthn; ADR-0025 / plan §13.4 choose an operator-run script).

In one transaction the bootstrap:

1. draws **32 random bytes** for the Group key from the **injected CSPRNG** (`RngSecretSource` — no
   ambient randomness in the core, ADR-0021);
2. draws a **fresh 24-byte nonce** from the same CSPRNG and computes
   `wrapped_key = secretbox(group_key, nonce, KEK)` stored as `nonce ‖ ciphertext`;
3. writes the single `groups` row and the `delegated_keys` row (`group_id`, `wrapped_key`,
   `kek_version = 1`, audit columns). The **plaintext Group key is never written** — only the wrapped
   blob.

Issuance (spec 008) **fails closed** with `ADMIN_GROUP_KEY_MISSING` if no `delegated_keys` row exists
for the Group; it never stores an unencrypted address as a fallback (AC12, I1).

> **Status:** the bootstrap *decision* (mint → wrap → row shape) lands in core at **spec 008 T04**;
> the operator-run provisioning extension (analogous to `scripts/provision-neon.sh`) writes the rows.
> The `kek_version` column **is created by** migration **0009** (spec 008 T03 — not yet landed)
> specifically so a re-wrap is traceable.

---

## 3 — KEK rotation (cheap — re-wrap only)

**Cadence (recommended default, ADR-0025):** **annually**, and **immediately on any suspected KEK
compromise** (a leaked Secrets Store secret, an operator off-boarding, etc.).

A KEK rotation changes **only** the outer wrap. The plaintext Group key and **all** PII ciphertext are
unchanged — so it is O(number of Groups) `delegated_keys` rows, not O(PII rows).

Procedure (documented; the automated Workflow is **deferred/unbuilt** — see ADR-0025 / DEFERRED.md):

1. Provision **KEK′** (new) in Secrets Store alongside the current **KEK** (keep both reachable during
   the cutover).
2. For each `delegated_keys` row: `group_key = unwrap(wrapped_key, KEK)`;
   `wrapped_key′ = secretbox(group_key, fresh_nonce, KEK′)` (a **fresh CSPRNG nonce, never reused** —
   §4, the same discipline as §2's wrap and `encrypt_field`); `UPDATE … SET wrapped_key = wrapped_key′,
   kek_version = kek_version + 1` (the bump records that the row is now wrapped under the new KEK).
   The transient plaintext `group_key` lives in a `Zeroizing<…>` buffer and is wiped immediately.
3. Point the Worker's `KEK` binding at **KEK′**; evict the cached plaintext Group key from the
   `GroupHub` DO so the next unwrap uses the new wrap; verify a decrypt round-trips.
4. Retire **KEK** from Secrets Store once every row reports the new `kek_version`.

No client change, no member impact, no re-encryption of PII.

---

## 4 — Group-key rotation (expensive — re-encrypt, **deferred / unbuilt**)

Rotating the **Group key** itself (not just its wrap) requires re-encrypting **every** PII field under
the new key — a maintenance **Workflow** that streams `members` rows, decrypts each
`address_encrypted`/`name_encrypted` with the old key, re-encrypts with the new key + a fresh nonce,
and writes back, then re-wraps and stores the new Group key. This is **manual / runbook-driven, not
automatic**, and is **deferred** (no rotation trigger ships in spec 008 — ADR-0025 §rotation;
DEFERRED.md records the unbuilt Workflow).

Until it is built, the Group key is long-lived and the corpus relies on:

- **KEK protection** (§1) + cheap KEK rotation (§3) as the realistic recovery path for a *wrap*
  compromise, and
- the **nonce discipline** (a fresh CSPRNG nonce per `encrypt_field`, never reused — the catastrophic
  secretbox footgun) + **zeroize discipline** (`GroupKey`/`Kek` wiped on drop; the unwrapped key never
  persisted) as the controls that make a single long-lived key acceptable for v1 (ADR-0025 §residual
  risk).

When a true Group-key compromise is suspected, the only correct response is to build/run this
re-encrypt Workflow (and rotate the KEK in the same maintenance window).

---

## References

- **ADR-0025** — per-Group field-encryption key lifecycle (the decision this runbook operationalizes).
- ADR-0014 — Secrets Store as the secret home; server-driven config.
- ADR-0021 — `RngSecretSource` / no ambient randomness in the core (the nonce + key bytes are injected).
- `docs/privacy-invariants.md` — I1 (per-Group key + KEK), I2/P3 (plaintext only during matching), I9
  (the separate true-E2E live tracker, sealed boxes — *not* this key).
- `docs/architecture.md` §5–§6 — `delegated_keys` table; `GroupHub` DO "Per-Group key (decrypted via
  Secrets Store on init)".
- `docs/error-codes.md` — `ADMIN_GROUP_KEY_MISSING` (issuance fail-closed).
- Migrations — `server/migrations/0009_delegated_keys.*` (`wrapped_key`, `kek_version`) — **spec 008
  T03, not yet landed**.
