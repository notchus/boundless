//! Admin alerts / flags emitted by the auth layer — **PII-free by construction** (O4/AC8,
//! AC14, AC15, AC17; R12; I8/P2).
//!
//! In the deployable Worker (T07-shell) these are delivered to the admin via Cloudflare
//! Queues; the [`GroupHubState`](crate::hub::GroupHubState) dedups them to **at most one per
//! `(member, kind, day)`** so a flapping client cannot flood the admin (R12). Here in the core
//! we define the payloads and the dedup key.
//!
//! Why this is PII-free *structurally*: every field is a non-tainted value — an opaque
//! [`MemberId`] (not PII; the deletion stand-in, I12) and a version string. The tainted types
//! (`PhoneNumber`/`DeviceToken`/`…`) are **not** `Serialize`, so one could not be placed in an
//! `#[derive(Serialize)]` alert even by mistake — the crate would fail to compile (P2/I8).

use boundless_domain::{AppVersion, MemberId};
use serde::{Deserialize, Serialize};

/// A non-PII admin alert/flag the auth layer emits to the per-Group admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdminAlert {
    /// A device reported a version below `client_min_version` (O4/AC8). The admin nudges the
    /// household to update; the rider sees only the calm degradation screen, never a prompt.
    BelowMinVersion {
        /// The member whose device is too old.
        member: MemberId,
        /// The version the client reported (non-PII; for the O5 stragglers panel).
        reported_version: AppVersion,
    },
    /// A previously-valid session was invalidated and the member needs help re-establishing it
    /// (AC15) — a Rider cannot self-recover, so the admin is told.
    SessionInvalidated {
        /// The member whose session ended.
        member: MemberId,
    },
    /// Onboarding-Code bind attempts hit the rate limit and the code is locked (AC17 / R4).
    OnboardingCodeLocked {
        /// The member whose code is locked.
        member: MemberId,
    },
    /// Notification permission was declined at onboarding, so doorbell notifications are not
    /// enabled (AC14). Recorded, never blocking or scolding.
    NotificationsNotEnabled {
        /// The member who declined.
        member: MemberId,
    },
}

impl AdminAlert {
    /// The member this alert concerns (the dedup subject).
    pub fn member(&self) -> MemberId {
        match self {
            Self::BelowMinVersion { member, .. }
            | Self::SessionInvalidated { member }
            | Self::OnboardingCodeLocked { member }
            | Self::NotificationsNotEnabled { member } => *member,
        }
    }

    /// The kind of this alert — the second half of the `(member, kind, day)` dedup key.
    pub fn kind(&self) -> AlertKind {
        match self {
            Self::BelowMinVersion { .. } => AlertKind::BelowMinVersion,
            Self::SessionInvalidated { .. } => AlertKind::SessionInvalidated,
            Self::OnboardingCodeLocked { .. } => AlertKind::OnboardingCodeLocked,
            Self::NotificationsNotEnabled { .. } => AlertKind::NotificationsNotEnabled,
        }
    }

    /// The stable error/operational code for this alert (`docs/error-codes.md`, P12).
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::BelowMinVersion { .. } => "AUTH_BELOW_MIN_VERSION",
            Self::SessionInvalidated { .. } => "AUTH_SESSION_INVALIDATED",
            Self::OnboardingCodeLocked { .. } => "AUTH_ONBOARDING_CODE_RATE_LIMITED",
            Self::NotificationsNotEnabled { .. } => "AUTH_NOTIFICATIONS_NOT_ENABLED",
        }
    }
}

/// The kind of an [`AdminAlert`], without its data — the dedup discriminant the GroupHub keys
/// "one alert per member per day" on (so two distinct kinds for one member on one day each
/// emit once, but the same kind twice emits once).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertKind {
    /// [`AdminAlert::BelowMinVersion`].
    BelowMinVersion,
    /// [`AdminAlert::SessionInvalidated`].
    SessionInvalidated,
    /// [`AdminAlert::OnboardingCodeLocked`].
    OnboardingCodeLocked,
    /// [`AdminAlert::NotificationsNotEnabled`].
    NotificationsNotEnabled,
}
