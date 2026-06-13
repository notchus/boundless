//! Option B1 admin-WebAuthn persistence DTOs (spec 009 **T02**, ADR-0027) — the **PII-free**
//! record/wire types the new server-to-server admin-auth endpoints move (invite-resolve +
//! credential CRUD), and the port-method outcome/write structs the store impls take.
//!
//! ## One type, two jobs (no two-type split needed here)
//!
//! Unlike the member surface — where [`MemberDetail`](crate::MemberDetail) holds tainted PII and so
//! needs a separate `Serialize` wire twin ([`MemberDetailView`](crate::MemberDetailView)) — these
//! rows carry **no PII**: opaque ids ([`MemberId`]/group `Uuid`), opaque WebAuthn bytes
//! (`credential_id`/`public_key`/`aaguid` — a COSE public key and a credential handle are public, not
//! secret), a signature counter, role-free server-time instants. So [`AdminInviteRecord`] and
//! [`AdminCredential`] are **both** the store-return shape and the wire DTO. They live HERE (not the
//! Worker) so the field names are single-sourced in this drift-tracked crate (P4 — the seam the
//! spec-001 `ManifestPointer` miss came through).
//!
//! ## Wire conventions (frozen by the T03 contract; mirrored to the web's camelCase ports in T05)
//!
//! - **`snake_case` keys**, matching the rest of `/api/admin/*` (the member DTOs). The SvelteKit web
//!   tier maps them to its camelCase `InviteRecord`/`StoredCredential` ports (`webauthn/ports.ts`).
//! - **`bytea` fields → base64url, no padding** (`credential_id`/`public_key`/`aaguid`), via the
//!   [`b64`]/[`b64_opt`] adapters; held internally as `Vec<u8>` so the store constructs them from raw
//!   `bytea` and tests assert on bytes — the base64 happens only at serialize time (in the Worker).
//!   **Decode asymmetry at the T05 seam:** the web adapter base64url-**decodes** `public_key` back to
//!   a `Uint8Array` for `@simplewebauthn` (its `WebAuthnCredential.publicKey` is bytes), but leaves
//!   `credential_id` as the base64url **string** (its `StoredCredential.credentialId` is a string).
//! - **timestamps → epoch-seconds integers** ([`UnixSeconds`] serializes transparently as `i64`).
//!   `consumed_at`/`revoked_at` are explicitly nullable (`number | null`); `transports`/`aaguid` are
//!   omitted when absent (the optional `?` fields).
//!
//! These types are **PII-free in the I5 sense** (no decrypted member name/phone/address), so the new
//! endpoints are not `x-requires-audit` (AC13). The Worker (T04) blesses them
//! [`AuditedResponse`](crate::AuditedResponse) where it serializes them through `admin_response_body`.
//!
//! [`MemberId`]: boundless_domain::MemberId

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use boundless_auth::UnixSeconds;
use boundless_domain::MemberId;
use serde::{Serialize, Serializer};
use uuid::Uuid;

/// Serialize a `bytea` wire field as **base64url, no padding** (the WebAuthn-byte convention the web
/// tier + `@simplewebauthn` already speak for `credential_id`/`public_key`). Takes `&[u8]` so the
/// `&Vec<u8>` field reference deref-coerces in (and clippy's `ptr_arg` stays quiet).
fn b64<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&URL_SAFE_NO_PAD.encode(bytes))
}

/// Same as [`b64`] for an optional `bytea` (`aaguid`). Reached only for `Some` (paired with
/// `skip_serializing_if`), but total for safety.
fn b64_opt<S: Serializer>(opt: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error> {
    match opt {
        Some(bytes) => b64(bytes, serializer),
        None => serializer.serialize_none(),
    }
}

/// A pending-admin invitation row (`admin_invitations`) as B1 invite-resolve returns it — PII-free,
/// so it doubles as the wire DTO. The TTL/consumed *verdict* (`evaluateInvite`) is computed edge-TS
/// from `expires_at`/`consumed_at`; the HMAC token match runs in the core (ADR-0017 carve-out).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdminInviteRecord {
    /// The pending Admin (member) this invitation is for (the audited actor, post-registration).
    pub admin_id: MemberId,
    /// The Group the invitation belongs to (the Worker's single-install `GROUP_ID`, D3). Carried for
    /// the web port's completeness; never a security input (RLS is server-side).
    pub group_id: Uuid,
    /// Server-side TTL instant, epoch seconds (`admin_invitations.expires_at`).
    pub expires_at: UnixSeconds,
    /// Single-use marker, epoch seconds; `null` while live (`admin_invitations.consumed_at`).
    pub consumed_at: Option<UnixSeconds>,
}

