# ADR-0023: Auth requests carry the plaintext phone (over TLS), not a client-computed `phone_lookup_hash`

- **Status:** Accepted
- **Date:** 2026-06-07
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0018 (keyed HMAC-SHA256); I3 / I5 / P2 / P4; spec 001 §C, AC3, AC7; tasks T03/T04/T07 (T07-shell-B), T10
- **Resolves:** the "OpenAPI `SignInRequest` ↔ I3 contract defect" recorded in `DEFERRED.md` (T07 register) and flagged in `server/src/runtime/mod.rs`'s module header.

## Context

The frozen auth contract said the **client** computes and sends `phone_lookup_hash`:

- spec 001 §C step 2: "enter the member's phone number; the client sends `phone_lookup_hash` to the server."
- spec 001 AC3: "the plaintext phone is never sent in an auth lookup beyond the hashing boundary."
- spec 001 `plan.md` §4: the `POST /api/auth/{signin,recovery/rebind}` endpoint sketches show a
  `{ phone_lookup_hash }` request body.
- `api/openapi.yaml`: `SignInRequest` / `BindDeviceRequest` / `RecoveryRebindRequest` each `require` a
  `phone_lookup_hash` field, described as "computed by the client (I3/AC3). The plaintext phone never
  crosses the wire."
- `fixtures/compat/{current,n_minus_1,n_minus_2}.json`: request `body` carries `phone_lookup_hash`;
  `fixtures/auth/README.md` narrates the (now-false) "plaintext phone never appears" framing.

This is **impossible as specified.** Per **I3** (and ADR-0018), `phone_lookup_hash` is **HMAC-SHA256
keyed by a per-instance server secret** (`boundless_crypto::phone_lookup_hash`, `HmacKey` from Secrets
Store). A client cannot hold that secret, so a client cannot compute the hash. The contract is
internally inconsistent with the invariant it cites.

The **implementation already resolves the contradiction the only way it can** — server-side:

- `core::server::sign_in(SignInRequest { phone: PhoneNumber, .. })` takes the **raw (normalized)
  phone** and hashes it server-side (`core/server/src/service.rs`); there is no client-side hashing
  anywhere in the tree.
- AC3's own named test `i3_phone_lookup_constant_time` (`core/crypto/tests/invariants.rs`) exercises the
  **server-side** keyed hash + constant-time compare.
- The deployable Worker skeleton already takes `phone` on the wire and hashes server-side
  (`server/src/runtime/mod.rs`), its header noting the contract "needs an ADR."

So the documents are stale; the code is correct and internally consistent. This ADR aligns the documents
to the code.

