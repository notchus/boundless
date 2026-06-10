//! Field-level symmetric encryption for PII at rest (I1, ADR-0025).
//!
//! The per-Group **secretbox** key (`GroupKey`) encrypts a member's address and name at rest
//! (`members.{address,name}_encrypted`); the key itself is stored only **KEK-wrapped**
//! (`wrap_group_key`/`unwrap_group_key` against the [`Kek`] from Cloudflare Secrets Store) in
//! `delegated_keys.wrapped_key`. The primitive is dryoc's `crypto_secretbox` (XSalsa20-Poly1305),
//! verified against the locked dryoc 0.8.0 (`Key=[u8;32]`, `Nonce=[u8;24]`, `Mac=[u8;16]`; the
//! `_easy` combined mode stores `MAC ‖ ciphertext`).
//!
//! **R1 — nonce discipline.** [`encrypt_field`] is the *only* field-encryption entry point and it
//! **requires** a caller-supplied [`Nonce`]; there is no nonce-less overload, so a caller cannot omit
//! or hand-roll the nonce. The nonce is a fresh random draw from the **injected** CSPRNG
//! (`SecretSource::fresh_nonce`, ADR-0021 — no ambient randomness in the core; this crate stays
//! `wasm32`-safe and never calls `getrandom`). XSalsa20-Poly1305 nonce reuse is catastrophic and the
//! Group key is long-lived (rotation deferred — ADR-0025), so nonce uniqueness is the load-bearing
//! guard; a counter/deterministic nonce is forbidden (a pooled multi-isolate Worker fleet has no
//! shared counter). The stored blob is `nonce ‖ ciphertext`, the nonce carried alongside.
//!
//! **R2 — zeroize the keys.** [`GroupKey`]/[`Kek`] implement neither `Debug`/`Display`/`Serialize`
//! (compile-asserted in `tests/invariants.rs`, like `HmacKey`) **nor** expose their bytes, and they
//! zeroize on drop. Unlike the process-lifetime `HmacKey`, the Group key is unwrapped per-DO-init and
//! the threat is a Durable Object memory snapshot, so zeroize is load-bearing here, not GA hardening.

use dryoc::classic::crypto_secretbox::{crypto_secretbox_easy, crypto_secretbox_open_easy};
use dryoc::constants::{
    CRYPTO_SECRETBOX_KEYBYTES, CRYPTO_SECRETBOX_MACBYTES, CRYPTO_SECRETBOX_NONCEBYTES,
};
use zeroize::{Zeroize, Zeroizing};

/// secretbox key length — 32 bytes (XSalsa20-Poly1305; from the dryoc 0.8.0 constant).
pub const KEY_LEN: usize = CRYPTO_SECRETBOX_KEYBYTES;
/// secretbox nonce length — 24 bytes.
pub const NONCE_LEN: usize = CRYPTO_SECRETBOX_NONCEBYTES;
/// Poly1305 authentication-tag length, prepended to each ciphertext — 16 bytes.
pub const MAC_LEN: usize = CRYPTO_SECRETBOX_MACBYTES;

/// A single-use random nonce for exactly one [`encrypt_field`]/[`wrap_group_key`] call (R1).
///
/// Not secret (it is stored alongside the ciphertext, so it is **not** zeroized), but it **must** be
/// unique per (key, encryption) — see the module docs. Constructed from the injected CSPRNG
/// (`SecretSource::fresh_nonce`); never counter-derived.
#[derive(Clone)]
pub struct Nonce([u8; NONCE_LEN]);

impl Nonce {
    /// Wrap `NONCE_LEN` nonce bytes. In production these come from the injected CSPRNG
    /// (`SecretSource::fresh_nonce`), never counter/time-derived — the constructor itself does not
    /// enforce that (tests pass fixed bytes); the uniqueness invariant lives at the production caller.
    pub fn from_bytes(bytes: [u8; NONCE_LEN]) -> Self {
        Self(bytes)
    }

    /// The raw nonce bytes (not secret — stored alongside the ciphertext as `nonce ‖ ciphertext`).
    pub fn as_bytes(&self) -> &[u8; NONCE_LEN] {
        &self.0
    }
}

