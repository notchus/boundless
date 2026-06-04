//! Server-driven-config manifest verification + tiered fallback (ADR-0014, AC10 / O2).
//!
//! The KV manifest is signed with an **Ed25519 detached signature** (via dryoc). The
//! signature covers the **canonical** (sorted-key, compact) JSON of the manifest content
//! object — the byte contract shared by the manifest-mint Worker (which signs) and every
//! client (which verifies); both MUST produce identical bytes.
//!
//! ## Security: the trusted key is *bundled*, never taken from the envelope
//!
//! [`verify_manifest_signature`] verifies against a [`VerifyingKey`] supplied by the
//! caller — in production the Ed25519 public key embedded in the app binary (ADR-0014).
//! It deliberately does **not** read any `public_key` carried inside the manifest
//! envelope; trusting an envelope-supplied key would let any attacker sign with their own
//! key and have it accepted. (`manifest_verify_rejects_attacker_supplied_key` proves this.)

use dryoc::classic::crypto_sign::crypto_sign_verify_detached;
use serde_json::Value;

/// Length in bytes of an Ed25519 public (verifying) key.
pub const PUBLIC_KEY_LEN: usize = 32;
/// Length in bytes of an Ed25519 detached signature.
pub const SIGNATURE_LEN: usize = 64;

/// The trusted Ed25519 verification key — in production, embedded in the app binary
/// (ADR-0014). Public material, so `Debug` is fine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyingKey([u8; PUBLIC_KEY_LEN]);

impl VerifyingKey {
    /// Wrap the bundled 32-byte Ed25519 public key.
    pub fn from_bytes(bytes: [u8; PUBLIC_KEY_LEN]) -> Self {
        Self(bytes)
    }
}

/// An Ed25519 detached signature over the canonical manifest bytes. Public material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature([u8; SIGNATURE_LEN]);

impl Signature {
    /// Wrap a 64-byte detached signature.
    pub fn from_bytes(bytes: [u8; SIGNATURE_LEN]) -> Self {
        Self(bytes)
    }

    /// Parse from a byte slice (e.g. base64-decoded from the envelope); `None` on wrong length.
    pub fn try_from_slice(bytes: &[u8]) -> Option<Self> {
        let arr: [u8; SIGNATURE_LEN] = bytes.try_into().ok()?;
        Some(Self(arr))
    }
}

/// The exact bytes the Ed25519 signature covers: the manifest content object as **canonical
/// JSON** — object keys sorted, compact (no insignificant whitespace).
///
/// `serde_json::Map` is a `BTreeMap` (the `preserve_order` feature is NOT enabled — guarded
/// by `canonical_manifest_bytes_is_deterministic_sorted`), so `to_vec` is already
/// sorted-and-compact. This is the signing contract: the mint Worker and the client must
/// both canonicalize the same way, or signatures will not verify.
///
/// **Contract: the manifest must be integer-only — no floating-point numbers.** The current
/// schema is float-free (`manifest_version`, `spacing_unit`, booleans, strings, nested
/// objects/arrays). Floats have no canonical cross-implementation serialization (NaN, `-0.0`,
/// `1e3` vs `1000.0`, precision), so introducing one could make this and the mint Worker
/// disagree and silently break verification. Adding a float-valued manifest field is a
/// breaking change to the signing contract (see `DEFERRED.md` → Crypto, manifest-mint item).
pub fn canonical_manifest_bytes(manifest: &Value) -> Vec<u8> {
    serde_json::to_vec(manifest).expect("a serde_json::Value always serializes")
}

/// Verify a detached Ed25519 signature over `content` against the **trusted bundled key**.
///
/// Returns `true` iff the signature is valid. Never panics on a bad signature (dryoc returns
/// `Err`, which maps to `false`).
pub fn verify_manifest_signature(
    content: &[u8],
    signature: &Signature,
    trusted_key: &VerifyingKey,
) -> bool {
    // dryoc's classic types are transparent byte-array aliases: Signature = [u8; 64],
    // PublicKey = [u8; 32]. Our newtypes hold exactly those, so we pass the arrays straight in.
    crypto_sign_verify_detached(&signature.0, content, &trusted_key.0).is_ok()
}

/// Whether a previously-applied manifest is cached on-device, and at what version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestCache {
    /// No manifest has ever been applied (true first launch).
    Absent,
    /// A manifest is cached at this `manifest_version`.
    Present { version: u64 },
}

