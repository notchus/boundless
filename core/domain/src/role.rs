//! Member roles.

use serde::{Deserialize, Serialize};

/// A member's role within a Group (glossary). A single person may hold several roles
/// across contexts (e.g. a Driver on weekdays who also Rides on weekends, or Sarah who
/// is both Admin and a member) — see ADR-0016 and the spec's "member holds multiple
/// roles" edge case. Roles are selected post-onboarding without re-authentication.
///
/// Wire form is `snake_case`: `"rider"`, `"driver"`, `"admin"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// A group member who needs transportation to the Gathering; in by default.
    Rider,
    /// A member with a car who has flipped their Seat Toggle on.
    Driver,
    /// A trusted member who manages membership; issued only by the Developer (I11).
    Admin,
}
