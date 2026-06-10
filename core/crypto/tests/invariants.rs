//! Privacy/operational invariant tests for `core::crypto` (spec 001 T03).
//!
//! - **I3 / AC3** — phone-lookup HMAC-SHA256 + constant-time compare.
//! - **AC10 / O2** — manifest Ed25519 verification + ADR-0014 tiered fallback.
//!
//! The manifest tests replay the golden envelopes in `fixtures/manifest/**`. The valid
//! signature vector in `verify_ok.json` is generated **here** (T03 owns crypto) from a
//! documented, checked-in seed; `verify_ok_vector_reproducible_from_seed` regenerates it
//! and asserts the committed fixture matches, so the vector is reproducible and tamper-evident.

use std::fs;
use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use boundless_crypto::{
    access_token_hash, access_token_matches, admin_invitation_token_hash,
    admin_invitation_token_matches, canonical_manifest_bytes, decide_manifest, decrypt_field,
    encrypt_field, onboarding_code_hash, onboarding_code_matches, phone_lookup_hash,
    phone_lookup_matches, recovery_code_hash, recovery_code_matches, refresh_token_hash,
    refresh_token_matches, unwrap_group_key, wrap_group_key, AccessTokenHash,
    AdminInvitationTokenHash, CodeHash, FetchedManifest, GroupKey, HmacKey, Kek, ManifestCache,
    ManifestDecision, ManifestErrorCode, Nonce, PhoneLookupHash, RefreshTokenHash, SecretboxError,
    Signature, VerifyingKey, KEY_LEN, MAC_LEN, NONCE_LEN,
};
use boundless_domain::{
    AccessToken, Address, AdminInvitationToken, MemberName, OnboardingCode, PhoneNumber,
    RecoveryCode, RefreshToken,
};
use dryoc::classic::crypto_sign::{crypto_sign_detached, crypto_sign_seed_keypair};
use proptest::prelude::*;
use serde_json::Value;

// === I3 / AC3 — keyed-hash constant-time verify ===================================

fn test_key() -> HmacKey {
    // A fixed non-zero test secret. In production this comes from Secrets Store.
    HmacKey::from_bytes([0x42; 32])
}

#[test]
fn i3_phone_lookup_constant_time() {
    let key = test_key();
    let alice = PhoneNumber::new("+1-555-0101");
    let bob = PhoneNumber::new("+1-555-0202");

    let alice_hash = phone_lookup_hash(&key, &alice);

    // Deterministic: the same phone hashes to the same value (so a stored hash matches).
    assert_eq!(
        phone_lookup_hash(&key, &alice).as_bytes(),
        alice_hash.as_bytes(),
        "phone-lookup hash must be deterministic for a stable lookup"
    );

    // Constant-time match: the comparison goes through `verify_slice` (no `==` on the hash,
    // which would be a non-constant-time membership oracle — R2).
    assert!(
        phone_lookup_matches(&key, &alice, &alice_hash),
        "the matching phone must verify against its stored hash"
    );
    assert!(
        !phone_lookup_matches(&key, &bob, &alice_hash),
        "a different phone must not verify (no false positive / no oracle)"
    );

    // Different inputs ⇒ different hashes.
    assert_ne!(
        phone_lookup_hash(&key, &bob).as_bytes(),
        alice_hash.as_bytes(),
        "distinct phones must produce distinct hashes"
    );

    // The secret matters: a different per-instance key must not validate the old hash.
    let other_key = HmacKey::from_bytes([0x43; 32]);
    assert!(
        !phone_lookup_matches(&other_key, &alice, &alice_hash),
        "a hash is bound to its per-instance secret"
    );
}

#[test]
fn codes_hash_at_rest_and_verify_constant_time() {
    let key = test_key();

    let onboarding = OnboardingCode::new("ONB-7F3K");
    let onboarding_hash = onboarding_code_hash(&key, &onboarding);
    assert!(onboarding_code_matches(&key, &onboarding, &onboarding_hash));
    assert!(!onboarding_code_matches(
        &key,
        &OnboardingCode::new("ONB-WRONG"),
        &onboarding_hash
    ));

    let recovery = RecoveryCode::new("REC-9QW2");
    let recovery_hash = recovery_code_hash(&key, &recovery);
    assert!(recovery_code_matches(&key, &recovery, &recovery_hash));
    assert!(!recovery_code_matches(
        &key,
        &RecoveryCode::new("REC-WRONG"),
        &recovery_hash
    ));
}