/// A manifest fetched from KV this launch, ready for the tiered decision.
pub struct FetchedManifest<'a> {
    /// The fetched `manifest_version` (monotonic; lower-than-cached is ignored).
    pub version: u64,
    /// The canonical bytes the signature is verified against (see [`canonical_manifest_bytes`]).
    pub canonical_content: &'a [u8],
    /// The detached signature from the envelope; `None` (e.g. unsigned) ⇒ treated as a
    /// verification failure (unless the stale check short-circuits first).
    pub signature: Option<Signature>,
}

/// Which manifest the client should apply this launch (ADR-0014 tiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestDecision {
    /// Signature verified and version is not stale → apply the freshly-fetched manifest.
    ApplyFetched,
    /// Tier 2: keep using the previously-cached manifest (verify failed / offline, cache exists).
    /// Never blocks the primary surface (`ManifestFailReturning`).
    KeepCached,
    /// Tier 3: fall back to the bundled-in-binary catalog (verify failed / offline, no cache).
    UseBundled,
    /// Fetched version is lower than the cached version → ignore it (before any signature check).
    IgnoreStale,
}

/// The stable error codes a manifest resolution can surface (P12; see `docs/error-codes.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestErrorCode {
    /// `MANIFEST_VERIFY_FAILED` — signature verification failed; fell back per tiers.
    VerifyFailed,
    /// `MANIFEST_VERSION_STALE` — fetched version lower than cached; ignored.
    VersionStale,
    /// `NET_OFFLINE` — no connectivity; used cache or bundled catalog.
    Offline,
}

impl ManifestErrorCode {
    /// The exact stable code string registered in `docs/error-codes.md`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifyFailed => "MANIFEST_VERIFY_FAILED",
            Self::VersionStale => "MANIFEST_VERSION_STALE",
            Self::Offline => "NET_OFFLINE",
        }
    }
}

/// The outcome of [`decide_manifest`]: which manifest to apply, and the non-fatal code (if any)
/// to log via the PII-free `emit()` path (never surfaced to the rider — ADR-0014 / O8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestResolution {
    /// Which manifest the client applies this launch.
    pub decision: ManifestDecision,
    /// The stable error code to log, if this was a fallback/ignore path.
    pub code: Option<ManifestErrorCode>,
}

/// Decide which manifest to apply, implementing ADR-0014's tiers and the lower-version-ignore
/// rule (AC10).
///
/// - `fetched = None` models the **offline** path (no bytes to verify).
/// - The **stale** check happens first, before any signature work (ADR-0014: a lower
///   `manifest_version` is ignored "before any signature check").
/// - On verify failure (or an unsigned/missing signature), fall back to the cache if present,
///   else the bundled catalog. The primary surface is never blocked.
pub fn decide_manifest(
    fetched: Option<FetchedManifest>,
    cache: ManifestCache,
    trusted_key: &VerifyingKey,
) -> ManifestResolution {
    let Some(f) = fetched else {
        // Offline: returning device keeps its cache; true first launch uses the bundled catalog.
        let decision = match cache {
            ManifestCache::Present { .. } => ManifestDecision::KeepCached,
            ManifestCache::Absent => ManifestDecision::UseBundled,
        };
        return ManifestResolution {
            decision,
            code: Some(ManifestErrorCode::Offline),
        };
    };

    // Stale check FIRST — never even inspect the signature of an out-of-date manifest.
    if let ManifestCache::Present { version: cached } = cache {
        if f.version < cached {
            return ManifestResolution {
                decision: ManifestDecision::IgnoreStale,
                code: Some(ManifestErrorCode::VersionStale),
            };
        }
    }

    let verified = match &f.signature {
        Some(sig) => verify_manifest_signature(f.canonical_content, sig, trusted_key),
        None => false, // unsigned ⇒ verification failure
    };

    if verified {
        ManifestResolution {
            decision: ManifestDecision::ApplyFetched,
            code: None,
        }
    } else {
        let decision = match cache {
            ManifestCache::Present { .. } => ManifestDecision::KeepCached,
            ManifestCache::Absent => ManifestDecision::UseBundled,
        };
        ManifestResolution {
            decision,
            code: Some(ManifestErrorCode::VerifyFailed),
        }
    }
}