/// The per-Group field-encryption key (the DEK), 32 bytes (ADR-0025).
///
/// Encrypts member PII at rest. Tainted by construction: **no** `Debug`/`Display`/`Serialize`
/// (compile-asserted, P2) and **no** byte accessor — only this module's own `encrypt_field`/
/// `wrap_group_key` reach the inner bytes. Zeroized on drop (R2). Construct from the injected CSPRNG
/// at Group bootstrap, or via [`unwrap_group_key`]; the plaintext key lives in `GroupHub` DO memory
/// only and is never persisted (only the KEK-wrapped blob is — `delegated_keys.wrapped_key`).
pub struct GroupKey([u8; KEY_LEN]);

impl GroupKey {
    /// Wrap raw key bytes — from the injected CSPRNG at bootstrap, or a freshly-unwrapped key.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }
}

impl Drop for GroupKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// The Key-Encryption-Key (KEK), 32 bytes, loaded from Cloudflare Secrets Store (ADR-0025 R3).
///
/// Wraps/unwraps the per-Group key ([`wrap_group_key`]/[`unwrap_group_key`]); it never encrypts
/// member fields directly. Same unloggable + byte-opaque + zeroize-on-drop discipline as [`GroupKey`].
pub struct Kek([u8; KEY_LEN]);

impl Kek {
    /// Wrap the raw KEK bytes loaded from Secrets Store.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }
}

impl Drop for Kek {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Why a field/key could not be decrypted. Carries **no** plaintext, key, or ciphertext (P2) — only
/// the reason — so it is safe to log/surface; the caller maps it to a stable upstream code (e.g. a
/// failed key unwrap becomes `ADMIN_GROUP_KEY_MISSING`, decided by the caller, not here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretboxError {
    /// The stored blob is shorter than `nonce ‖ mac` (or the wrong length for a wrapped key) —
    /// truncated/corrupt, not a valid ciphertext.
    Malformed,
    /// Authentication failed: a wrong key or a tampered byte (Poly1305 tag mismatch).
    Decrypt,
}

/// Encrypt one PII field under the per-Group key with a caller-supplied fresh nonce (R1).
///
/// Returns `nonce ‖ ciphertext` where `ciphertext` is dryoc's `MAC ‖ encrypted`, so the blob is
/// exactly `NONCE_LEN + MAC_LEN + plaintext.len()` bytes. This is the **only** field-encryption entry
/// point — no nonce-less overload — so a caller cannot omit the nonce (the nonce-reuse footgun, R1).
pub fn encrypt_field(plaintext: &[u8], key: &GroupKey, nonce: &Nonce) -> Vec<u8> {
    let mut combined = vec![0u8; MAC_LEN + plaintext.len()];
    // `crypto_secretbox_easy` returns `Ok(())` unconditionally (verified in the vendored 0.8.0
    // source: it writes the MAC into combined[..MAC_LEN] and the ciphertext into combined[MAC_LEN..],
    // with no error branch); it would only *panic* (slice out-of-bounds) on a mis-sized output
    // buffer. Ours is exactly MAC_LEN + plaintext.len() — the required size — so it can neither panic
    // nor error; `.expect()` is the forbidden-patterns form for a provably-unreachable `Result`.
    crypto_secretbox_easy(&mut combined, plaintext, &nonce.0, &key.0)
        .expect("secretbox_easy returns Ok at the exact buffer size MAC_LEN + plaintext.len()");
    let mut out = Vec::with_capacity(NONCE_LEN + combined.len());
    out.extend_from_slice(&nonce.0);
    out.extend_from_slice(&combined);
    out
}

