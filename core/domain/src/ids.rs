//! Stable, opaque identifiers.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A stable, opaque identifier for a member — a (Group, Person) pair (glossary).
///
/// It is **never shown in the UI**, but it is *not* PII: it is the privacy-preserving
/// stand-in that PII is replaced with on deletion (I12), so it is safe to carry in audit
/// logs and on the wire. Serializes transparently as the canonical hyphenated UUID
/// string (e.g. `"550e8400-e29b-41d4-a716-446655440000"`), so it round-trips as a bare
/// string rather than a wrapper object.
///
/// We only ever *parse/format* `MemberId`s here — generation (which needs randomness)
/// happens server-side, keeping `core::domain` pure and wasm-safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MemberId(Uuid);

impl MemberId {
    /// Wrap an existing UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// The underlying UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for MemberId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// A stable, opaque identifier for a **session family** — the lineage of one member login
/// and all of its rotated refresh credentials (ADR-0016 D2). Every silent refresh rotates
/// the credential but keeps the same `SessionFamilyId`, so replay detection can revoke the
/// whole family at once (see `core::auth`).
///
/// Like [`MemberId`] it is opaque, never shown in the UI, and **not** PII — safe to carry on
/// the wire and in audit logs. Serializes transparently as the canonical hyphenated UUID
/// string. We only ever *parse/format* it here; generation (which needs randomness) happens
/// server-side, keeping `core::domain` pure and wasm-safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionFamilyId(Uuid);

impl SessionFamilyId {
    /// Wrap an existing UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// The underlying UUID.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for SessionFamilyId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}
