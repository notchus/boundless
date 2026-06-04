//! Client build targets.

use serde::{Deserialize, Serialize};

/// A client build target. It is one leg of the device-token binding tuple
/// `(member_id, platform, app_version)` (privacy invariant I4), so a member's push token
/// on their iPhone is distinct from the same member's token on an iPad or Android phone.
///
/// Wire form uses the canonical lowercase platform identifiers (`"ios"`, `"ipados"`,
/// `"watchos"`, `"macos"`, `"android"`, `"wearos"`, `"web"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    #[serde(rename = "ios")]
    Ios,
    #[serde(rename = "ipados")]
    IpadOs,
    #[serde(rename = "watchos")]
    WatchOs,
    #[serde(rename = "macos")]
    MacOs,
    #[serde(rename = "android")]
    Android,
    #[serde(rename = "wearos")]
    WearOs,
    /// The admin web (SvelteKit). Has no push device token; authenticates via WebAuthn.
    #[serde(rename = "web")]
    Web,
}
