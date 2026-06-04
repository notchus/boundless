//! `boundless-crypto` — single-source cryptography for Boundless (P4).
//!
//! Implemented in spec 001 **T03**:
//! - **Keyed hashing** ([`hashing`]) — HMAC-SHA256 phone-lookup hash and Onboarding/Recovery
//!   code-at-rest hash, with a constant-time verify (I3 / AC3). Via RustCrypto `hmac` + `sha2`
//!   (dryoc has no SHA-256 — see ADR-0018).
//! - **Manifest verification** ([`manifest`]) — Ed25519 detached-signature verification and
//!   the ADR-0014 tiered fallback (verify-fail → cached → bundled) + lower-version-ignore
//!   (AC10 / O2). Via `dryoc` (libsodium), the sole signature implementation across the system.
//!
//! All operations here are **deterministic** — no ambient randomness is *used*. dryoc
//! transitively pulls `rand`/`getrandom` (not feature-gated); on `wasm32-unknown-unknown`
//! (the Cloudflare Worker server + browser admin web) `getrandom`'s `wasm_js` backend is
//! enabled only to satisfy compilation (`core/crypto/Cargo.toml`). See ADR-0018 and the
//! `DEFERRED.md` → Crypto register for what is intentionally **out of scope** of T03
//! (per-Group sealed-box/secretbox PII encryption for I1, code generation/TTL/rate-limit,
//! key zeroization, the workspace RNG-backend policy).

mod hashing;
mod manifest;

pub use hashing::{
    onboarding_code_hash, onboarding_code_matches, phone_lookup_hash, phone_lookup_matches,
    recovery_code_hash, recovery_code_matches, CodeHash, HmacKey, PhoneLookupHash, HASH_LEN,
    HMAC_KEY_LEN,
};
pub use manifest::{
    canonical_manifest_bytes, decide_manifest, verify_manifest_signature, FetchedManifest,
    ManifestCache, ManifestDecision, ManifestErrorCode, ManifestResolution, Signature,
    VerifyingKey, PUBLIC_KEY_LEN, SIGNATURE_LEN,
};
