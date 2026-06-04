//! Device-token binding and the session/token invalidation triggers (privacy invariant I4;
//! spec AC4/AC18; ADR-0016 D2).
//!
//! A push device token is bound to the tuple `(member_id, platform, app_version)` ([`DeviceBinding`],
//! I4), so a member's token on their iPhone is distinct from the same member's token on an
//! iPad or Android phone. This module owns two pure decisions (P4):
//!
//! 1. **What ends a session and clears tokens** — the exhaustive, admin-mediated
//!    [`InvalidationTrigger`] set (there is deliberately *no* time/inactivity trigger; that is
//!    the indefinite-session guarantee, see [`crate::session`]).
//! 2. **Which tokens a re-onboarding clears** — registering a new device for a member
//!    invalidates the member's prior device token (AC4), and never touches a *different*
//!    member's device ([`reonboarding_invalidation`]).
//!
//! The actual push registration (APNs/FCM) and the Postgres `device_tokens` rows are the
//! server's (T07 / the push spec); this module is the policy those consume.

use boundless_domain::{AppVersion, MemberId, Platform};
use serde::{Deserialize, Serialize};

/// The stable error code emitted when a `(member_id, platform, app_version)` device token is
/// invalidated (`docs/error-codes.md`, P12). Silent to the client, which simply re-registers
/// a token on its next bind; recorded for operability.
pub const DEVICE_TOKEN_INVALIDATED_CODE: &str = "AUTH_DEVICE_TOKEN_INVALIDATED";

/// The identity a push device token is bound to (I4): the member, their platform, and the app
/// version. Distinct tuples hold distinct tokens; an auth change invalidates the member's
/// token(s) per [`invalidation_for`] / [`reonboarding_invalidation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceBinding {
    /// Whose device this is.
    pub member_id: MemberId,
    /// The client build target (one leg of the I4 tuple).
    pub platform: Platform,
    /// The app version the token was registered under (one leg of the I4 tuple).
    pub app_version: AppVersion,
}

impl DeviceBinding {
    /// Construct a binding from the I4 tuple.
    pub const fn new(member_id: MemberId, platform: Platform, app_version: AppVersion) -> Self {
        Self {
            member_id,
            platform,
            app_version,
        }
    }
}

/// The events that end a member session and invalidate its device token(s) (ADR-0016 D2).
///
/// This set is **exhaustive and entirely admin-mediated** — there is no time-based or
/// inactivity variant, by construction. Adding a variant here is the only way to introduce a
/// new way for a session to end, which keeps the indefinite-session model honest (AC18) and
/// forces every consumer (and the exhaustiveness tests) to account for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationTrigger {
    /// An admin revoked the member, or the member logged out.
    AdminRevokeOrLogout,
    /// The member bound a **new device** (re-onboarding); the prior device's token is
    /// invalidated (AC4, I4).
    NewDeviceReonboarding,
    /// The member's account was deleted (I12).
    AccountDeletion,
}

/// Which of a member's device tokens an invalidation clears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenInvalidationScope {
    /// **All** of the member's device tokens (admin revoke/logout, account deletion).
    AllForMember,
    /// Only the **previously-bound device's** token — a new device replaces it (re-onboarding,
    /// AC4's "the prior device's token").
    PriorDevice,
}

/// The effect of an [`InvalidationTrigger`]: whether it ends the session and which device
/// tokens it clears. Every current trigger ends the session (there is no "soft" trigger).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInvalidation {
    /// Whether the (prior) session is ended by this event. Always `true` for the current set.
    pub ends_session: bool,
    /// Which of the member's device tokens are invalidated.
    pub token_scope: TokenInvalidationScope,
}

impl SessionInvalidation {
    /// Whether this invalidation ends the session (and so revokes the family — the session is
    /// then dead, routing a Rider to help and a Driver to interactive re-auth).
    pub const fn is_session_ended(&self) -> bool {
        self.ends_session
    }
}

