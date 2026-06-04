//! App / client version value types.
//!
//! These are *value types only*. The N-2 backward-compatibility **policy** — comparing a
//! reported version against the server's `client_min_version` to decide the
//! `BelowMinVersion` degradation (operational invariants O1/O4) — lives in `core::auth`,
//! not here. This module just models a version and its natural semantic ordering.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A semantic application version, `major.minor.patch`.
///
/// Ordering is semantic: by `major`, then `minor`, then `patch` (the derived `Ord`
/// matches the field declaration order). Serializes as a string — `"1.2.0"` — so it
/// reads naturally in the auth contract (`client_min_version`,
/// `client_recommended_version`) and in the signed KV manifest (ADR-0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl AppVersion {
    /// Construct from explicit components.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for AppVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Why an `AppVersion` string could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppVersionParseError {
    /// Not exactly three dot-separated components.
    WrongComponentCount(usize),
    /// A component was not a non-negative integer that fits in `u32`.
    NonNumericComponent,
}

impl fmt::Display for AppVersionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongComponentCount(n) => write!(
                f,
                "expected 3 dot-separated components (major.minor.patch), found {n}"
            ),
            Self::NonNumericComponent => {
                f.write_str("each version component must be a non-negative integer")
            }
        }
    }
}

impl std::error::Error for AppVersionParseError {}

impl FromStr for AppVersion {
    type Err = AppVersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(AppVersionParseError::WrongComponentCount(parts.len()));
        }
        let component = |p: &str| {
            p.parse::<u32>()
                .map_err(|_| AppVersionParseError::NonNumericComponent)
        };
        Ok(Self {
            major: component(parts[0])?,
            minor: component(parts[1])?,
            patch: component(parts[2])?,
        })
    }
}

impl Serialize for AppVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // `collect_str` formats via `Display` without an intermediate allocation.
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for AppVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AppVersionVisitor;

        impl Visitor<'_> for AppVersionVisitor {
            type Value = AppVersion;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(r#"a version string "major.minor.patch""#)
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<AppVersion, E> {
                AppVersion::from_str(value).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(AppVersionVisitor)
    }
}

/// What a client reports about itself at the auth handshake: which build target it is and
/// which version it runs. `core::auth` compares `app_version` against the server's
/// `client_min_version` (O4) to decide whether to route to the calm `BelowMinVersion`
/// screen. The matching engine never reads this (O6) — it is purely an auth-layer signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientVersion {
    pub platform: crate::Platform,
    pub app_version: AppVersion,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_from_str_round_trip() {
        let v = AppVersion::new(1, 2, 0);
        assert_eq!(v.to_string(), "1.2.0");
        assert_eq!("1.2.0".parse::<AppVersion>().unwrap(), v);
        assert_eq!(
            "10.20.30".parse::<AppVersion>().unwrap(),
            AppVersion::new(10, 20, 30)
        );
    }

    #[test]
    fn from_str_rejects_malformed() {
        assert_eq!(
            "1.2".parse::<AppVersion>(),
            Err(AppVersionParseError::WrongComponentCount(2))
        );
        assert_eq!(
            "1.2.3.4".parse::<AppVersion>(),
            Err(AppVersionParseError::WrongComponentCount(4))
        );
        assert_eq!(
            "".parse::<AppVersion>(),
            Err(AppVersionParseError::WrongComponentCount(1))
        );
        assert_eq!(
            "1.x.0".parse::<AppVersion>(),
            Err(AppVersionParseError::NonNumericComponent)
        );
        assert_eq!(
            "1.2.-1".parse::<AppVersion>(),
            Err(AppVersionParseError::NonNumericComponent)
        );
    }

    #[test]
    fn serde_uses_string_form() {
        let v = AppVersion::new(2, 0, 5);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#""2.0.5""#);
        let back: AppVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn ordering_is_semantic_not_lexicographic() {
        assert!(AppVersion::new(1, 0, 0) < AppVersion::new(1, 0, 1));
        assert!(AppVersion::new(1, 0, 1) < AppVersion::new(1, 1, 0));
        assert!(AppVersion::new(1, 1, 0) < AppVersion::new(2, 0, 0));
        // The reason we don't compare version *strings*: "1.10.0" would sort BEFORE
        // "1.2.0" lexicographically, which is wrong. Numeric ordering gets it right.
        assert!(AppVersion::new(1, 2, 0) < AppVersion::new(1, 10, 0));
    }
}