/// A stored admin WebAuthn credential (`admin_webauthn_credentials`) — PII-free, so it doubles as the
/// wire DTO. An admin may hold more than one active credential (passkey + hardware backup, AC20).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdminCredential {
    /// The WebAuthn credential id (`credential_id` `bytea`) — base64url on the wire.
    #[serde(serialize_with = "b64")]
    pub credential_id: Vec<u8>,
    /// The owning Admin (member) id.
    pub admin_id: MemberId,
    /// The COSE public key bytes (`public_key` `bytea`) — base64url on the wire. Public, not secret.
    #[serde(serialize_with = "b64")]
    pub public_key: Vec<u8>,
    /// The WebAuthn signature counter (`sign_count` `bigint`). Advanced only-if-greater (R10).
    pub sign_count: i64,
    /// The authenticator transports, if known (omitted on the wire when absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transports: Option<Vec<String>>,
    /// The authenticator AAGUID bytes, if known (`aaguid` `bytea`) — base64url, omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "b64_opt")]
    pub aaguid: Option<Vec<u8>>,
    /// Revocation instant, epoch seconds; `null` while active (`revoked_at`).
    pub revoked_at: Option<UnixSeconds>,
}

/// The material to persist for a newly-registered admin credential
/// ([`insert_credential`](crate::AdminWebAuthnStore::insert_credential) /
/// [`register_complete`](crate::AdminWebAuthnStore::register_complete)). PII-free. The owning
/// `admin_id` is **not** carried here: `insert_credential` takes it explicitly, and `register_complete`
/// derives it from the just-consumed invitation row (never a web-supplied value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAdminCredential {
    /// The WebAuthn credential id (`bytea`).
    pub credential_id: Vec<u8>,
    /// The COSE public key bytes (`bytea`).
    pub public_key: Vec<u8>,
    /// The initial signature counter from the registration ceremony.
    pub sign_count: i64,
    /// The authenticator transports, if the ceremony reported them.
    pub transports: Option<Vec<String>>,
    /// The authenticator AAGUID bytes, if the ceremony reported them.
    pub aaguid: Option<Vec<u8>>,
}

/// The outcome of [`register_complete`](crate::AdminWebAuthnStore::register_complete) (R11 — one txn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterCompleteOutcome {
    /// The invitation was consumed, the admin's prior credentials revoked (D4), and the new
    /// credential inserted — all atomically. Carries the admin id derived from the consumed row.
    Completed {
        /// The Admin the consumed invitation named (server-derived, the audited actor).
        admin_id: MemberId,
    },
    /// The token matched no live (unconsumed) invitation in this tenant — already consumed, unknown,
    /// or cross-tenant. Nothing was written (the txn rolled back). Value-free: **no existence
    /// oracle** distinguishing the cases; edge-TS surfaces it as `ADMIN_INVITE_CONSUMED` (the TOCTOU
    /// backstop after the edge `evaluateInvite`).
    InviteNotConsumable,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin() -> MemberId {
        MemberId::from_uuid(Uuid::from_u128(0x11))
    }

    // The wire shape is FROZEN here (the spec-009 T03 contract mirrors it; the web T05 adapter maps
    // it). A field rename / reorder / encoding change must break these, exactly like the member
    // `member_detail_view_wire_keys_are_pinned` pins.

    #[test]
    fn admin_invite_record_wire_keys_are_pinned() {
        let live = AdminInviteRecord {
            admin_id: admin(),
            group_id: Uuid::from_u128(0x22),
            expires_at: UnixSeconds::new(100_000),
            consumed_at: None,
        };
        assert_eq!(
            serde_json::to_string(&live).unwrap(),
            r#"{"admin_id":"00000000-0000-0000-0000-000000000011","group_id":"00000000-0000-0000-0000-000000000022","expires_at":100000,"consumed_at":null}"#
        );

        let consumed = AdminInviteRecord {
            consumed_at: Some(UnixSeconds::new(50_000)),
            ..live
        };
        assert_eq!(
            serde_json::to_string(&consumed).unwrap(),
            r#"{"admin_id":"00000000-0000-0000-0000-000000000011","group_id":"00000000-0000-0000-0000-000000000022","expires_at":100000,"consumed_at":50000}"#
        );
    }

    #[test]
    fn admin_credential_wire_keys_are_pinned() {
        // Full credential: every optional present. credential_id=[1,2,3]→"AQID", public_key=[4,5,6]→
        // "BAUG", aaguid=[0xAB]→"qw" (base64url, no padding).
        let full = AdminCredential {
            credential_id: vec![1, 2, 3],
            admin_id: admin(),
            public_key: vec![4, 5, 6],
            sign_count: 7,
            transports: Some(vec!["usb".to_string(), "nfc".to_string()]),
            aaguid: Some(vec![0xAB]),
            revoked_at: None,
        };
        assert_eq!(
            serde_json::to_string(&full).unwrap(),
            r#"{"credential_id":"AQID","admin_id":"00000000-0000-0000-0000-000000000011","public_key":"BAUG","sign_count":7,"transports":["usb","nfc"],"aaguid":"qw","revoked_at":null}"#
        );

        // Minimal: optionals absent (omitted), revoked.
        let minimal = AdminCredential {
            credential_id: vec![1, 2, 3],
            admin_id: admin(),
            public_key: vec![4, 5, 6],
            sign_count: 0,
            transports: None,
            aaguid: None,
            revoked_at: Some(UnixSeconds::new(1234)),
        };
        assert_eq!(
            serde_json::to_string(&minimal).unwrap(),
            r#"{"credential_id":"AQID","admin_id":"00000000-0000-0000-0000-000000000011","public_key":"BAUG","sign_count":0,"revoked_at":1234}"#
        );
    }
}