#[test]
fn domain_separation_across_code_kinds() {
    // Same secret + same code string, but hashed as an Onboarding Code, must NOT verify as
    // a Recovery Code (and vice-versa) — distinct domain tags prevent cross-role reuse.
    let key = test_key();
    let shared = "SAME-STRING-1234";

    let as_onboarding = onboarding_code_hash(&key, &OnboardingCode::new(shared));
    assert!(!recovery_code_matches(
        &key,
        &RecoveryCode::new(shared),
        &as_onboarding
    ));

    let as_recovery = recovery_code_hash(&key, &RecoveryCode::new(shared));
    assert!(!onboarding_code_matches(
        &key,
        &OnboardingCode::new(shared),
        &as_recovery
    ));
}

#[test]
fn refresh_token_hashes_at_rest_and_verifies_constant_time() {
    // T07: the refresh-credential at-rest hash backs the session-lineage classification
    // (current vs rotated-away), compared constant-time (R6 — no oracle on a long-lived
    // credential). Deterministic for a stable lookup; bound to the per-instance secret.
    let key = test_key();
    let token = RefreshToken::new("refresh-aaaa-bbbb-cccc");
    let hash = refresh_token_hash(&key, &token);

    assert!(refresh_token_matches(&key, &token, &hash));
    assert!(!refresh_token_matches(
        &key,
        &RefreshToken::new("refresh-aaaa-bbbb-WRONG"),
        &hash
    ));
    // The hash is deterministic (re-derivable for a lineage lookup)...
    assert_eq!(refresh_token_hash(&key, &token).as_bytes(), hash.as_bytes());
    // ...and bound to the per-instance secret.
    assert!(!refresh_token_matches(
        &HmacKey::from_bytes([0x43; 32]),
        &token,
        &hash
    ));
}

#[test]
fn access_token_hashes_at_rest_and_verifies_constant_time() {
    // ADR-0021: the access token is an opaque-random bearer; the server verifies a presented token
    // by re-deriving this keyed hash and looking it up (constant-time, R2/R6 — no oracle on a
    // bearer). Deterministic for a stable lookup; bound to the per-instance secret.
    let key = test_key();
    let token = AccessToken::new("access-1111-2222-3333");
    let hash = access_token_hash(&key, &token);

    assert!(access_token_matches(&key, &token, &hash));
    assert!(!access_token_matches(
        &key,
        &AccessToken::new("access-1111-2222-WRONG"),
        &hash
    ));
    // Deterministic (re-derivable for the store lookup)...
    assert_eq!(access_token_hash(&key, &token).as_bytes(), hash.as_bytes());
    // ...and bound to the per-instance secret.
    assert!(!access_token_matches(
        &HmacKey::from_bytes([0x43; 32]),
        &token,
        &hash
    ));
}

#[test]
fn access_token_domain_separated_from_refresh_and_codes() {
    // An access token and a refresh credential (or a code) that happen to share the same string
    // must hash differently — distinct domain tags prevent any cross-artifact reuse (e.g. a stolen
    // refresh credential presented as an access bearer must not verify, and vice-versa).
    let key = test_key();
    let shared = "SAME-STRING-1234";
    let as_access = access_token_hash(&key, &AccessToken::new(shared));
    let as_refresh = refresh_token_hash(&key, &RefreshToken::new(shared));
    let as_onboarding = onboarding_code_hash(&key, &OnboardingCode::new(shared));
    let as_recovery = recovery_code_hash(&key, &RecoveryCode::new(shared));
    assert_ne!(as_access.as_bytes(), as_refresh.as_bytes());
    assert_ne!(as_access.as_bytes(), as_onboarding.as_bytes());
    assert_ne!(as_access.as_bytes(), as_recovery.as_bytes());
}

