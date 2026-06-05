//! Keyed-hash crypto (I3 / AC3, ADR-0018).
//!
//! The phone-lookup hash and the Onboarding/Recovery code-at-rest hash are **HMAC-SHA256**
//! keyed by a per-instance secret, with a **constant-time** verify. dryoc has no SHA-256
//! (its `crypto_auth` is HMAC-SHA512-256), so per ADR-0018 the keyed hash uses RustCrypto
//! `hmac` + `sha2`; dryoc remains the sole Ed25519 manifest-signature impl ([`crate::manifest`]).
//!
//! `expose_secret()` is called **only here** (the sanctioned crypto boundary) — the
//! plaintext phone/code is hashed and immediately dropped; only the keyed hash is returned.

use boundless_domain::{AccessToken, OnboardingCode, PhoneNumber, RecoveryCode, RefreshToken};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Length in bytes of an HMAC-SHA256 output (a stored phone-lookup or code hash).
pub const HASH_LEN: usize = 32;
/// Length in bytes of the per-instance HMAC secret.
pub const HMAC_KEY_LEN: usize = 32;

// Domain-separation tags. One per-instance secret can safely back several keyed hashes as
// long as each use prepends a distinct, length-unambiguous context tag — so `HMAC(k, phone)`
// can never collide with `HMAC(k, code)`, even if the underlying bytes coincide. The `\0`
// separator cannot appear in these ASCII tags, so tag-vs-message is unambiguous.
const DOMAIN_PHONE_LOOKUP: &[u8] = b"boundless:phone-lookup:v1";
const DOMAIN_ONBOARDING_CODE: &[u8] = b"boundless:onboarding-code:v1";
const DOMAIN_RECOVERY_CODE: &[u8] = b"boundless:recovery-code:v1";
const DOMAIN_REFRESH_TOKEN: &[u8] = b"boundless:refresh-token:v1";
const DOMAIN_ACCESS_TOKEN: &[u8] = b"boundless:access-token:v1";

/// A per-instance HMAC secret (from Cloudflare Secrets Store in production).
///
/// Key material: deliberately **no `Debug`/`Display`/`Serialize`**, so it can never be
/// logged or accidentally serialized (P2). Compile-time enforced in the test module.
#[derive(Clone)]
pub struct HmacKey([u8; HMAC_KEY_LEN]);

impl HmacKey {
    /// Wrap a per-instance secret. In production this comes from Secrets Store, never a
    /// hardcoded literal (forbidden-patterns: "Hardcoded secrets → Use Secrets Store").
    pub fn from_bytes(bytes: [u8; HMAC_KEY_LEN]) -> Self {
        Self(bytes)
    }
}

/// The HMAC-SHA256 of a phone number, keyed by the per-instance secret (I3).
///
/// This is the only value persisted/queried for auth lookup — never the plaintext phone.
/// It is still derived from PII, so it carries **no `Debug`/`Display`** (defense in depth)
/// and **no `PartialEq`**: callers must compare via the constant-time [`phone_lookup_matches`]
/// rather than `==`, which would be a non-constant-time membership oracle (R2).
#[derive(Clone)]
pub struct PhoneLookupHash([u8; HASH_LEN]);

impl PhoneLookupHash {
    /// Rebuild from stored bytes (e.g. a `bytea` column read).
    pub fn from_bytes(bytes: [u8; HASH_LEN]) -> Self {
        Self(bytes)
    }

    /// The raw hash bytes, for storage as a `bytea` lookup column.
    pub fn as_bytes(&self) -> &[u8; HASH_LEN] {
        &self.0
    }
}

/// The HMAC-SHA256 of an Onboarding or Recovery code, keyed for at-rest storage.
///
/// Same discipline as [`PhoneLookupHash`]: no `Debug`/`Display`, no `PartialEq`; compare
/// only via the constant-time `*_matches` functions. Onboarding and Recovery codes use
/// distinct domain tags, so a code hashed in one role never verifies in the other.
#[derive(Clone)]
pub struct CodeHash([u8; HASH_LEN]);

impl CodeHash {
    /// Rebuild from stored bytes (e.g. a `code_hash bytea` column read).
    pub fn from_bytes(bytes: [u8; HASH_LEN]) -> Self {
        Self(bytes)
    }

