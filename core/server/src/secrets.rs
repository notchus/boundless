//! The production [`SecretSource`]: fresh credentials drawn from an **injected** CSPRNG.
//!
//! The core forbids ambient randomness, so the generator is injected as a type parameter
//! (`R: RngCore + CryptoRng`) exactly like the [`Clock`](boundless_auth::Clock) is injected: the
//! deployable Worker supplies a getrandom-backed CSPRNG (T07-shell-B), the host tests supply a
//! seeded [`ChaCha20Rng`](https://docs.rs/rand_chacha) for reproducibility. Because the only
//! `rand_core` surface used here is the *traits* (`default-features = false`, no `getrandom`), this
//! crate stays `wasm32`-safe and introduces no ambient randomness of its own.
//!
//! **Token shape (ADR-0021).** Access and refresh credentials are **opaque-random** 256-bit draws,
//! lowercase-hex-encoded into the tainted newtype. The client treats them as opaque strings and
//! never decodes them, so the encoding is an internal detail — hex is chosen for being
//! dependency-free and trivially `wasm32`-safe (the token is hashed at rest + echoed on the wire,
//! never parsed, so base64-vs-hex is immaterial). Verification is a constant-time keyed-HMAC store
//! lookup (`boundless_crypto::access_token_hash` / `refresh_token_hash`), not parsing.

use boundless_domain::{AccessToken, RecoveryCode, RefreshToken};
use rand_core::{CryptoRng, RngCore};

use crate::ports::SecretSource;

/// Bytes of entropy per generated secret: 256-bit, matching the refresh credential (ADR-0016 D2).
const SECRET_BYTES: usize = 32;

/// A production [`SecretSource`] backed by an **injected** CSPRNG `R`.
///
/// The `R: RngCore + CryptoRng` bound is the contract that the injected generator is
/// cryptographically secure (the `CryptoRng` marker). No ambient randomness is introduced into the
/// core — the generator is owned by the caller (the Worker / the test), mirroring the injected
/// `Clock`. Construct with [`RngSecretSource::new`].
pub struct RngSecretSource<R> {
    rng: R,
}

impl<R: RngCore + CryptoRng> RngSecretSource<R> {
    /// Wrap an injected CSPRNG. In production this is a getrandom-backed generator the Worker
    /// supplies (T07-shell-B); in tests a seeded `ChaCha20Rng` for reproducibility.
    pub fn new(rng: R) -> Self {
        Self { rng }
    }

    /// Draw [`SECRET_BYTES`] of entropy and lowercase-hex-encode them into an opaque token string.
    fn fresh_opaque(&mut self) -> String {
        let mut bytes = [0u8; SECRET_BYTES];
        self.rng.fill_bytes(&mut bytes);
        to_hex(&bytes)
    }
}

impl<R: RngCore + CryptoRng> SecretSource for RngSecretSource<R> {
    fn fresh_refresh(&mut self) -> RefreshToken {
        RefreshToken::new(self.fresh_opaque())
    }

    fn fresh_access(&mut self) -> AccessToken {
        AccessToken::new(self.fresh_opaque())
    }

    fn fresh_recovery_code(&mut self) -> RecoveryCode {
        // INTERIM FORMAT — confirm at spec-008 issuance. A driver-typed Recovery Code's human-facing
        // format (length, grouping, alphabet) is an issuance/UX decision that does not exist yet.
        // Here it is the same high-entropy opaque draw as the tokens: the *security* property
        // (256-bit, single-use, rotated on use, hashed at rest via `recovery_code_hash`) is settled;
        // only the *display* format is spec-008's to refine. Tracked in DEFERRED.md.
        RecoveryCode::new(self.fresh_opaque())
    }
}

/// Lowercase-hex-encode bytes (dependency-free; the token is opaque, so the encoding is immaterial).
fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::to_hex;

    #[test]
    fn to_hex_is_lossless_full_width_lowercase() {
        // A hand-rolled encoder on secret material: pin it against a fixed vector so a future
        // `>> 4` / nibble-order typo, truncation, or alphabet slip fails here (sec-audit F7).
        assert_eq!(to_hex(&[0x00, 0xff, 0x10, 0x0a, 0xa0]), "00ff100aa0");
        assert_eq!(to_hex(&[]), "");
        assert_eq!(to_hex(&[0x42]), "42");
        // Every byte value round-trips to exactly two lowercase-hex chars (no width loss / bias).
        let all: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
        let hex = to_hex(&all);
        assert_eq!(hex.len(), 512);
        assert!(hex
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert!(hex.starts_with("000102") && hex.ends_with("fdfeff"));
    }
}