#[test]
fn admin_invitation_token_hashes_at_rest_and_verifies_constant_time() {
    // AC16 / ADR-0015: the Admin registration invitation is single-use; only its keyed hash is
    // stored (never the token). Verification (T09, consume-on-register) is constant-time (R9/R2 —
    // no oracle on a registration capability). Deterministic; bound to the per-instance secret.
    let key = test_key();
    let token = AdminInvitationToken::new("invite-aaaa-bbbb-cccc");
    let hash = admin_invitation_token_hash(&key, &token);

    assert!(admin_invitation_token_matches(&key, &token, &hash));
    assert!(!admin_invitation_token_matches(
        &key,
        &AdminInvitationToken::new("invite-aaaa-bbbb-WRONG"),
        &hash
    ));
    // Deterministic (re-derivable for the consume lookup)...
    assert_eq!(
        admin_invitation_token_hash(&key, &token).as_bytes(),
        hash.as_bytes()
    );
    // ...and bound to the per-instance secret.
    assert!(!admin_invitation_token_matches(
        &HmacKey::from_bytes([0x43; 32]),
        &token,
        &hash
    ));
}

#[test]
fn admin_invitation_domain_separated_from_other_artifacts() {
    // An Admin invitation token that happens to share a string with another secret must hash
    // differently — a distinct domain tag prevents any cross-artifact reuse (e.g. a leaked access
    // bearer or refresh credential presented as a registration invitation must not verify).
    let key = test_key();
    let shared = "SAME-STRING-1234";
    let as_invite = admin_invitation_token_hash(&key, &AdminInvitationToken::new(shared));
    let as_access = access_token_hash(&key, &AccessToken::new(shared));
    let as_refresh = refresh_token_hash(&key, &RefreshToken::new(shared));
    let as_onboarding = onboarding_code_hash(&key, &OnboardingCode::new(shared));
    let as_recovery = recovery_code_hash(&key, &RecoveryCode::new(shared));
    assert_ne!(as_invite.as_bytes(), as_access.as_bytes());
    assert_ne!(as_invite.as_bytes(), as_refresh.as_bytes());
    assert_ne!(as_invite.as_bytes(), as_onboarding.as_bytes());
    assert_ne!(as_invite.as_bytes(), as_recovery.as_bytes());
}

#[test]
fn refresh_token_domain_separated_from_codes() {
    // A refresh credential and a code that happen to share the same string must hash
    // differently — distinct domain tags prevent any cross-artifact reuse. The hashes are
    // distinct *types*, so the only observable is the underlying bytes.
    let key = test_key();
    let shared = "SAME-STRING-1234";
    let as_refresh = refresh_token_hash(&key, &RefreshToken::new(shared));
    let as_onboarding = onboarding_code_hash(&key, &OnboardingCode::new(shared));
    let as_recovery = recovery_code_hash(&key, &RecoveryCode::new(shared));
    assert_ne!(as_refresh.as_bytes(), as_onboarding.as_bytes());
    assert_ne!(as_refresh.as_bytes(), as_recovery.as_bytes());
}

// === AC10 / O2 — manifest verification + tiered fallback ==========================

/// Documented, checked-in seed used to derive the `verify_ok` signing keypair. Exactly 32
/// bytes. The public half is the "bundled" trusted verification key in these tests; the
/// secret half exists only to mint the fixture vector (test-only signing).
const MANIFEST_SEED: [u8; 32] = *b"boundless-test-manifest-seed-001";

