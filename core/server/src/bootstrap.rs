//! Group bootstrap — per-Group key generation + the fail-closed key-load gate (AC12, ADR-0025).
//!
//! Spec 008 is the first spec that writes PII at rest, so each Group needs a **secretbox key (the
//! DEK)** to encrypt member name/address (I1). This module is the *functional core* of that key's
//! lifecycle decisions:
//!
//! - [`generate_group_key`] — mint a fresh key from the **injected** CSPRNG
//!   ([`SecretSource::fresh_group_key`]), KEK-wrap it ([`wrap_group_key`]), and shape the
//!   `delegated_keys` write. The plaintext key is returned for the `GroupHub` DO to cache in memory
//!   only; **only the wrapped blob is ever persisted** (AC12 — "the plaintext key never appears in
//!   durable storage or logs").
//! - [`load_group_key`] — the **fail-closed gate** a field-encrypting operation (issuance, T05's
//!   `MemberService::issue`) calls *first*: a missing bootstrap row **or** a wrong-KEK/corrupt blob
//!   both yield [`GroupKeyMissing`] → `ADMIN_GROUP_KEY_MISSING`, so the caller returns before
//!   composing any member write (no row is written; no `unwrap()` on the key load).
//!
//! **Not here (deferred shells):** the actual `delegated_keys` row write
//! ([`crate::ports`]/`PgDelegatedKeyStore`, T07), the operator-run provisioning that creates the
//! single `groups` + `delegated_keys` rows (plan §13.4), and the Worker loading the [`Kek`] from
//! Cloudflare Secrets Store + caching the unwrapped [`GroupKey`] in the DO (T09). Bootstrap is
//! operator-run provisioning, **never a Worker request path**, so the Worker's `SecretSource` never
//! mints a Group key (its `fresh_group_key` is `unreachable!`).

use boundless_crypto::{unwrap_group_key, wrap_group_key, GroupKey, Kek};

use crate::ports::SecretSource;

/// The KEK version a freshly-bootstrapped Group key is wrapped under (`delegated_keys.kek_version`
/// starts at 1; a KEK re-wrap rotation bumps it — runbook-documented, unbuilt, ADR-0025).
pub const INITIAL_KEK_VERSION: i32 = 1;

/// The product of Group bootstrap: the cached plaintext key + the `delegated_keys` write shape.
///
/// Holds a [`GroupKey`], so it is — by construction — not `Debug`/`Display`/`Serialize` (P2): the
/// plaintext key cannot be logged or serialized out. The Worker caches [`group_key`](Self::group_key)
/// in `GroupHub` DO memory and persists only [`wrapped_key`](Self::wrapped_key) +
/// [`kek_version`](Self::kek_version).
pub struct GroupKeyBootstrap {
    /// The freshly-minted plaintext per-Group key (the DEK) — cached in `GroupHub` DO memory only,
    /// **never persisted** (only the wrapped blob is). Zeroizes on drop (R2).
    pub group_key: GroupKey,
    /// The KEK-wrapped key for `delegated_keys.wrapped_key` — `nonce ‖ ciphertext`, never plaintext.
    pub wrapped_key: Vec<u8>,
    /// The KEK version this blob is wrapped under ([`INITIAL_KEK_VERSION`] at bootstrap).
    pub kek_version: i32,
}

/// Mint a fresh per-Group key and KEK-wrap it for at-rest storage (AC12, ADR-0025).
///
/// Both the key and the wrap nonce come from the **injected** CSPRNG (no ambient randomness in the
/// core — ADR-0021; the nonce-reuse footgun guard, R1). The returned [`GroupKeyBootstrap`] carries
/// the plaintext key (for the DO cache) and the wrapped blob (for the `delegated_keys` write).
pub fn generate_group_key(secrets: &mut impl SecretSource, kek: &Kek) -> GroupKeyBootstrap {
    let group_key = secrets.fresh_group_key();
    let nonce = secrets.fresh_nonce();
    let wrapped_key = wrap_group_key(&group_key, kek, &nonce);
    GroupKeyBootstrap {
        group_key,
        wrapped_key,
        kek_version: INITIAL_KEK_VERSION,
    }
}

/// The per-Group key could not be made available for a field-encrypting operation (AC12 fail-closed).
///
/// Maps to the stable `ADMIN_GROUP_KEY_MISSING` code (`docs/error-codes.md`, registered at T01).
/// Carries **no** key, ciphertext, or PII — only the fact of failure — so it is safe to surface/log.
/// Deliberately does not distinguish "no row" from "unwrap failed": both are an absent usable key,
/// and revealing which is a needless signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupKeyMissing;

impl GroupKeyMissing {
    /// The stable error code (`docs/error-codes.md`, P12).
    pub const fn error_code(self) -> &'static str {
        "ADMIN_GROUP_KEY_MISSING"
    }
}

/// Load + unwrap the per-Group key, **failing closed** (AC12, ADR-0025 R11/R12).
///
/// A field-encrypting operation (issuance — T05's `MemberService::issue`) calls this as its **first**
/// step, before composing any member write. `wrapped == None` (no bootstrap row — the key was never
/// generated) and an [`unwrap_group_key`] failure (wrong KEK / tampered or corrupt blob) **both**
/// yield [`GroupKeyMissing`] → `ADMIN_GROUP_KEY_MISSING`. Because the gate runs before the store is
/// touched, a missing key means **no row is written** (and there is no `unwrap()` on the key load —
/// this returns a `Result`, never panics).
pub fn load_group_key(wrapped: Option<&[u8]>, kek: &Kek) -> Result<GroupKey, GroupKeyMissing> {
    let wrapped = wrapped.ok_or(GroupKeyMissing)?;
    unwrap_group_key(wrapped, kek).map_err(|_| GroupKeyMissing)
}

