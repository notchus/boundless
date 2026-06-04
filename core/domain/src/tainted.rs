//! Tainted secret/PII newtypes (constitution **P2**).
//!
//! Each type wraps a secret `String` and deliberately implements **no `Debug`, no
//! `Display`, and no `Serialize`/`Deserialize`**. The only way to read a summary is
//! [`redacted_summary`](PhoneNumber::redacted_summary), which never contains secret
//! material; the only way to reach the raw value is the intentionally-alarming
//! `expose_secret`, used solely at the crypto/wire boundary (`core::crypto` in T03,
//! `core::auth` in T04/T05).
//!
//! Why no `Serialize`? Defense in depth. A [`PhoneNumber`] *cannot* be accidentally
//! serialized into a request or a log line — its plaintext never crosses the wire (only
//! the HMAC lookup hash does, I3). Wire DTOs convert explicitly at the boundary, which
//! keeps the conversion auditable instead of implicit.
//!
//! The `assert_not_impl_any!` checks in the test module turn "no formatter" into a
//! *compile-time* guarantee: if anyone ever adds `#[derive(Debug)]` to one of these, the
//! crate fails to build.

/// Defines a tainted secret newtype with the standard P2 surface and nothing else.
macro_rules! tainted_secret {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        ///
        /// Tainted (P2): no `Debug`/`Display`/`Serialize`. Use
        /// [`redacted_summary`](Self::redacted_summary) for logging and
        /// `expose_secret` only at the crypto/wire boundary.
        #[derive(Clone)]
        pub struct $name(String);

        impl $name {
            /// Wrap a raw secret value.
            pub fn new(secret: impl Into<String>) -> Self {
                Self(secret.into())
            }

            /// Reveal the raw secret.
            ///
            /// **Danger:** the returned value is plaintext PII/secret. Never log it, never
            /// persist it unencrypted (P2). This exists only so `core::crypto` can hash or
            /// seal it and `core::auth` can put it on the wire at an explicit boundary.
            pub fn expose_secret(&self) -> &str {
                &self.0
            }

            /// A privacy-safe, leak-free summary for use anywhere `Debug` would otherwise
            /// be used. Always `"<TypeName>(redacted)"`; never contains secret material.
            pub fn redacted_summary(&self) -> String {
                format!("{}(redacted)", stringify!($name))
            }
        }
    };
}

tainted_secret!(
    /// A member's phone number — their human-facing identity. Per I3 the plaintext is
    /// **never** sent in an auth lookup (only its HMAC-SHA256 hash) and **never** logged.
    PhoneNumber
);

tainted_secret!(
    /// A push device token, bound to `(member_id, platform, app_version)` and invalidated
    /// on auth change / new-device re-onboarding (I4).
    DeviceToken
);

tainted_secret!(
    /// The admin-issued, single-use, short-TTL first-launch device-binding secret
    /// (glossary; ADR-0016 D1). Server-validated; carries no PII.
    OnboardingCode
);

tainted_secret!(
    /// The driver-held, single-use self-serve device-replacement secret, rotated on use
    /// (glossary; ADR-0016 D3).
    RecoveryCode
);

tainted_secret!(
    /// A short-lived bearer access token (~15 min; ADR-0016 D2 / plan §10-D).
    AccessToken
);

tainted_secret!(
    /// A long-lived, rotating refresh credential — indefinite lifetime with silent
    /// rotation and replay detection (ADR-0016 D2).
    RefreshToken
);

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_not_impl_any;

    // P2 (the core invariant T02 must enforce): tainted types expose no formatter.
    // These are compile-time assertions — adding Debug/Display fails the build.
    assert_not_impl_any!(PhoneNumber: core::fmt::Debug, core::fmt::Display);
    assert_not_impl_any!(DeviceToken: core::fmt::Debug, core::fmt::Display);
    assert_not_impl_any!(OnboardingCode: core::fmt::Debug, core::fmt::Display);
    assert_not_impl_any!(RecoveryCode: core::fmt::Debug, core::fmt::Display);
    assert_not_impl_any!(AccessToken: core::fmt::Debug, core::fmt::Display);
    assert_not_impl_any!(RefreshToken: core::fmt::Debug, core::fmt::Display);

    // Defense in depth: they must not be serde-serializable either (I3 — plaintext never
    // crosses the wire implicitly).
    assert_not_impl_any!(PhoneNumber: serde::Serialize, serde::de::DeserializeOwned);
    assert_not_impl_any!(DeviceToken: serde::Serialize, serde::de::DeserializeOwned);
    assert_not_impl_any!(OnboardingCode: serde::Serialize, serde::de::DeserializeOwned);
    assert_not_impl_any!(RecoveryCode: serde::Serialize, serde::de::DeserializeOwned);
    assert_not_impl_any!(AccessToken: serde::Serialize, serde::de::DeserializeOwned);
    assert_not_impl_any!(RefreshToken: serde::Serialize, serde::de::DeserializeOwned);

    #[test]
    fn redacted_summary_never_leaks_and_expose_secret_round_trips() {
        // A sentinel that would be unmistakable if it leaked into a summary.
        let sentinel = "SUPERSECRET-d3adb33f-0118-999";

        macro_rules! check {
            ($t:ty) => {{
                let value = <$t>::new(sentinel);
                let summary = value.redacted_summary();
                assert!(
                    !summary.contains("SUPERSECRET"),
                    "{} redacted_summary leaked the secret: {summary}",
                    stringify!($t)
                );
                assert!(!summary.contains(sentinel));
                assert_eq!(summary, format!("{}(redacted)", stringify!($t)));
                // The raw secret is still reachable for the crypto/wire boundary.
                assert_eq!(value.expose_secret(), sentinel);
                // Cloning preserves the secret.
                assert_eq!(value.clone().expose_secret(), sentinel);
            }};
        }

        check!(PhoneNumber);
        check!(DeviceToken);
        check!(OnboardingCode);
        check!(RecoveryCode);
        check!(AccessToken);
        check!(RefreshToken);
    }
}
