# ADR-0018: Keyed-hash algorithm — HMAC-SHA256 via RustCrypto (dryoc lacks SHA-256)

- **Status:** Accepted
- **Date:** 2026-06-04
- **Author:** notch
- **Deciders:** notch
- **Relates to:** I3, AC3, ADR-0014, ADR-0016, ADR-0017; spec 001 task T03

## Context

Privacy-invariant **I3** specifies that a member's phone number is stored as a
`phone_lookup_hash` computed with **HMAC-SHA256** keyed by a per-instance secret, compared
in constant time. Spec 001 §B and **AC3** repeat "HMAC-SHA256." Task **T03** (`core::crypto`)
implements this, plus the Onboarding/Recovery **code-at-rest** hash.

The project had committed to **`dryoc` 0.8.0** as its single crypto library (ADR-0014 for the
Ed25519 manifest signature; ADR-0017 for the broader crypto surface), and `docs/stack-matrix.md`
even stated that dryoc would do "the constant-time HMAC-SHA256 phone-lookup hash."

While implementing T03 we verified dryoc 0.8.0's actual API against its docs/source
(2026-06-04, via `docs-researcher` + direct docs.rs inspection):

- `dryoc::classic::crypto_auth` is documented verbatim as **HMAC-SHA512-256** (libsodium's
  default), 32-byte output — **not** HMAC-SHA256.
- `dryoc::classic::crypto_hash` provides **SHA-512 only** — there is no SHA-256.

So **dryoc cannot produce HMAC-SHA256**, nor can it be built from dryoc's primitives. The
prior stack-matrix claim was factually wrong, and I3-as-written cannot be satisfied by
"dryoc only." A decision was forced: change the invariant, or change the library set.

## Decision

**Keep I3 exactly as written (HMAC-SHA256). Implement the keyed phone/code hash with the
RustCrypto crates `hmac` + `sha2`. `dryoc` remains the sole Ed25519 *signature*
implementation.**

- `core::crypto` uses `Hmac<Sha256>` (constant-time `verify_slice`) for `phone_lookup_hash`,
  `onboarding_code_hash`, and `recovery_code_hash`, each with a distinct domain-separation tag.
- `dryoc` continues to own the **manifest signature** primitive (`crypto_sign_*`,
  Ed25519) — the actual subject of ADR-0014's "single crypto implementation across the
  system," which is about that primitive being **byte-identical on client and server**.
- **I3, the constitution, and the privacy-invariants doc are NOT amended.** This ADR is a
  decision record, not an invariant change. (`docs/stack-matrix.md` is corrected to drop the
  inaccurate "dryoc does HMAC-SHA256" line and to add the `hmac`/`sha2` rows.)

### Rationale

1. A privacy invariant should drive the implementation, not be bent to a library's feature
   gap. Amending a numbered invariant to rename its algorithm sets a poor precedent.
2. The phone/code keyed hash is a **server-side-only** primitive — clients never compute it
   (only the manifest *verification* runs client-side). So there is **no cross-platform
   parity (P4) reason** it must be the same crate as the signature primitive; "single crypto
   impl" is preserved where it actually matters (the signature, still dryoc-only).
3. `hmac` + `sha2` are the de-facto-standard, audited, pure-Rust RustCrypto crates:
   `no_std`-capable, **wasm32-safe**, **no `getrandom`** (HMAC/SHA are deterministic),
   MIT OR Apache-2.0. `Mac::verify_slice` gives constant-time comparison for free, so no
   separate `subtle`/`sodium_memcmp` dependency is needed.

### Secondary decision: getrandom `wasm_js` backend on wasm32

`dryoc` 0.8.0 has an **un-feature-gated** dependency on `rand` → `getrandom 0.4`, and
`getrandom 0.4` `compile_error!`s on `wasm32-unknown-unknown` unless a backend is chosen.
Because `core/crypto` must build for **wasm32** (the Cloudflare Worker server compiles via
workers-rs; the browser admin web via wasm-bindgen), we enable `getrandom`'s **`wasm_js`**
backend for wasm32 targets in `core/crypto/Cargo.toml`. This is **compile-enablement only**:
T03's code calls **zero** randomness (deterministic HMAC-verify + Ed25519-verify), and
`wasm_js` (Web Crypto `crypto.getRandomValues`) is the correct backend for both Workers and
browsers when randomness *is* eventually needed (server-side code/nonce generation).

The workspace-wide RNG-backend policy — and whether to forbid ambient randomness outright via
a custom erroring `getrandom` backend until the server genuinely needs it — is deferred
(`DEFERRED.md` → Crypto).

## Considered alternatives

### Option B — Amend I3 to HMAC-SHA512-256 (use dryoc's `crypto_auth`)

**Rejected.** Would keep a single crypto crate and give constant-time verify for free, but
requires amending a numbered privacy invariant (+ an ADR + edits to `privacy-invariants.md`,
spec 001, and the stack-matrix). HMAC-SHA512-256 is neutral-to-stronger, so this was a
legitimate option — but bending the invariant to fit the library is the wrong direction for a
privacy-first, constitution-led project. The owner chose to keep the invariant.

### Option C — Build HMAC-SHA256 from dryoc primitives

**Infeasible.** dryoc 0.8.0 exposes no SHA-256 at all (`crypto_hash` is SHA-512 only), so
HMAC-SHA256 cannot be constructed from it.

## Consequences

### Positive

- I3 is honored verbatim; no invariant amendment.
- Constant-time keyed-hash verification with no extra `subtle` dependency.
- `core/crypto` builds for both native and wasm32; deterministic, no randomness used.

### Negative / costs

- **Two crypto crates in `core`**: `dryoc` (Ed25519 signatures, future I1 sealed-box) and
  RustCrypto `hmac`+`sha2` (keyed hashing). Slightly larger audit surface — acceptable, as
  HMAC and Ed25519 are distinct primitives and both crates are gold-standard and wasm-safe.
- `getrandom` is compiled (via dryoc→rand) but unused on T03's paths; the `wasm_js` backend
  is enabled only so wasm32 compiles. The permanent RNG-backend policy is a follow-up.

### Neutral / follow-ups

- `docs/stack-matrix.md` updated: dryoc row corrected; `hmac`/`sha2`/`base64` rows added.
- The phone-number **normalization** (E.164) that precedes hashing is the caller's
  responsibility (`core::auth`, T04); `core::crypto` hashes the bytes it is given.
- Code **generation / TTL / rate-limit / single-use** semantics are T04/T07, not T03.

## References

- `docs/privacy-invariants.md` I3 · `specs/001-onboarding/spec.md` AC3
- ADR-0014 (server-driven config / manifest signing), ADR-0016 (auth model), ADR-0017 (admin WebAuthn)
- dryoc 0.8.0 docs: `classic::crypto_auth` (HMAC-SHA512-256), `classic::crypto_hash` (SHA-512), `classic::crypto_sign` (Ed25519) — verified 2026-06-04
- [RustCrypto `hmac`](https://docs.rs/hmac) · [`sha2`](https://docs.rs/sha2) · [getrandom WebAssembly support](https://docs.rs/getrandom/0.4.2/#webassembly-support)