/// Decrypt a `nonce ‖ ciphertext` blob produced by [`encrypt_field`].
///
/// Requires the **unwrapped** `&GroupKey` by type, so "decrypt without the key" is unrepresentable
/// (I1's `from_db(bytes, &GroupKey)` shape). Returns `Err` — never garbage, never a panic — on a
/// wrong key or any tampered byte (Poly1305), and on a truncated blob.
///
/// The recovered plaintext is returned as a plain `Vec<u8>` (NOT `Zeroizing`) **deliberately**:
/// wiping only this transient buffer would not close the DO-memory-snapshot threat, because the
/// boundary (spec 008 T05/T09) re-wraps the bytes into a tainted `Address`/`MemberName` whose inner
/// `String` is not zeroized either. Whether to give the tainted types a zeroizing buffer (so the
/// decrypted PII is wiped on drop, P3/I2) is a cross-cutting decision deferred to the consuming
/// task — see `DEFERRED.md` → spec 008. Keeping the return non-`Zeroizing` here avoids security
/// theatre that the downstream `String` copy would defeat.
pub fn decrypt_field(stored: &[u8], key: &GroupKey) -> Result<Vec<u8>, SecretboxError> {
    if stored.len() < NONCE_LEN + MAC_LEN {
        return Err(SecretboxError::Malformed);
    }
    let (nonce, combined) = stored.split_at(NONCE_LEN);
    let nonce: &[u8; NONCE_LEN] = nonce
        .try_into()
        .expect("split_at(NONCE_LEN) yields exactly NONCE_LEN bytes");
    let mut plaintext = vec![0u8; combined.len() - MAC_LEN];
    crypto_secretbox_open_easy(&mut plaintext, combined, nonce, &key.0)
        .map_err(|_| SecretboxError::Decrypt)?;
    Ok(plaintext)
}

/// Wrap (encrypt) the per-Group key with the KEK for at-rest storage in `delegated_keys.wrapped_key`
/// (ADR-0025). Like [`encrypt_field`] it takes a fresh caller-supplied nonce (R1 — no nonce-less
/// overload, even for the wrap). Returns `nonce ‖ ciphertext` (`NONCE_LEN + MAC_LEN + KEY_LEN` bytes).
pub fn wrap_group_key(group_key: &GroupKey, kek: &Kek, nonce: &Nonce) -> Vec<u8> {
    let mut combined = vec![0u8; MAC_LEN + KEY_LEN];
    // Same as `encrypt_field`: `crypto_secretbox_easy` is `Ok`-only and our buffer is the exact
    // required size (MAC_LEN + KEY_LEN), so the `.expect()` is provably unreachable.
    crypto_secretbox_easy(&mut combined, &group_key.0, &nonce.0, &kek.0)
        .expect("secretbox_easy returns Ok at the exact buffer size MAC_LEN + KEY_LEN");
    let mut out = Vec::with_capacity(NONCE_LEN + combined.len());
    out.extend_from_slice(&nonce.0);
    out.extend_from_slice(&combined);
    out
}

/// Unwrap a `delegated_keys.wrapped_key` blob with the KEK, recovering the per-Group key.
///
/// Returns `Err` (never a panic) on a wrong KEK / tampered blob, so a key-load failure **fails
/// closed** (the caller renders `ADMIN_GROUP_KEY_MISSING` and writes no member row). The transient
/// plaintext key bytes are zeroized once moved into the returned [`GroupKey`].
pub fn unwrap_group_key(stored: &[u8], kek: &Kek) -> Result<GroupKey, SecretboxError> {
    if stored.len() != NONCE_LEN + MAC_LEN + KEY_LEN {
        return Err(SecretboxError::Malformed);
    }
    let (nonce, combined) = stored.split_at(NONCE_LEN);
    let nonce: &[u8; NONCE_LEN] = nonce
        .try_into()
        .expect("split_at(NONCE_LEN) yields exactly NONCE_LEN bytes");
    // `Zeroizing` so the transient key buffer is wiped on EVERY exit path — including the
    // wrong-KEK/tamper error path. dryoc's `crypto_secretbox_open_easy` XORs the keystream into the
    // output buffer *in place before* it checks the Poly1305 tag (verified in the vendored 0.8.0
    // source), so `key_bytes` already holds (wrong-key) plaintext when `?` returns the `Err`; a bare
    // `[u8; KEY_LEN]` zeroized only after the `?` would leak it on that path.
    let mut key_bytes = Zeroizing::new([0u8; KEY_LEN]);
    crypto_secretbox_open_easy(&mut *key_bytes, combined, nonce, &kek.0)
        .map_err(|_| SecretboxError::Decrypt)?;
    Ok(GroupKey::from_bytes(*key_bytes))
}