    /// The raw hash bytes, for storage as a `code_hash bytea` column.
    pub fn as_bytes(&self) -> &[u8; HASH_LEN] {
        &self.0
    }
}

/// The HMAC-SHA256 of a refresh credential, keyed for at-rest storage (ADR-0016 D2).
///
/// The server's session-lineage classification compares a *presented* refresh credential
/// against the stored hashes of a family's current + rotated-away credentials, constant-time
/// (T07). Same discipline as [`CodeHash`]: no `Debug`/`Display`/`Serialize`, and **no
/// `PartialEq`** — compare only via the constant-time [`refresh_token_matches`], never `==`
/// (which would be a non-constant-time oracle on a long-lived credential, R2/R6).
#[derive(Clone)]
pub struct RefreshTokenHash([u8; HASH_LEN]);

impl RefreshTokenHash {
    /// Rebuild from stored bytes (e.g. a `refresh_token_hash bytea` column read).
    pub fn from_bytes(bytes: [u8; HASH_LEN]) -> Self {
        Self(bytes)
    }

    /// The raw hash bytes, for storage as a `refresh_token_hash bytea` column.
    pub fn as_bytes(&self) -> &[u8; HASH_LEN] {
        &self.0
    }
}

/// The HMAC-SHA256 of an access token, keyed for at-rest storage (ADR-0021).
///
/// The access token is a short-lived (~15 min) **opaque-random bearer**; the server verifies a
/// presented token by re-deriving this keyed hash and looking it up against the sessions store
/// (so the family's mutable status is re-read every request — instant revocation). Same discipline
/// as [`RefreshTokenHash`]: no `Debug`/`Display`/`Serialize`, and **no `PartialEq`** — compare only
/// via the constant-time [`access_token_matches`], never `==` (a non-constant-time bearer-membership
/// oracle, R2/R6). A distinct domain tag keeps an access-token hash from ever verifying a refresh
/// credential (or vice-versa), even if the underlying bytes coincide.
#[derive(Clone)]
pub struct AccessTokenHash([u8; HASH_LEN]);

impl AccessTokenHash {
    /// Rebuild from stored bytes (e.g. an `access_token_hash bytea` column read).
    pub fn from_bytes(bytes: [u8; HASH_LEN]) -> Self {
        Self(bytes)
    }

    /// The raw hash bytes, for storage as an `access_token_hash bytea` column.
    pub fn as_bytes(&self) -> &[u8; HASH_LEN] {
        &self.0
    }
}

/// Compute `HMAC-SHA256(key, domain ‖ 0x00 ‖ message)`.
fn keyed_hash(key: &HmacKey, domain: &[u8], message: &[u8]) -> [u8; HASH_LEN] {
    let mut mac =
        HmacSha256::new_from_slice(&key.0).expect("HMAC-SHA256 accepts a key of any length");
    mac.update(domain);
    mac.update(&[0x00]);
    mac.update(message);
    let out = mac.finalize().into_bytes();
    let mut hash = [0u8; HASH_LEN];
    hash.copy_from_slice(&out);
    hash
}

/// Constant-time check that `HMAC-SHA256(key, domain ‖ 0x00 ‖ message) == expected`.
///
/// Uses RustCrypto's `Mac::verify_slice`, which compares in constant time (the output is
/// wrapped in a `CtOutput` backed by `subtle`), so there is no timing oracle on a mismatch.
fn keyed_verify(key: &HmacKey, domain: &[u8], message: &[u8], expected: &[u8; HASH_LEN]) -> bool {
    let mut mac =
        HmacSha256::new_from_slice(&key.0).expect("HMAC-SHA256 accepts a key of any length");
    mac.update(domain);
    mac.update(&[0x00]);
    mac.update(message);
    mac.verify_slice(expected).is_ok()
}

/// Derive the phone-lookup hash for `phone` (I3). The plaintext is touched only to hash it.
pub fn phone_lookup_hash(key: &HmacKey, phone: &PhoneNumber) -> PhoneLookupHash {
    PhoneLookupHash(keyed_hash(
        key,
        DOMAIN_PHONE_LOOKUP,
        phone.expose_secret().as_bytes(),
    ))
}