The reconciliation has two candidate directions, because the spec was trying to satisfy two goals that a
**single** keyed hash cannot both meet: (G1, I3) the at-rest lookup hash is keyed by a server secret so a
stolen DB cannot be brute-forced back to phone numbers; (G2, the spec's wire wish) the raw phone does not
transit the wire. A client-computed lookup hash is fundamentally incompatible with a server-held key.

## Decision

**Auth request bodies carry the member's plaintext phone number** (E.164, `phone`), transported under
TLS. The **server** normalizes it (`core::server::normalize_phone`, the P4 single-source canonicalizer),
computes the I3 keyed `phone_lookup_hash` with the per-instance secret, performs the constant-time
lookup, and **drops the plaintext** — which is a tainted `PhoneNumber` (no `Debug`/`Display`/`Serialize`)
so it is never logged (P2) and is never persisted in plaintext (stored only as the I3 keyed hash, and
`phone_encrypted` for audit-logged Admin display per I5).

Concretely:

- `api/openapi.yaml` `SignInRequest` / `BindDeviceRequest` / `RecoveryRebindRequest`: the
  `phone_lookup_hash` field becomes `phone` (E.164 string), in both `required` and `properties`.
- spec 001 §C step 2 and AC3 are reworded to the server-side hashing model (AC3's checkbox state and its
  named test `i3_phone_lookup_constant_time` are unchanged — the test already proves the server-side
  property); `plan.md` §4's endpoint sketches show `{ phone }`.
- The `fixtures/compat/**` illustrative request bodies carry `phone`; `fixtures/auth/README.md`'s
  narration is corrected; the Worker's `server/src/runtime/mod.rs` contract note is marked resolved.
- A regression guard (`web/tests/contract/api-contract.test.ts`) asserts each auth request requires
  `phone` and exposes no `phone_lookup_hash`.

This keeps G1 (the at-rest hash stays server-keyed — unchanged) and drops G2 (the raw phone transits the
wire, protected by TLS). I3, P2, P4, and ADR-0018 are unchanged: this ADR amends only the **wire/spec
description**, not the storage invariant or the crypto.

## Considered alternatives

### (B) Client computes a keyless pre-hash; server re-hashes with the per-instance key

The client normalizes the phone and sends `H1 = SHA-256(normalized_phone)`; the server computes the
stored lookup hash as `HMAC_key(H1)` (issuance, spec 008, must pre-hash identically). This preserves G2
("raw phone never on the wire") while keeping G1 (at-rest still keyed).

**Rejected.** Three decisive costs for an illusory gain:

1. **The "privacy" gain is largely illusory.** `H1` is an **unsalted** hash of a low-entropy value (a
   phone number is ~10 digits ≈ 10^10 candidates) — trivially brute-forced if it ever leaks (a log line,
   a captured request). So `H1` on the wire is a reversible pseudonym of the phone, only marginally
   better than the phone itself, while creating a false sense of protection. TLS already protects the
   phone in transit; the controls that actually matter are at-rest (G1/I3 + I5 encryption) and in-logs
   (P2 tainted type) — both fully honored by Option A.
2. **It violates P4.** `normalize_phone` is deliberately the **server-side single source** of
   canonicalization (so the number Sarah types at issuance and the helper types at onboarding hash
   identically). Option B forces byte-identical normalization onto all five clients (iOS, Android × 2,
   web, …); any divergence silently breaks lookups.
3. **It is larger and touches crypto.** It changes the I3 hash *input* (HMAC over a pre-hash, not the
   phone) → amends ADR-0018 + `core/crypto` + spec-008 issuance, for no real benefit.

### Do nothing / keep the defect recorded

**Rejected.** The contradiction is a hard blocker for the T10-shell OpenAPI **request** codegen (a
generated client would send a base64 hash the server cannot use → 400) and for the T07-shell-B PgAuthStore
sign-in lookup. Resolving it now — while the contract is small and **no shipped client consumes these
request envelopes yet** (the iOS/Android `OnboardingNetworking` are stubs) — is strictly cheaper than later.

## Consequences

### Positive

- The contract, spec, fixtures, and implementation agree. The T10-shell request codegen and the
  T07-shell-B sign-in lookup are unblocked.
- One canonicalization source preserved (P4): `normalize_phone` stays server-side.
- No change to I3, P2, ADR-0018, or any crypto — the storage/redaction posture is exactly as built.

### Negative / accepted

- The raw phone (PII) transits the wire (under TLS). This is the standard posture for phone-based auth
  and is consistent with I3 (a storage invariant) and P2 (a no-logs invariant) — neither constrains TLS
  transit. The server must continue to treat the received phone as a tainted `PhoneNumber`: hash, use,
  drop; never log; never persist in plaintext.

### Follow-ups (already tracked in `DEFERRED.md`)

- The deployable Worker's sign-in lookup over `PgAuthStore`/Hyperdrive (T07-shell-B) and the OpenAPI
  request codegen (T10-shell) consume this now-consistent contract.
- The bind-device / recovery-rebind request paths carry `phone` for the same reason (this ADR fixes all
  three auth requests, not just sign-in).

## References

- `core/crypto/src/hashing.rs` (`phone_lookup_hash`, server-keyed `HmacKey`) · `core/crypto/tests/invariants.rs` (`i3_phone_lookup_constant_time`)
- `core/server/src/service.rs` (`sign_in` takes `PhoneNumber`) · `core/server/src/phone.rs` (`normalize_phone`, P4 single-source) · `server/src/runtime/mod.rs` (Worker skeleton; the flagged note)
- `api/openapi.yaml` (the three auth requests) · `server/tests/compat/replay.rs` (fixture `body` is illustrative — reads only `client_version`)
- ADR-0018 (keyed HMAC-SHA256) · `docs/privacy-invariants.md` I3/I5 · constitution P2/P4 · spec 001 §C, AC3, AC7