/// The invalidation effect of a trigger (ADR-0016 D2; AC18). Single-sourced so every endpoint
/// agrees on what each event does. The match is **wildcard-free**: a new
/// [`InvalidationTrigger`] variant will fail to compile here until its effect is defined.
pub const fn invalidation_for(trigger: InvalidationTrigger) -> SessionInvalidation {
    match trigger {
        // An auth change for the whole member: drop every device token (I4).
        InvalidationTrigger::AdminRevokeOrLogout | InvalidationTrigger::AccountDeletion => {
            SessionInvalidation {
                ends_session: true,
                token_scope: TokenInvalidationScope::AllForMember,
            }
        }
        // A new device supersedes the old one: invalidate the prior device's token (AC4).
        InvalidationTrigger::NewDeviceReonboarding => SessionInvalidation {
            ends_session: true,
            token_scope: TokenInvalidationScope::PriorDevice,
        },
    }
}

/// Decide what happens to a `prior` device binding when the same member re-onboards a `new`
/// device (AC4, I4). Returns the invalidation to apply to the prior device iff the bindings
/// belong to the **same member** — a different member's device is never touched (a
/// cross-member isolation property). Same-member is the only condition: re-binding always
/// supersedes the member's previous device, regardless of platform/app-version leg.
pub fn reonboarding_invalidation(
    prior: &DeviceBinding,
    new: &DeviceBinding,
) -> Option<SessionInvalidation> {
    if prior.member_id == new.member_id {
        Some(invalidation_for(InvalidationTrigger::NewDeviceReonboarding))
    } else {
        None
    }
}

/// The platform-appropriate secure store a refresh credential must be held in (plan §10-F).
/// Each is the platform's hardware-backed / server-side secret store — never plaintext app
/// storage (`UserDefaults`/`@AppStorage`/`localStorage` are forbidden-patterns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecureStoreClass {
    /// Apple Keychain (iOS/iPadOS/watchOS/macOS).
    AppleKeychain,
    /// Android EncryptedSharedPreferences / Keystore-backed storage (phone + Wear).
    AndroidKeystore,
    /// An httpOnly, Secure, SameSite=Strict server-side session cookie (admin web).
    ServerSideHttpOnlyCookie,
}

/// The required secure store for the refresh credential on `platform` (plan §10-F). A
/// single-sourced core contract the UI tasks (T11–T15) read instead of re-deciding storage.
/// The match is wildcard-free so a new [`Platform`] must declare its store here.
pub const fn required_refresh_store(platform: Platform) -> SecureStoreClass {
    match platform {
        Platform::Ios | Platform::IpadOs | Platform::WatchOs | Platform::MacOs => {
            SecureStoreClass::AppleKeychain
        }
        Platform::Android | Platform::WearOs => SecureStoreClass::AndroidKeystore,
        Platform::Web => SecureStoreClass::ServerSideHttpOnlyCookie,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn member() -> MemberId {
        MemberId::from_uuid(Uuid::from_u128(1))
    }

    #[test]
    fn reonboarding_invalidates_prior_device_token() {
        let prior = DeviceBinding::new(member(), Platform::Ios, AppVersion::new(1, 1, 0));
        let new = DeviceBinding::new(member(), Platform::Ios, AppVersion::new(1, 2, 0));
        let outcome = reonboarding_invalidation(&prior, &new).expect("same member → invalidated");
        assert!(outcome.is_session_ended());
        assert_eq!(outcome.token_scope, TokenInvalidationScope::PriorDevice);
    }

    #[test]
    fn reonboarding_never_touches_a_different_member() {
        let prior = DeviceBinding::new(member(), Platform::Ios, AppVersion::new(1, 1, 0));
        let other = DeviceBinding::new(
            MemberId::from_uuid(Uuid::from_u128(2)),
            Platform::Ios,
            AppVersion::new(1, 2, 0),
        );
        assert_eq!(reonboarding_invalidation(&prior, &other), None);
    }
}
