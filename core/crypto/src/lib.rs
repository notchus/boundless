//! `boundless-crypto` — single-source cryptography for Boundless (P4).
//!
//! Implemented in spec 001 **T03**:
//! - **Keyed hashing** ([`hashing`]) — HMAC-SHA256 phone-lookup hash and Onboarding/Recovery
//!   code-at-rest hash, with a constant-time verify (I3 / AC3). Via RustCrypto `hmac` + `sha2`
//!   (dryoc has no SHA-256 — see ADR-0018). **T07** adds the refresh-credential at-rest hash
//!   ([`refresh_token_hash`] / [`refresh_token_matches`], domain-separated) backing the
//!   session-lineage classification (ADR-0016 D2), and the access-token at-rest hash
//!   ([`access_token_hash`] / [`access_token_matches`]) backing the opaque-bearer store lookup
//!   (ADR-0021). **T08** adds the Admin registration-invitation at-rest hash
//!   ([`admin_invitation_token_hash`] / [`admin_invitation_token_matches`], ADR-0015 / AC16).
//!   Each artifact carries a distinct domain tag, so a hash for one role never verifies in
//!   another.
//! - **Manifest verification** ([`manifest`]) — Ed25519 detached-signature verification and
//!   the ADR-0014 tiered fallback (verify-fail → cached → bundled) + lower-version-ignore
//!   (AC10 / O2). Via `dryoc` (libsodium), the sole signature implementation across the system.
//!
//! Added in spec 008 **T02**:
//! - **Field-level PII encryption** ([`secretbox`]) — per-Group `crypto_secretbox`
//!   (XSalsa20-Poly1305) encryption of address/name at rest ([`encrypt_field`]/[`decrypt_field`]),
//!   the per-Group [`GroupKey`] (the DEK) and the [`Kek`] that KEK-wraps it
//!   ([`wrap_group_key`]/[`unwrap_group_key`]), both unloggable + zeroized on drop (I1, ADR-0025).
//!
//! Hashing + manifest verification are **deterministic** — no ambient randomness is *used*. The
//! secretbox primitive does not draw randomness either: [`encrypt_field`] takes its [`Nonce`] **as a
//! parameter** from the injected CSPRNG (`SecretSource::fresh_nonce`, ADR-0021), so the crate stays
//! randomness-free. dryoc still transitively pulls `rand`/`getrandom` (not feature-gated); on
//! `wasm32-unknown-unknown` (the Cloudflare Worker server + browser admin web) `getrandom`'s
//! `wasm_js` backend is enabled only to satisfy compilation (`core/crypto/Cargo.toml`). See ADR-0018
//! / ADR-0025 and the `DEFERRED.md` → Crypto register for what remains out of scope (code
//! generation/TTL/rate-limit, the workspace RNG-backend policy, Group-key rotation tooling).

mod hashing;
mod manifest;
mod secretbox;

pub use hashing::{
    access_token_hash, access_token_matches, admin_invitation_token_hash,
    admin_invitation_token_matches, onboarding_code_hash, onboarding_code_matches,
    phone_lookup_hash, phone_lookup_matches, recovery_code_hash, recovery_code_matches,
    refresh_token_hash, refresh_token_matches, AccessTokenHash, AdminInvitationTokenHash, CodeHash,
    HmacKey, PhoneLookupHash, RefreshTokenHash, HASH_LEN, HMAC_KEY_LEN,
};
pub use manifest::{
    canonical_manifest_bytes, decide_manifest, verify_manifest_signature, FetchedManifest,
    ManifestCache, ManifestDecision, ManifestErrorCode, ManifestResolution, Signature,
    VerifyingKey, PUBLIC_KEY_LEN, SIGNATURE_LEN,
};
pub use secretbox::{
    decrypt_field, encrypt_field, unwrap_group_key, wrap_group_key, GroupKey, Kek, Nonce,
    SecretboxError, KEY_LEN, MAC_LEN, NONCE_LEN,
};
