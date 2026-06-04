//! Client-version compatibility — the O4 below-minimum gate and the O1 N-2 support window.
//!
//! Every `/api/auth/*` response and WebSocket open handshake carries `client_min_version`
//! (O4) and `client_recommended_version` (O5's straggler signal). The decision of whether a
//! reporting client is too old — and therefore must see the calm `BelowMinVersion`
//! degradation screen with **no** "Update Now" control (O8) — lives here in the core (P4),
//! so every platform decides identically. The matching engine never reads any of this (O6).
//!
//! This module owns only the *comparison*; routing the verdict to an [`OnboardingState`] is
//! [`crate::state`], and emitting the rate-limited admin alert is the server (T07).
//!
//! [`OnboardingState`]: crate::OnboardingState

use boundless_domain::AppVersion;
use serde::{Deserialize, Serialize};

/// The version policy an auth response asserts: the minimum supported version (O4) and the
/// recommended version (O5). Carried in every `/api/auth/*` response (spec AC7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionRequirement {
    /// Clients below this are routed to the calm degradation screen (O4).
    pub min: AppVersion,
    /// The version stragglers are nudged toward via the admin panel (O5). Informational to
    /// the client; never a rider-facing prompt (O8).
    pub recommended: AppVersion,
}

impl VersionRequirement {
    /// Construct from a minimum and a recommended version.
    pub const fn new(min: AppVersion, recommended: AppVersion) -> Self {
        Self { min, recommended }
    }
}

/// The outcome of comparing a reported client version against a [`VersionRequirement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionVerdict {
    /// At or above the recommended version — fully current.
    Supported,
    /// At or above the minimum but below the recommended version — works, but a straggler
    /// the admin panel surfaces (O5). Still no rider-facing update prompt (O8).
    SupportedButOutdated,
    /// Below the minimum supported version — must show the calm degradation screen (O4/O8).
    BelowMinimum,
}

impl VersionVerdict {
    /// The stable error code for this verdict, or `None` when no degradation applies
    /// (`docs/error-codes.md`, P12).
    pub const fn error_code(self) -> Option<&'static str> {
        match self {
            Self::Supported | Self::SupportedButOutdated => None,
            Self::BelowMinimum => Some("AUTH_BELOW_MIN_VERSION"),
        }
    }

    /// Whether this verdict forces the `BelowMinVersion` degradation (O4).
    pub const fn is_below_minimum(self) -> bool {
        matches!(self, Self::BelowMinimum)
    }
}

/// Compare a `reported` client version against the server's `requirement` (O4/O5).
///
/// Uses [`AppVersion`]'s semantic ordering (major, then minor, then patch) — never a
/// lexicographic string compare, so `1.10.0` correctly ranks above `1.2.0`.
///
/// `recommended` is treated as `max(recommended, min)` so that a malformed requirement with
/// `recommended < min` (a server-config error — both fields come from the signed config) can
/// never misclassify a below-minimum client as merely outdated: the below-minimum gate is
/// checked first and independently.
pub fn evaluate_version(reported: &AppVersion, requirement: &VersionRequirement) -> VersionVerdict {
    let recommended = requirement.recommended.max(requirement.min);
    if *reported < requirement.min {
        VersionVerdict::BelowMinimum
    } else if *reported < recommended {
        VersionVerdict::SupportedButOutdated
    } else {
        VersionVerdict::Supported
    }
}

/// The oldest version the server still supports given an N-2-style window (O1): the server
/// supports the `current` minor and the `n_minus` previous **minor** versions.
///
/// The window is computed within the current major (it does not roll back across a major
/// bump): `1.2.x` with `n_minus = 2` yields `1.0.0`; `1.1.x` yields `1.0.0` (floored, not
/// `1.-1.0`). This expresses the server's support policy once, in the core, so the
/// `client_min_version` it advertises and the verdict the client computes cannot drift.
pub fn minimum_supported(current: AppVersion, n_minus: u32) -> AppVersion {
    let floor_minor = current.minor.saturating_sub(n_minus);
    AppVersion::new(current.major, floor_minor, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> VersionRequirement {
        VersionRequirement::new(AppVersion::new(1, 0, 0), AppVersion::new(1, 2, 0))
    }

    #[test]
    fn below_min_is_degradation() {
        let v = evaluate_version(&AppVersion::new(0, 9, 0), &req());
        assert_eq!(v, VersionVerdict::BelowMinimum);
        assert!(v.is_below_minimum());
        assert_eq!(v.error_code(), Some("AUTH_BELOW_MIN_VERSION"));
    }

    #[test]
    fn at_min_is_supported_but_outdated() {
        let v = evaluate_version(&AppVersion::new(1, 0, 0), &req());
        assert_eq!(v, VersionVerdict::SupportedButOutdated);
        assert_eq!(v.error_code(), None);
    }

    #[test]
    fn at_or_above_recommended_is_supported() {
        assert_eq!(
            evaluate_version(&AppVersion::new(1, 2, 0), &req()),
            VersionVerdict::Supported
        );
        assert_eq!(
            evaluate_version(&AppVersion::new(1, 10, 0), &req()),
            VersionVerdict::Supported
        );
    }

    #[test]
    fn minimum_supported_floors_within_major() {
        assert_eq!(
            minimum_supported(AppVersion::new(1, 2, 5), 2),
            AppVersion::new(1, 0, 0)
        );
        // Floors at .0 rather than underflowing the minor.
        assert_eq!(
            minimum_supported(AppVersion::new(1, 1, 0), 2),
            AppVersion::new(1, 0, 0)
        );
    }
}