#[cfg(test)]
mod tests {
    use super::{generate_group_key, load_group_key, INITIAL_KEK_VERSION};
    use crate::ports::SecretSource;
    use crate::secrets::RngSecretSource;
    use boundless_crypto::{
        decrypt_field, encrypt_field, unwrap_group_key, Kek, KEY_LEN, MAC_LEN, NONCE_LEN,
    };
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    /// Bootstrap mints a key from the injected CSPRNG and KEK-wraps it: the persisted blob is the
    /// wrapped key (not plaintext), it round-trips only via the real KEK, and the blob wraps exactly
    /// the same key cached for the DO (AC12). Driven by a *seeded* production `RngSecretSource`.
    #[test]
    fn bootstrap_generates_wrapped_key_from_injected_seed() {
        let kek = Kek::from_bytes([0x11; KEY_LEN]);
        let mut secrets = RngSecretSource::new(ChaCha20Rng::seed_from_u64(7));
        let boot = generate_group_key(&mut secrets, &kek);

        // The persisted blob is the wrapped key: exactly `nonce ‖ mac ‖ key`, version 1.
        assert_eq!(boot.wrapped_key.len(), NONCE_LEN + MAC_LEN + KEY_LEN);
        assert_eq!(boot.kek_version, INITIAL_KEK_VERSION);

        // It is genuinely KEK-encrypted, not the key in the clear: a wrong KEK cannot unwrap it.
        let wrong_kek = Kek::from_bytes([0x22; KEY_LEN]);
        assert!(unwrap_group_key(&boot.wrapped_key, &wrong_kek).is_err());

        // It round-trips via the real KEK, AND the unwrapped key is the SAME key cached for the DO:
        // encrypt a probe with the cached `group_key`, decrypt it with the key recovered from the
        // blob. (Also proves ciphertext ≠ plaintext.)
        let recovered = unwrap_group_key(&boot.wrapped_key, &kek).expect("correct KEK unwraps");
        let nonce = secrets.fresh_nonce();
        let probe = b"123 Maple St";
        let ct = encrypt_field(probe, &boot.group_key, &nonce);
        assert_ne!(
            &ct[NONCE_LEN..],
            &probe[..],
            "ciphertext must not equal plaintext"
        );
        assert_eq!(
            decrypt_field(&ct, &recovered).expect("blob wraps the same key the DO cached"),
            probe
        );
    }

    /// Two Groups bootstrapped from **independent** CSPRNG instances must not share a wrapped key —
    /// the cross-isolate concern behind the R1 nonce discipline, at the key level (a pooled Worker
    /// fleet has no shared state). A per-instance constant/counter key would tie these.
    #[test]
    fn bootstrap_generates_distinct_keys_per_group() {
        let kek = Kek::from_bytes([0x11; KEY_LEN]);
        let mut a = RngSecretSource::new(ChaCha20Rng::seed_from_u64(1));
        let mut b = RngSecretSource::new(ChaCha20Rng::seed_from_u64(2));
        let boot_a = generate_group_key(&mut a, &kek);
        let boot_b = generate_group_key(&mut b, &kek);
        assert_ne!(
            boot_a.wrapped_key, boot_b.wrapped_key,
            "independently-bootstrapped Groups must not share a key"
        );
    }

    /// The fail-closed gate (AC12) issuance calls FIRST (T05's `MemberService::issue` wires it in):
    /// with no Group key — or a corrupt/wrong-KEK one — it returns `ADMIN_GROUP_KEY_MISSING`, and
    /// because the gate runs before any member write, no row is composed (no `unwrap()` on the load).
    #[test]
    fn issuance_fails_closed_without_group_key() {
        let kek = Kek::from_bytes([0x33; KEY_LEN]);

        // (a) No Group key was ever generated → fail closed with the registered code. (`let-else`
        // rather than `expect_err`, which would require the `Ok` type `GroupKey` to be `Debug` — it
        // deliberately is not, P2.)
        let Err(missing) = load_group_key(None, &kek) else {
            panic!("absent key must fail closed");
        };
        assert_eq!(missing.error_code(), "ADMIN_GROUP_KEY_MISSING");

        // (b) A present-but-undecryptable blob also fails closed — never garbage, never a panic.
        let mut secrets = RngSecretSource::new(ChaCha20Rng::seed_from_u64(5));
        let boot = generate_group_key(&mut secrets, &kek);
        let wrong_kek = Kek::from_bytes([0x44; KEY_LEN]);
        let Err(wrong) = load_group_key(Some(&boot.wrapped_key), &wrong_kek) else {
            panic!("wrong KEK must fail closed");
        };
        assert_eq!(wrong.error_code(), "ADMIN_GROUP_KEY_MISSING");
        let mut tampered = boot.wrapped_key.clone();
        *tampered.last_mut().expect("non-empty blob") ^= 0x01;
        assert!(
            load_group_key(Some(&tampered), &kek).is_err(),
            "a tampered blob must fail closed (Poly1305)"
        );

        // (c) The correct KEK + a valid blob loads the key — the gate opens only when it is present.
        assert!(load_group_key(Some(&boot.wrapped_key), &kek).is_ok());
    }
}
