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

use boundless_crypto::{GroupKey, Nonce, KEY_LEN, NONCE_LEN};
use boundless_domain::{
    AccessToken, AdminInvitationToken, OnboardingCode, RecoveryCode, RefreshToken,
};
use rand_core::{CryptoRng, RngCore};
use zeroize::Zeroizing;

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

    fn fresh_admin_invitation(&mut self) -> AdminInvitationToken {
        // 256-bit opaque registration capability (ADR-0015 / AC16): the recipient never parses it
        // (it is echoed in the registration URL and hashed at rest), so hex is fine — same draw as
        // the tokens. Single-use + server-TTL are enforced by the store/orchestration, not here.
        AdminInvitationToken::new(self.fresh_opaque())
    }

    fn fresh_onboarding_code(&mut self) -> OnboardingCode {
        // INTERIM FORMAT — confirm at spec-008 issuance UX. Like the Recovery Code, an Onboarding
        // Code's human-facing format (length, grouping, alphabet — a helper may read it aloud) is a
        // UX decision that does not exist yet. Here it is the same high-entropy opaque draw as the
        // tokens: the *security* property (256-bit, single-use, server-TTL, rate-limited, hashed at
        // rest via `onboarding_code_hash`) is settled; only the *display* format is spec-008's to
        // refine. Tracked in DEFERRED.md.
        OnboardingCode::new(self.fresh_opaque())
    }

    fn fresh_nonce(&mut self) -> Nonce {
        // A 24-byte secretbox nonce drawn straight from the injected CSPRNG (NOT hex-encoded — the
        // nonce is raw bytes prepended to the ciphertext). Uniqueness is the only guard against the
        // XSalsa20-Poly1305 nonce-reuse footgun (R1, ADR-0025), so it must be a fresh CSPRNG draw.
        let mut bytes = [0u8; NONCE_LEN];
        self.rng.fill_bytes(&mut bytes);
        Nonce::from_bytes(bytes)
    }

    fn fresh_group_key(&mut self) -> GroupKey {
        // 32 key bytes straight from the injected CSPRNG (NOT hex — the key is raw bytes, KEK-wrapped
        // at rest). `Zeroizing` wipes the *local* source buffer on drop; the `*bytes` move into
        // `GroupKey::from_bytes` is a by-`Copy` of `[u8; KEY_LEN]`, so a short-lived argument
        // temporary still holds the key until the frame unwinds (the unavoidable cost of moving an
        // array by value in safe Rust — same residual as `unwrap_group_key`). The load-bearing wipe
        // is `GroupKey`'s own `Drop` (R2, ADR-0025), which covers the bytes that live for the key's
        // lifetime; this `Zeroizing` is a best-effort wipe of the source draw. Unlike `fresh_nonce`,
        // the nonce is not secret — a key is, hence the wipe here at all.
        let mut bytes = Zeroizing::new([0u8; KEY_LEN]);
        self.rng.fill_bytes(&mut *bytes);
        GroupKey::from_bytes(*bytes)
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
    use super::{to_hex, RngSecretSource};
    use crate::ports::SecretSource;
    use proptest::prelude::*;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;
    use std::collections::HashSet;

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

    proptest! {
        /// The secretbox nonce-reuse footgun (R1, ADR-0025): every `fresh_nonce` draw must be
        /// distinct. Driving the production `RngSecretSource` over a seeded CSPRNG, 1000 consecutive
        /// nonces are all unique (a collision would be a catastrophic key-stream reuse).
        #[test]
        fn prop_secretbox_nonce_unique_across_calls(seed: u64) {
            let mut src = RngSecretSource::new(ChaCha20Rng::seed_from_u64(seed));
            let mut seen = HashSet::new();
            for _ in 0..1000 {
                let nonce = src.fresh_nonce();
                prop_assert!(
                    seen.insert(*nonce.as_bytes()),
                    "fresh_nonce collided within 1000 draws"
                );
            }
        }

        /// The *actual* R1 threat the docs name (ports.rs / secretbox.rs): a pooled, multi-isolate
        /// Worker fleet has no shared counter, so nonces drawn from **independent** `RngSecretSource`
        /// instances must not collide either. Two independently-seeded sources produce distinct first
        /// nonces and a collision-free combined stream — this would FAIL for a per-instance counter
        /// seeded at 0 (the forbidden deterministic pattern), which the single-stream test above would
        /// not catch.
        #[test]
        fn prop_secretbox_nonce_unique_across_isolates(seed_a: u64, seed_b: u64) {
            prop_assume!(seed_a != seed_b);
            let mut a = RngSecretSource::new(ChaCha20Rng::seed_from_u64(seed_a));
            let mut b = RngSecretSource::new(ChaCha20Rng::seed_from_u64(seed_b));

            let mut seen = HashSet::new();
            let first_a = *a.fresh_nonce().as_bytes();
            let first_b = *b.fresh_nonce().as_bytes();
            // Distinct seeds → distinct first nonces (a counter-from-0 impl would tie here).
            prop_assert_ne!(first_a, first_b);
            prop_assert!(seen.insert(first_a));
            prop_assert!(seen.insert(first_b));
            // The two independent streams never collide with each other.
            for _ in 0..500 {
                prop_assert!(seen.insert(*a.fresh_nonce().as_bytes()), "cross-isolate nonce collision");
                prop_assert!(seen.insert(*b.fresh_nonce().as_bytes()), "cross-isolate nonce collision");
            }
        }
    }
}
