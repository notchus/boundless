//! The member session model: indefinite sessions, silent refresh-token rotation, and
//! replay/lineage detection (ADR-0016 D2; spec AC18; plan §10-D).
//!
//! This is the **core leg** of the session lifecycle — the policy, defined once (P4) so every
//! platform decides identically and no endpoint can drift. The server (**T07**) owns the
//! *enforcement*: the Postgres `sessions` lineage chain, the refresh credential's at-rest
//! HMAC hashing, the DB lookup that classifies a presented credential (see
//! [`RefreshPresentation`]), access-token signing, and the wall-clock TTL source. This module
//! owns the rotation/replay **decision** and the indefinite-liveness rule.
//!
//! ## Why indefinite sessions are safe (ADR-0016 D2, risk register R5/R6)
//!
//! A member session never expires from time or inactivity ([`Session::is_live`] ignores the
//! clock) — that is the constitutional promise that Maria is never dropped to a sign-in form
//! on a routine weekly open (P10/AC15). The accepted residual of a long-lived refresh
//! credential is bounded by **three compensating controls**: refresh-token rotation (here),
//! device binding ([`crate::device`], I4), and a working admin revoke
//! ([`crate::device::InvalidationTrigger`]). The control this module enforces is rotation
//! with **replay detection**: presenting a rotated-away (stale) credential revokes the entire
//! family ([`RefreshVerdict::ReplayDetectedKillFamily`]) — for everyone, including the
//! legitimate holder — so a stolen-then-rotated token cannot be quietly reused. The test
//! `auth_refresh_rotation_replay_detected` is the sole enforced control behind R6.
//!
//! ## At-rest storage contract (plan §10-F) — consumed by the UI tasks (T11–T15)
//!
//! The long-lived refresh credential (`boundless_domain::RefreshToken`) must be held in the
//! platform's **secure** store, never in plaintext app storage:
//!
//! - **Apple** (iOS/iPadOS/watchOS/macOS): the **Keychain** — never `UserDefaults`/`@AppStorage`.
//! - **Android** (phone/Wear): **EncryptedSharedPreferences / Keystore-backed** storage.
//! - **Admin web**: an **httpOnly, Secure, SameSite=Strict** server-side session cookie set
//!   post-WebAuthn-assertion — never `localStorage`.
//!
//! [`crate::device::required_refresh_store`] expresses this mapping as a testable core
//! contract so the platforms read it rather than re-deciding it.

use boundless_domain::{MemberId, SessionFamilyId};
use serde::{Deserialize, Serialize};

use crate::clock::Clock;

/// The lifecycle status of a session *family* — one member login and the lineage of all its
/// rotated refresh credentials (ADR-0016 D2). A family is born `Active` and, once `Revoked`
/// (by replay detection or any [`crate::device::InvalidationTrigger`]), never returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionFamilyStatus {
    /// Live — silent refresh keeps issuing access tokens; the member stays signed in.
    Active,
    /// Ended — admin revoke/logout, new-device re-onboarding, account deletion, or a detected
    /// refresh replay. A revoked family never rotates again; the member must re-establish auth
    /// (a Rider via help, a Driver interactively — see [`crate::reauth_state_for`]).
    Revoked,
}

impl SessionFamilyStatus {
    /// Whether the family is still live.
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

/// A member's session as the client/core reasons about it (ADR-0016 D2).
///
/// Deliberately **PII-free and free of secret material**: it carries the access-token
/// *expiry* (so the client knows when to silently refresh), never the access or refresh token
/// itself — those tainted secrets (`boundless_domain::{AccessToken, RefreshToken}`) live only
/// in the platform secure store (the §10-F contract above), so a `Session` is safe to log and
/// serialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Whose session this is.
    pub member_id: MemberId,
    /// The refresh-credential lineage this session belongs to (revoked as a unit on replay).
    pub family_id: SessionFamilyId,
    /// When the current short-lived access token (~15 min, plan §10-D) expires. Compared only
    /// against the **injected** clock — never a device clock — by [`Session::needs_refresh`].
    pub access_token_expires_at: crate::clock::UnixSeconds,
    /// The family's lifecycle status — the *only* thing that determines liveness (below).
    pub family_status: SessionFamilyStatus,
}

impl Session {
    /// Whether the session is still live. **Time-independent by design** (ADR-0016 D2): a
    /// session is live iff its family is `Active`, regardless of how long the device has been
    /// idle or whether the access token has expired. This is the indefinite-session guarantee
    /// behind AC18 — an expired access token triggers a silent refresh, not a sign-out.
    pub const fn is_live(&self) -> bool {
        self.family_status.is_active()
    }

    /// Whether the access token should be silently refreshed *now*, given the injected clock
    /// and a `skew_secs` lead time (refresh shortly before expiry to avoid a race). Only a
    /// **live** session refreshes; a revoked family is dead and routes to re-auth, never a
    /// silent refresh. The device clock never participates — `clock` is server time in
    /// production (consistent with the wrong-device-clock edge case).
    ///
    /// `skew_secs` is unsigned so a negative lead time is unrepresentable; the widening to
    /// `i64` is lossless (a `u32` of seconds is ~136 years, far beyond any token TTL) and the
    /// add is saturating, so no overflow path exists.
    pub fn needs_refresh(&self, clock: &impl Clock, skew_secs: u32) -> bool {
        self.is_live()
            && clock.now().saturating_add_secs(i64::from(skew_secs)) >= self.access_token_expires_at
    }
}

