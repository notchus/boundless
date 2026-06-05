//! Tests for the production [`RngSecretSource`] (ADR-0021): opaque-random credentials from an
//! injected CSPRNG. Driven by a **seeded** `ChaCha20Rng` so the suite is reproducible (the Worker
//! injects a getrandom-backed CSPRNG instead — same trait, no code change). The cross-check that a
//! minted token verifies against the `core::crypto` at-rest hash ties the mint side (this slice) to
//! the verify side (the deferred store lookup) end-to-end at the unit level.

use boundless_crypto::{
    access_token_hash, access_token_matches, admin_invitation_token_hash,
    admin_invitation_token_matches, refresh_token_hash, refresh_token_matches, HmacKey,
};
use boundless_domain::{AccessToken, AdminInvitationToken, RefreshToken};
use boundless_server_core::{RngSecretSource, SecretSource};
use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;

fn key() -> HmacKey {
    HmacKey::from_bytes([0x42; 32])
}

fn seeded(seed: u64) -> RngSecretSource<ChaCha20Rng> {
    RngSecretSource::new(ChaCha20Rng::seed_from_u64(seed))
}

#[test]
fn deterministic_given_the_same_seed() {
    // Same seed ⇒ identical credential sequence (so a test is reproducible). The Worker injects a
    // real CSPRNG instead; this only proves the generator is a pure function of its injected RNG.
    let mut a = seeded(7);
    let mut b = seeded(7);
    for _ in 0..5 {
        assert_eq!(
            a.fresh_refresh().expose_secret(),
            b.fresh_refresh().expose_secret()
        );
        assert_eq!(
            a.fresh_access().expose_secret(),
            b.fresh_access().expose_secret()
        );
        assert_eq!(
            a.fresh_recovery_code().expose_secret(),
            b.fresh_recovery_code().expose_secret()
        );
    }
}

#[test]
fn different_seeds_diverge() {
    let mut a = seeded(1);
    let mut b = seeded(2);
    assert_ne!(
        a.fresh_access().expose_secret(),
        b.fresh_access().expose_secret(),
        "distinct seeds must produce distinct tokens"
    );
}

#[test]
fn fresh_secrets_are_distinct_across_calls() {
    // Each draw must be fresh (a rotated credential must differ from the prior one).
    let mut s = seeded(42);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..64 {
        assert!(
            seen.insert(s.fresh_refresh().expose_secret().to_owned()),
            "refresh tokens must not repeat"
        );
        assert!(
            seen.insert(s.fresh_access().expose_secret().to_owned()),
            "access tokens must not repeat (and must differ from refresh tokens)"
        );
    }
}

#[test]
fn opaque_tokens_are_256_bit_lowercase_hex() {
    // 32 bytes → 64 lowercase-hex chars; opaque (the client never parses it).
    let mut s = seeded(99);
    for token in [
        s.fresh_access().expose_secret(),
        s.fresh_refresh().expose_secret(),
    ] {
        assert_eq!(token.len(), 64, "256-bit token = 64 hex chars");
        assert!(
            token
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "token must be lowercase hex: {token}"
        );
    }
}

#[test]
fn minted_access_token_verifies_against_its_at_rest_hash() {
    // The mint side (this slice) and the verify side (the deferred store lookup) agree: a minted
    // access token hashes to a value its own constant-time matcher accepts, and a different token
    // does not — exactly what the server's per-request verify lookup will rely on (ADR-0021).
    let mut s = seeded(5);
    let token = s.fresh_access();
    let stored = access_token_hash(&key(), &token);
    assert!(access_token_matches(&key(), &token, &stored));
    assert!(!access_token_matches(
        &key(),
        &AccessToken::new("not-the-minted-token"),
        &stored
    ));
}

#[test]
fn minted_refresh_token_verifies_against_its_at_rest_hash() {
    let mut s = seeded(6);
    let token = s.fresh_refresh();
    let stored = refresh_token_hash(&key(), &token);
    assert!(refresh_token_matches(&key(), &token, &stored));
    assert!(!refresh_token_matches(
        &key(),
        &RefreshToken::new("not-the-minted-token"),
        &stored
    ));
}

#[test]
fn minted_admin_invitation_is_opaque_and_verifies_against_its_at_rest_hash() {
    // T08 / AC16: a minted Admin invitation is a 256-bit opaque token (the recipient never parses
    // it) that hashes to a value its own constant-time matcher accepts — exactly what the deferred
    // consume-on-register path (T09) relies on.
    let mut s = seeded(8);
    let token = s.fresh_admin_invitation();
    assert_eq!(
        token.expose_secret().len(),
        64,
        "256-bit token = 64 hex chars"
    );
    assert!(token
        .expose_secret()
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));

    let stored = admin_invitation_token_hash(&key(), &token);
    assert!(admin_invitation_token_matches(&key(), &token, &stored));
    assert!(!admin_invitation_token_matches(
        &key(),
        &AdminInvitationToken::new("not-the-minted-token"),
        &stored
    ));
}
