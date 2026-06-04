//! `POST /api/auth/recovery/rebind` — Driver self-serve device replacement with a Recovery Code
//! (AC19, ADR-0016 D3): re-bind a new device, invalidate the old token (I4), issue a fresh
//! Recovery Code (rotated on use), and mint a new session. Riders have **no** self-serve path —
//! a non-Driver is short-circuited to `NotAvailable` before any secret comparison.
//!
//! Like the other endpoints it carries the version handshake (AC7/O4) and degrades below
//! `client_min_version` without acting.

use boundless_auth::{
    evaluate_recovery_code, evaluate_version, recovery_available_for, RecoveryChallenge,
    RecoveryCodeVerdict, VersionRequirement,
};
use boundless_crypto::recovery_code_hash;
use boundless_domain::{ClientVersion, DeviceToken, PhoneNumber, RecoveryCode};

use crate::ports::{AdminAlertSink, AuthStore, SecretSource, SessionMaterial};
use crate::service::AuthService;

/// A recovery re-bind request. Holds tainted material (phone, code, push token), so not `Serialize`.
pub struct RecoveryRequest {
    /// The Driver's phone number, already canonicalized to E.164.
    pub phone: PhoneNumber,
    /// The Recovery Code the Driver holds.
    pub code: RecoveryCode,
    /// What the client reports about itself (platform + version).
    pub reported: ClientVersion,
    /// The push device token for the new device.
    pub device_token: DeviceToken,
}

/// The outcome of a recovery re-bind. `Rebound` carries tainted material (the session **and** the
/// freshly-issued Recovery Code the Driver must capture), so neither this nor [`RecoveryResponse`]
/// is `Serialize`/`Debug`.
pub enum RecoveryOutcome {
    /// Accepted: new device bound, old token invalidated, a fresh Recovery Code issued.
    Rebound {
        /// The new session.
        material: SessionMaterial,
        /// The fresh Recovery Code to show the Driver once (rotated on use, ADR-0016 D3).
        fresh_recovery_code: RecoveryCode,
    },
    /// Rejected — bad/used code (`Invalid`), or self-serve recovery is not available for this
    /// member's role (`NotAvailable`; Riders recover only via an Admin re-issue).
    Rejected(RecoveryCodeVerdict),
    /// The reporting client is below `client_min_version`; nothing was attempted (O4/O8).
    BelowMinVersion,
}

/// A recovery re-bind response — always carries the version requirement (AC7).
pub struct RecoveryResponse {
    /// `client_min_version` + `client_recommended_version` (AC7).
    pub version: VersionRequirement,
    /// What happened.
    pub outcome: RecoveryOutcome,
}

impl RecoveryResponse {
    /// The stable error code for this response, or `None` on a clean re-bind (P12).
    pub fn error_code(&self) -> Option<&'static str> {
        match &self.outcome {
            RecoveryOutcome::Rebound { .. } => None,
            RecoveryOutcome::Rejected(v) => v.error_code(),
            RecoveryOutcome::BelowMinVersion => Some("AUTH_BELOW_MIN_VERSION"),
        }
    }
}

impl<St, Sk, Sec, Clk> AuthService<St, Sk, Sec, Clk>
where
    St: AuthStore,
    Sk: AdminAlertSink,
    Sec: SecretSource,
    Clk: boundless_auth::Clock,
{
    /// Handle a Driver recovery re-bind (spec edge "device replacement / recovery" / AC19).
    pub fn recovery_rebind(&mut self, req: RecoveryRequest) -> RecoveryResponse {
        let now = self.clock.now();
        let verdict_v = evaluate_version(&req.reported.app_version, &self.config.requirement);
        let hash = self.lookup_hash(&req.phone);
        let member = self.store.find_member_by_phone(&hash);

        if verdict_v.is_below_minimum() {
            if let Some(m) = &member {
                self.note_below_min(m.member_id, req.reported.app_version);
            }
            return self.recovery_response(RecoveryOutcome::BelowMinVersion);
        }

        // No member for this phone: uniform `Invalid` (no existence leak).
        let Some(member) = member else {
            return self.recovery_response(RecoveryOutcome::Rejected(RecoveryCodeVerdict::Invalid));
        };
        let role = member.recovery_role();

        // Short-circuit non-Drivers before loading any challenge (AC19; the core gate also
        // enforces this, so the rule is single-sourced and cannot drift). The distinct
        // `NotAvailable` (Rider) vs `Invalid` (Driver/unknown) verdict is an intentional, bounded
        // role signal (it requires already knowing the phone, and matches the `AUTH_RECOVERY_*`
        // error codes / ADR-0016 D3 "Drivers self-serve, Riders call the admin"). sec-audit F3.
        if !recovery_available_for(role) {
            return self
                .recovery_response(RecoveryOutcome::Rejected(RecoveryCodeVerdict::NotAvailable));
        }

        // No live recovery code → `Invalid` (the Driver falls back to the Admin path).
        let Some(row) = self.store.load_live_recovery(member.member_id) else {
            return self.recovery_response(RecoveryOutcome::Rejected(RecoveryCodeVerdict::Invalid));
        };

        let challenge = RecoveryChallenge {
            // `load_live_recovery` returns only the live row; single-use is the atomic
            // `consume_and_rotate_recovery` below (defense-in-depth flags stay false here).
            code_hash: row.code_hash,
            consumed: false,
            superseded: false,
        };
        let verdict = evaluate_recovery_code(&challenge, &req.code, &self.config.hmac_key, role);

        match verdict {
            RecoveryCodeVerdict::Accepted => {
                // Mint + hash the fresh code, then atomically consume-and-rotate. A lost race
                // means the code was already used → `Invalid` (never a double re-bind).
                let fresh = self.secrets.fresh_recovery_code();
                let fresh_hash = recovery_code_hash(&self.config.hmac_key, &fresh);
                if !self
                    .store
                    .consume_and_rotate_recovery(member.member_id, fresh_hash, now)
                {
                    return self.recovery_response(RecoveryOutcome::Rejected(
                        RecoveryCodeVerdict::Invalid,
                    ));
                }
                let material = self.issue_session_and_bind(
                    member.member_id,
                    req.reported,
                    &req.device_token,
                    now,
                );
                self.recovery_response(RecoveryOutcome::Rebound {
                    material,
                    fresh_recovery_code: fresh,
                })
            }
            other => self.recovery_response(RecoveryOutcome::Rejected(other)),
        }
    }

    fn recovery_response(&self, outcome: RecoveryOutcome) -> RecoveryResponse {
        RecoveryResponse {
            version: self.requirement(),
            outcome,
        }
    }
}