fn fixtures_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/core/crypto`; fixtures live at `<repo>/fixtures`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
}

fn load_envelope(name: &str) -> Value {
    let path = fixtures_dir().join("manifest").join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// The canonical bytes the signature covers: the envelope's `manifest` content object.
fn content_of(envelope: &Value) -> Vec<u8> {
    canonical_manifest_bytes(&envelope["manifest"])
}

fn version_of(envelope: &Value) -> u64 {
    envelope["manifest"]["manifest_version"]
        .as_u64()
        .expect("manifest_version is a u64")
}

/// Decode the envelope's base64 `signature` field, if present (`null` ⇒ `None`).
fn signature_of(envelope: &Value) -> Option<Signature> {
    let s = envelope["signature"].as_str()?;
    let bytes = STANDARD.decode(s).expect("signature is valid base64");
    Some(Signature::try_from_slice(&bytes).expect("signature is 64 bytes"))
}

/// The bundled (trusted) verification key derived from the documented seed.
fn trusted_key() -> VerifyingKey {
    let (public_key, _secret) = crypto_sign_seed_keypair(&MANIFEST_SEED);
    VerifyingKey::from_bytes(public_key)
}

#[test]
fn ac10_manifest_verify_ok_applies_fetched() {
    let env = load_envelope("verify_ok.json");
    let content = content_of(&env);
    let sig = signature_of(&env).expect("verify_ok.json carries a signature (filled in T03)");

    // Verified, version not behind the cache → apply the fetched manifest.
    let res = decide_manifest(
        Some(FetchedManifest {
            version: version_of(&env),
            canonical_content: &content,
            signature: Some(sig),
        }),
        ManifestCache::Present { version: 6 },
        &trusted_key(),
    );
    assert_eq!(res.decision, ManifestDecision::ApplyFetched);
    assert_eq!(res.code, None);

    // Also applies on a true first launch (no cache).
    let sig2 = signature_of(&env).unwrap();
    let res2 = decide_manifest(
        Some(FetchedManifest {
            version: version_of(&env),
            canonical_content: &content,
            signature: Some(sig2),
        }),
        ManifestCache::Absent,
        &trusted_key(),
    );
    assert_eq!(res2.decision, ManifestDecision::ApplyFetched);
}

#[test]
fn ac10_manifest_verify_fail_with_cache() {
    // Invalid (all-zero) signature, version 8, cache present at 7 → keep the cached manifest.
    let env = load_envelope("verify_fail_with_cache.json");
    let content = content_of(&env);
    let res = decide_manifest(
        Some(FetchedManifest {
            version: version_of(&env),
            canonical_content: &content,
            signature: signature_of(&env),
        }),
        ManifestCache::Present { version: 7 },
        &trusted_key(),
    );
    assert_eq!(res.decision, ManifestDecision::KeepCached);
    assert_eq!(res.code, Some(ManifestErrorCode::VerifyFailed));
}

#[test]
fn ac10_manifest_verify_fail_no_cache() {
    // Invalid signature, version 1, no cache (true first launch) → bundled catalog.
    let env = load_envelope("verify_fail_no_cache.json");
    let content = content_of(&env);
    let res = decide_manifest(
        Some(FetchedManifest {
            version: version_of(&env),
            canonical_content: &content,
            signature: signature_of(&env),
        }),
        ManifestCache::Absent,
        &trusted_key(),
    );
    assert_eq!(res.decision, ManifestDecision::UseBundled);
    assert_eq!(res.code, Some(ManifestErrorCode::VerifyFailed));
}

#[test]
fn ac10_manifest_lower_version_ignored() {
    // Fetched version 2 < cached 7 → ignored BEFORE any signature check (signature is null).
    let env = load_envelope("lower_version_ignored.json");
    let content = content_of(&env);
    let res = decide_manifest(
        Some(FetchedManifest {
            version: version_of(&env),
            canonical_content: &content,
            signature: signature_of(&env), // None — must never be inspected on the stale path
        }),
        ManifestCache::Present { version: 7 },
        &trusted_key(),
    );
    assert_eq!(res.decision, ManifestDecision::IgnoreStale);
    assert_eq!(res.code, Some(ManifestErrorCode::VersionStale));
}

#[test]
fn ac10_manifest_offline_first_launch() {
    // No network (no fetched manifest) and no cache → fall back to the bundled catalog.
    let res = decide_manifest(None, ManifestCache::Absent, &trusted_key());
    assert_eq!(res.decision, ManifestDecision::UseBundled);
    assert_eq!(res.code, Some(ManifestErrorCode::Offline));

    // Offline on a returning device keeps the cached manifest (ManifestFailReturning).
    let res2 = decide_manifest(None, ManifestCache::Present { version: 7 }, &trusted_key());
    assert_eq!(res2.decision, ManifestDecision::KeepCached);
    assert_eq!(res2.code, Some(ManifestErrorCode::Offline));
}

#[test]
fn manifest_verify_rejects_attacker_supplied_key() {
    // The attacker re-signs verify_ok's real content with THEIR OWN key and would happily
    // ship their own public key in the envelope. Verification uses the BUNDLED trusted key,
    // never the envelope's, so the forged manifest must NOT be applied.
    let env = load_envelope("verify_ok.json");
    let content = content_of(&env);

    let attacker_seed: [u8; 32] = *b"attacker-controlled-seed-aaaaaaa";
    let (_attacker_pub, attacker_secret) = crypto_sign_seed_keypair(&attacker_seed);
    let mut forged = [0u8; 64];
    crypto_sign_detached(&mut forged, &content, &attacker_secret).expect("attacker can sign");

    let res = decide_manifest(
        Some(FetchedManifest {
            version: version_of(&env),
            canonical_content: &content,
            signature: Some(Signature::from_bytes(forged)),
        }),
        ManifestCache::Present { version: 6 },
        &trusted_key(), // the legitimate bundled key
    );
    assert_ne!(
        res.decision,
        ManifestDecision::ApplyFetched,
        "a manifest signed by an attacker key must never be applied"
    );
    assert_eq!(res.decision, ManifestDecision::KeepCached);
    assert_eq!(res.code, Some(ManifestErrorCode::VerifyFailed));
}

#[test]
fn ac10_manifest_tampered_content_rejected() {
    // The signature must be bound to the CONTENT (AC10 / O2): a signature that is genuinely
    // valid over the ORIGINAL bytes must NOT validate MUTATED content — otherwise an attacker
    // could tamper with validly-signed copy/error_messages. This is what makes the signature
    // load-bearing: a regression where verify stopped binding sig↔content would fail HERE.
    let env = load_envelope("verify_ok.json");
    let real_sig = signature_of(&env).expect("verify_ok carries a real, valid signature");

    // Mutate one byte of the signed content, keeping manifest_version = 7 so the stale check
    // does not short-circuit before verification.
    let mut tampered = env["manifest"].clone();
    tampered["copy"]["onboarding.autoupdate.enabled"] =
        Value::String("Automatic updates are ON.".to_string());
    let tampered_content = canonical_manifest_bytes(&tampered);
    assert_ne!(
        tampered_content,
        content_of(&env),
        "the mutation must actually change the signed bytes"
    );

    // Cache present → keep the (still-trusted) cached manifest; never apply the tampered one.
    let res = decide_manifest(
        Some(FetchedManifest {
            version: 7,
            canonical_content: &tampered_content,
            signature: Some(real_sig.clone()),
        }),
        ManifestCache::Present { version: 6 },
        &trusted_key(),
    );
    assert_eq!(res.decision, ManifestDecision::KeepCached);
    assert_eq!(res.code, Some(ManifestErrorCode::VerifyFailed));

    // No cache → bundled catalog (still never the tampered manifest).
    let res2 = decide_manifest(
        Some(FetchedManifest {
            version: 7,
            canonical_content: &tampered_content,
            signature: Some(real_sig),
        }),
        ManifestCache::Absent,
        &trusted_key(),
    );
    assert_eq!(res2.decision, ManifestDecision::UseBundled);
    assert_eq!(res2.code, Some(ManifestErrorCode::VerifyFailed));
}

#[test]
fn ac10_manifest_equal_version_applies_fetched() {
    // Boundary: the stale rule is strictly `fetched < cached`, so a valid re-fetch of the
    // SAME version (7 over a cached 7) is not stale and still applies.
    let env = load_envelope("verify_ok.json");
    let content = content_of(&env);
    let res = decide_manifest(
        Some(FetchedManifest {
            version: version_of(&env),
            canonical_content: &content,
            signature: signature_of(&env),
        }),
        ManifestCache::Present { version: 7 },
        &trusted_key(),
    );
    assert_eq!(res.decision, ManifestDecision::ApplyFetched);
    assert_eq!(res.code, None);
}

#[test]
fn signature_try_from_slice_enforces_length() {
    // Malformed-length signatures (e.g. a truncated base64 blob from a bad envelope) are
    // rejected at the parse boundary — `None`, never a panic.
    assert!(Signature::try_from_slice(&[0u8; 64]).is_some());
    assert!(Signature::try_from_slice(&[0u8; 63]).is_none());
    assert!(Signature::try_from_slice(&[0u8; 65]).is_none());
    assert!(Signature::try_from_slice(&[]).is_none());
}

#[test]
fn verify_ok_vector_reproducible_from_seed() {
    // Regenerate the committed vector from the documented seed and assert the fixture matches.
    // This both proves the vector's provenance and makes accidental fixture edits fail loudly.
    let env = load_envelope("verify_ok.json");
    let content = content_of(&env);

    let (public_key, secret_key) = crypto_sign_seed_keypair(&MANIFEST_SEED);
    let mut sig = [0u8; 64];
    crypto_sign_detached(&mut sig, &content, &secret_key).expect("sign verify_ok content");

    let sig_b64 = STANDARD.encode(sig);
    let pk_b64 = STANDARD.encode(public_key);

    assert_eq!(
        env["signature"].as_str(),
        Some(sig_b64.as_str()),
        "verify_ok.json signature must equal the seed-derived detached signature"
    );
    assert_eq!(
        env["public_key"].as_str(),
        Some(pk_b64.as_str()),
        "verify_ok.json public_key must equal the seed-derived public key"
    );
}

#[test]
fn canonical_manifest_bytes_is_deterministic_sorted() {
    // Guards the signing contract: keys are emitted in sorted order, compact (no whitespace),
    // regardless of insertion order. If serde_json's `preserve_order` feature were ever
    // enabled, this fails — and every signature would silently break — so catch it here.
    let a: Value = serde_json::from_str(r#"{"b":1,"a":{"y":2,"x":1},"c":[3,2,1]}"#).unwrap();
    let b: Value = serde_json::from_str(r#"{"c":[3,2,1],"a":{"x":1,"y":2},"b":1}"#).unwrap();
    let expected = br#"{"a":{"x":1,"y":2},"b":1,"c":[3,2,1]}"#;

    assert_eq!(canonical_manifest_bytes(&a), expected);
    assert_eq!(
        canonical_manifest_bytes(&a),
        canonical_manifest_bytes(&b),
        "canonical bytes must be independent of key insertion order"
    );
}

#[test]
fn manifest_error_codes_match_registry() {
    // The stable strings (P12) must match docs/error-codes.md exactly.
    assert_eq!(
        ManifestErrorCode::VerifyFailed.as_str(),
        "MANIFEST_VERIFY_FAILED"
    );
    assert_eq!(
        ManifestErrorCode::VersionStale.as_str(),
        "MANIFEST_VERSION_STALE"
    );
    assert_eq!(ManifestErrorCode::Offline.as_str(), "NET_OFFLINE");
}

// === P2 — key/hash types expose no formatter (compile-time) =======================

mod no_formatter {
    use super::*;
    use static_assertions::assert_not_impl_any;

    // Key material and PII-derived hashes must never be loggable/serializable (P2 / I3).
    assert_not_impl_any!(HmacKey: core::fmt::Debug, core::fmt::Display, serde::Serialize);
    assert_not_impl_any!(PhoneLookupHash: core::fmt::Debug, core::fmt::Display, serde::Serialize);
    assert_not_impl_any!(CodeHash: core::fmt::Debug, core::fmt::Display, serde::Serialize);
    assert_not_impl_any!(RefreshTokenHash: core::fmt::Debug, core::fmt::Display, serde::Serialize);
    assert_not_impl_any!(AccessTokenHash: core::fmt::Debug, core::fmt::Display, serde::Serialize);
    assert_not_impl_any!(
        AdminInvitationTokenHash: core::fmt::Debug,
        core::fmt::Display,
        serde::Serialize
    );

    // The hashes must NOT be `==`-comparable: a derived `PartialEq` on `[u8; 32]` short-circuits
    // and would be a non-constant-time membership oracle (R2). Callers must use the constant-time
    // `*_matches` functions instead. This makes "no `==` path" a compile-time guarantee.
    assert_not_impl_any!(PhoneLookupHash: core::cmp::PartialEq);
    assert_not_impl_any!(CodeHash: core::cmp::PartialEq);
    assert_not_impl_any!(RefreshTokenHash: core::cmp::PartialEq);
    assert_not_impl_any!(AccessTokenHash: core::cmp::PartialEq);
    assert_not_impl_any!(AdminInvitationTokenHash: core::cmp::PartialEq);

    // The per-Group field-encryption keys (spec 008 T02, ADR-0025 R2) are unloggable by design:
    // no formatter, not serializable. (They also expose no byte accessor — enforced by API, not a
    // trait — and zeroize on drop.)
    assert_not_impl_any!(GroupKey: core::fmt::Debug, core::fmt::Display, serde::Serialize);
    assert_not_impl_any!(Kek: core::fmt::Debug, core::fmt::Display, serde::Serialize);
}

// === I1 — field-level PII encryption (secretbox, ADR-0025; spec 008 T02) ===========

/// AC2: a created member's **address is encrypted at rest** with the per-Group secretbox key — the
/// stored blob is `nonce ‖ ciphertext` (ciphertext ≠ plaintext), the round-trip requires the
/// **unwrapped** `GroupKey`, a wrong key yields `Err` (not garbage), and a tampered byte yields `Err`.
#[test]
fn i1_addresses_encrypted() {
    let key = GroupKey::from_bytes([0x11; KEY_LEN]);
    let nonce = Nonce::from_bytes([0x22; NONCE_LEN]);
    let address = Address::new("742 Evergreen Terrace");

    let stored = encrypt_field(address.expose_secret().as_bytes(), &key, &nonce);

    // Stored shape: nonce ‖ MAC ‖ ciphertext; the nonce is carried alongside.
    assert_eq!(&stored[..NONCE_LEN], nonce.as_bytes());
    assert_eq!(
        stored.len(),
        NONCE_LEN + MAC_LEN + address.expose_secret().len()
    );
    // Ciphertext region differs from plaintext — it is genuinely encrypted, not stored in clear.
    assert_ne!(
        &stored[NONCE_LEN + MAC_LEN..],
        address.expose_secret().as_bytes()
    );

    // Round-trip requires the (unwrapped) GroupKey — I1's `from_db(bytes, &GroupKey)` shape. The
    // decrypted bytes are re-wrapped into the tainted `Address` at the boundary.
    let recovered = Address::new(
        String::from_utf8(decrypt_field(&stored, &key).expect("decrypt with the correct key"))
            .expect("address round-trips as UTF-8"),
    );
    assert_eq!(recovered.expose_secret(), address.expose_secret());

    // A wrong key fails the Poly1305 check (an Err, never garbage / never a panic).
    let wrong = GroupKey::from_bytes([0x99; KEY_LEN]);
    assert_eq!(decrypt_field(&stored, &wrong), Err(SecretboxError::Decrypt));

    // Tampering any ciphertext byte fails authentication.
    let mut tampered = stored.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    assert_eq!(decrypt_field(&tampered, &key), Err(SecretboxError::Decrypt));

    // A truncated blob is rejected as malformed (not a panic).
    assert_eq!(
        decrypt_field(&stored[..NONCE_LEN], &key),
        Err(SecretboxError::Malformed)
    );
}

/// AC3: a created member's **name is encrypted at rest** under the same per-Group key.
#[test]
fn i1_name_encrypted() {
    let key = GroupKey::from_bytes([0x33; KEY_LEN]);
    let nonce = Nonce::from_bytes([0x44; NONCE_LEN]);
    let name = MemberName::new("Maria Sánchez");

    let stored = encrypt_field(name.expose_secret().as_bytes(), &key, &nonce);

    assert_eq!(&stored[..NONCE_LEN], nonce.as_bytes());
    assert_ne!(
        &stored[NONCE_LEN + MAC_LEN..],
        name.expose_secret().as_bytes()
    );

    let recovered = MemberName::new(
        String::from_utf8(decrypt_field(&stored, &key).expect("decrypt with the correct key"))
            .expect("name round-trips as UTF-8"),
    );
    assert_eq!(recovered.expose_secret(), name.expose_secret());

    let wrong = GroupKey::from_bytes([0x55; KEY_LEN]);
    assert_eq!(decrypt_field(&stored, &wrong), Err(SecretboxError::Decrypt));
}

proptest! {
    /// Encrypt→decrypt round-trips for any plaintext; the stored blob is `nonce ‖ ciphertext` of the
    /// expected length with the nonce prepended; the ciphertext differs from the plaintext; and the
    /// same plaintext under a different nonce yields a different ciphertext (semantic security — the
    /// nonce is actually used). The footgun this guards is nonce reuse (R1).
    #[test]
    fn prop_secretbox_round_trip_and_ciphertext_differs(
        key_bytes in proptest::array::uniform32(any::<u8>()),
        n1 in proptest::array::uniform24(any::<u8>()),
        n2 in proptest::array::uniform24(any::<u8>()),
        plaintext in proptest::collection::vec(any::<u8>(), 0..256),
    ) {
        let key = GroupKey::from_bytes(key_bytes);
        let stored = encrypt_field(&plaintext, &key, &Nonce::from_bytes(n1));

        prop_assert_eq!(stored.len(), NONCE_LEN + MAC_LEN + plaintext.len());
        prop_assert_eq!(&stored[..NONCE_LEN], &n1[..]);
        if !plaintext.is_empty() {
            prop_assert_ne!(&stored[NONCE_LEN + MAC_LEN..], &plaintext[..]);
        }
        prop_assert_eq!(decrypt_field(&stored, &key).unwrap(), plaintext.clone());

        // Same plaintext, a different nonce → a different ciphertext region (the nonce matters).
        prop_assume!(n1 != n2);
        let stored2 = encrypt_field(&plaintext, &key, &Nonce::from_bytes(n2));
        prop_assert_ne!(&stored[NONCE_LEN..], &stored2[NONCE_LEN..]);
    }

    /// Decrypting with the wrong key always fails authentication (never returns garbage plaintext).
    #[test]
    fn prop_decrypt_wrong_key_fails(
        k1 in proptest::array::uniform32(any::<u8>()),
        k2 in proptest::array::uniform32(any::<u8>()),
        nonce in proptest::array::uniform24(any::<u8>()),
        plaintext in proptest::collection::vec(any::<u8>(), 1..256),
    ) {
        prop_assume!(k1 != k2);
        let stored = encrypt_field(&plaintext, &GroupKey::from_bytes(k1), &Nonce::from_bytes(nonce));
        prop_assert_eq!(
            decrypt_field(&stored, &GroupKey::from_bytes(k2)),
            Err(SecretboxError::Decrypt)
        );
    }

    /// KEK wrap→unwrap recovers the per-Group key: a field encrypted with the original key decrypts
    /// with the unwrapped key (functional key equality, without exposing key bytes). A wrong KEK
    /// fails to unwrap.
    #[test]
    fn prop_kek_wrap_unwrap_round_trips(
        group_key_bytes in proptest::array::uniform32(any::<u8>()),
        kek_bytes in proptest::array::uniform32(any::<u8>()),
        wrap_nonce in proptest::array::uniform24(any::<u8>()),
        field_nonce in proptest::array::uniform24(any::<u8>()),
        plaintext in proptest::collection::vec(any::<u8>(), 0..128),
    ) {
        let group_key = GroupKey::from_bytes(group_key_bytes);
        let kek = Kek::from_bytes(kek_bytes);

        let ciphertext = encrypt_field(&plaintext, &group_key, &Nonce::from_bytes(field_nonce));
        let wrapped = wrap_group_key(&group_key, &kek, &Nonce::from_bytes(wrap_nonce));
        prop_assert_eq!(wrapped.len(), NONCE_LEN + MAC_LEN + KEY_LEN);

        let unwrapped = unwrap_group_key(&wrapped, &kek).expect("unwrap with the correct KEK");
        prop_assert_eq!(decrypt_field(&ciphertext, &unwrapped).unwrap(), plaintext);

        // A wrong KEK cannot unwrap the key. (`matches!`, not `prop_assert_eq!`: GroupKey
        // deliberately has no Debug/PartialEq, so the Ok arm can't be formatted/compared.)
        let mut other = kek_bytes;
        other[0] ^= 0x01;
        prop_assert!(matches!(
            unwrap_group_key(&wrapped, &Kek::from_bytes(other)),
            Err(SecretboxError::Decrypt)
        ));
    }
}