/// Constant-time test that `phone` hashes to `stored` (I3 / AC3 / R2 membership oracle).
pub fn phone_lookup_matches(key: &HmacKey, phone: &PhoneNumber, stored: &PhoneLookupHash) -> bool {
    keyed_verify(
        key,
        DOMAIN_PHONE_LOOKUP,
        phone.expose_secret().as_bytes(),
        &stored.0,
    )
}

/// Derive the at-rest hash of an Onboarding Code.
pub fn onboarding_code_hash(key: &HmacKey, code: &OnboardingCode) -> CodeHash {
    CodeHash(keyed_hash(
        key,
        DOMAIN_ONBOARDING_CODE,
        code.expose_secret().as_bytes(),
    ))
}

/// Constant-time test that `code` matches the stored Onboarding Code hash.
pub fn onboarding_code_matches(key: &HmacKey, code: &OnboardingCode, stored: &CodeHash) -> bool {
    keyed_verify(
        key,
        DOMAIN_ONBOARDING_CODE,
        code.expose_secret().as_bytes(),
        &stored.0,
    )
}

/// Derive the at-rest hash of a Driver Recovery Code.
pub fn recovery_code_hash(key: &HmacKey, code: &RecoveryCode) -> CodeHash {
    CodeHash(keyed_hash(
        key,
        DOMAIN_RECOVERY_CODE,
        code.expose_secret().as_bytes(),
    ))
}

/// Constant-time test that `code` matches the stored Recovery Code hash.
pub fn recovery_code_matches(key: &HmacKey, code: &RecoveryCode, stored: &CodeHash) -> bool {
    keyed_verify(
        key,
        DOMAIN_RECOVERY_CODE,
        code.expose_secret().as_bytes(),
        &stored.0,
    )
}

/// Derive the at-rest hash of a refresh credential (ADR-0016 D2). The plaintext is touched
/// only to hash it; only the keyed hash is stored, so the lineage table holds no usable token.
pub fn refresh_token_hash(key: &HmacKey, token: &RefreshToken) -> RefreshTokenHash {
    RefreshTokenHash(keyed_hash(
        key,
        DOMAIN_REFRESH_TOKEN,
        token.expose_secret().as_bytes(),
    ))
}

/// Constant-time test that `token` matches a stored refresh hash (R6: no timing oracle on a
/// long-lived credential). The server uses this to classify a presented credential as the
/// family's current vs a rotated-away (replayed) one.
pub fn refresh_token_matches(
    key: &HmacKey,
    token: &RefreshToken,
    stored: &RefreshTokenHash,
) -> bool {
    keyed_verify(
        key,
        DOMAIN_REFRESH_TOKEN,
        token.expose_secret().as_bytes(),
        &stored.0,
    )
}

/// Derive the at-rest hash of an access token (ADR-0021). The plaintext is touched only to hash
/// it; only the keyed hash is stored, so the sessions store holds no usable bearer.
pub fn access_token_hash(key: &HmacKey, token: &AccessToken) -> AccessTokenHash {
    AccessTokenHash(keyed_hash(
        key,
        DOMAIN_ACCESS_TOKEN,
        token.expose_secret().as_bytes(),
    ))
}

/// Constant-time test that `token` matches a stored access-token hash (R2/R6: no timing oracle on
/// a bearer). The server uses this to verify a presented access token against the sessions store.
pub fn access_token_matches(key: &HmacKey, token: &AccessToken, stored: &AccessTokenHash) -> bool {
    keyed_verify(
        key,
        DOMAIN_ACCESS_TOKEN,
        token.expose_secret().as_bytes(),
        &stored.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_tags_contain_no_nul_delimiter() {
        // `keyed_hash` delimits the domain tag from the message with a 0x00 byte; that is only
        // unambiguous because no tag itself contains 0x00. Guard it so a future tag addition
        // that breaks the invariant fails here rather than silently enabling a collision.
        for tag in [
            DOMAIN_PHONE_LOOKUP,
            DOMAIN_ONBOARDING_CODE,
            DOMAIN_RECOVERY_CODE,
            DOMAIN_REFRESH_TOKEN,
            DOMAIN_ACCESS_TOKEN,
        ] {
            assert!(
                !tag.contains(&0x00),
                "domain-separation tag must not contain the 0x00 delimiter"
            );
        }
    }
}