/// How the server's lineage lookup classified a *presented* refresh credential within its
/// family. The classification itself is a server concern — a DB lineage lookup plus the
/// constant-time hash compare over the at-rest refresh hashes (T07) — but the **policy** on
/// the classification ([`evaluate_refresh`]) is single-sourced here so the rotation and
/// replay rules cannot drift between endpoints (P4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshPresentation {
    /// Matched the family's **current** (only valid) refresh credential — the happy path.
    Current,
    /// Matched a **prior, already-rotated** credential of the family — a replay. The genuine
    /// client always holds the current credential, so a superseded one resurfacing means it
    /// leaked; the only safe response is to kill the family.
    Superseded,
    /// Matched **no** credential of any live family (unknown or already-revoked lineage).
    Unknown,
}

/// The outcome of evaluating a presented refresh credential (ADR-0016 D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshVerdict {
    /// Accept: issue a fresh access token and **rotate** the refresh credential (the server
    /// supersedes the presented one and advances the family's current credential).
    Rotate,
    /// A rotated-away credential was replayed → **revoke the whole family** (R6). Every
    /// credential in the lineage, including the legitimate current one, stops working.
    ReplayDetectedKillFamily,
    /// Reject: an unknown credential, or the family is already revoked. The session is dead.
    Rejected,
}

impl RefreshVerdict {
    /// The stable error code for this verdict, or `None` when the refresh succeeds
    /// (`docs/error-codes.md`, P12).
    pub const fn error_code(self) -> Option<&'static str> {
        match self {
            Self::Rotate => None,
            Self::ReplayDetectedKillFamily => Some("AUTH_REFRESH_REPLAY_DETECTED"),
            // A rejected refresh ends the session — the same end-state (and code) as any
            // other invalidation. Single-source the literal via `state` so the two can't drift.
            Self::Rejected => Some(crate::state::SESSION_INVALIDATED_CODE),
        }
    }

    /// Whether the refresh is accepted (and the credential rotated).
    pub const fn is_accepted(self) -> bool {
        matches!(self, Self::Rotate)
    }

    /// The family's status after applying this verdict to a family that was `prior`.
    ///
    /// A replay revokes the family unconditionally — this is *how* replay detection kills the
    /// lineage. A successful rotation or a plain rejection leaves the family's status
    /// unchanged (a `Rotate` only happens on an `Active` family; a `Rejected` does not
    /// resurrect a revoked one or kill a live one — an unknown credential is simply ignored).
    pub const fn resulting_family_status(self, prior: SessionFamilyStatus) -> SessionFamilyStatus {
        match self {
            Self::ReplayDetectedKillFamily => SessionFamilyStatus::Revoked,
            Self::Rotate | Self::Rejected => prior,
        }
    }
}

/// Evaluate a presented refresh credential against its family (ADR-0016 D2). Pure policy; the
/// classification ([`RefreshPresentation`]) and the resulting persistence are the server's.
///
/// - A `Revoked` family never rotates — any presentation is `Rejected` (the dead family stays
///   dead; even a `Superseded` match adds nothing, it is already revoked).
/// - On an `Active` family: the `Current` credential rotates; a `Superseded` credential is a
///   replay that kills the family; an `Unknown` credential is rejected.
pub const fn evaluate_refresh(
    status: SessionFamilyStatus,
    presented: RefreshPresentation,
) -> RefreshVerdict {
    match (status, presented) {
        (SessionFamilyStatus::Revoked, _) => RefreshVerdict::Rejected,
        (SessionFamilyStatus::Active, RefreshPresentation::Current) => RefreshVerdict::Rotate,
        (SessionFamilyStatus::Active, RefreshPresentation::Superseded) => {
            RefreshVerdict::ReplayDetectedKillFamily
        }
        (SessionFamilyStatus::Active, RefreshPresentation::Unknown) => RefreshVerdict::Rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{FixedClock, UnixSeconds};
    use uuid::Uuid;

    fn session(expires_at: i64, status: SessionFamilyStatus) -> Session {
        Session {
            member_id: MemberId::from_uuid(Uuid::nil()),
            family_id: SessionFamilyId::from_uuid(Uuid::nil()),
            access_token_expires_at: UnixSeconds::new(expires_at),
            family_status: status,
        }
    }

    #[test]
    fn active_family_rotates_current_credential() {
        assert_eq!(
            evaluate_refresh(SessionFamilyStatus::Active, RefreshPresentation::Current),
            RefreshVerdict::Rotate
        );
    }

    #[test]
    fn live_session_is_time_independent() {
        // A live family is live even long past the access-token expiry — refresh, not sign-out.
        let s = session(1_000, SessionFamilyStatus::Active);
        assert!(s.is_live());
        assert!(s.needs_refresh(&FixedClock::at_secs(10_000), 0));
        assert!(s.is_live()); // still live; needing a refresh does not end the session
    }
}
