//! `POST /api/auth/bind-device` — the Onboarding-Code device bind (AC17), the device-token
//! binding + prior-token invalidation (AC4/I4), the rate-limit lock + admin alert (AC17/R4), and
//! the version handshake (AC7/O4). Plus `record_notification_decision` (AC14).
//!
//! Server-time validated (the injected clock is server time), so a bind **cannot complete
//! offline** (spec OQ8) and a wrong device clock cannot grant/deny it. The accept commit goes
//! through the atomic `consume_onboarding_if_live` so two concurrent presentations of one live
//! code can never both bind (carry-forward (a)).

use boundless_auth::{
    evaluate_onboarding_code, evaluate_version, should_flag_notifications_off,
    OnboardingCodeChallenge, OnboardingCodeVerdict, VersionRequirement,
};
use boundless_domain::{ClientVersion, DeviceToken, MemberId, OnboardingCode, PhoneNumber};

use crate::alerts::{AdminAlert, AlertKind};
use crate::ports::{AdminAlertSink, AuthStore, SecretSource, SessionMaterial};
use crate::service::AuthService;

/// A device-bind request. Holds tainted material (the code, the push token, the phone), so it is
/// not `Serialize`. The phone re-identifies the member; `reported` supplies both the version
/// handshake and the `(platform, app_version)` legs of the device binding (I4).
pub struct BindRequest {
    /// The member's phone number, already canonicalized to E.164.
    pub phone: PhoneNumber,
    /// The Onboarding Code the helper entered.
    pub code: OnboardingCode,
    /// What the client reports about itself (platform + version).
    pub reported: ClientVersion,
    /// The push device token the client obtained (APNs/FCM), to bind to this member/device.
    pub device_token: DeviceToken,
}

/// The outcome of a device-bind. `Bound` carries tainted [`SessionMaterial`], so neither this nor
/// [`BindResponse`] is `Serialize`/`Debug`.
pub enum BindOutcome {
    /// The code was accepted; the device is bound and a session issued.
    Bound(SessionMaterial),
    /// The code was rejected; the carried verdict's error code identifies why (all route to the
    /// same calm `BindingFailed` screen / `onboarding.binding.code_invalid` copy).
    Failed(OnboardingCodeVerdict),
    /// The reporting client is below `client_min_version`; the bind was not attempted (O4/O8).
    BelowMinVersion,
}

/// A device-bind response — always carries the version requirement (AC7).
pub struct BindResponse {
    /// `client_min_version` + `client_recommended_version` (AC7).
    pub version: VersionRequirement,
    /// What happened.
    pub outcome: BindOutcome,
}

impl BindResponse {
    /// The stable error code for this response, or `None` on a clean bind (P12).
    pub fn error_code(&self) -> Option<&'static str> {
        match &self.outcome {
            BindOutcome::Bound(_) => None,
            BindOutcome::Failed(v) => v.error_code(),
            BindOutcome::BelowMinVersion => Some("AUTH_BELOW_MIN_VERSION"),
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
    /// Handle a device bind (spec C.3 / AC17 / AC4). Order: version gate (O4) → resolve member
    /// (uniform) → rate-limit window → load + evaluate the code (server-time, full gate order) →
    /// on accept, **atomic** consume then issue session + (re)bind device.
    pub fn bind_device(&mut self, req: BindRequest) -> BindResponse {
        let now = self.clock.now();
        let verdict_v = evaluate_version(&req.reported.app_version, &self.config.requirement);

        // Resolve the member uniformly (the lookup hash work runs regardless of the version gate).
        let hash = self.lookup_hash(&req.phone);
        let member = self.store.find_member_by_phone(&hash);

        // Below-min degrades before the rate-limit window is charged or the code is compared — by
        // design (the app is too old to act). A below-min request therefore never consumes an
        // attempt and never probes the code, so it is not a rate-limit-bypass oracle: to actually
        // test a code an attacker must report a supported version, which *does* charge the window.
        if verdict_v.is_below_minimum() {
            if let Some(m) = &member {
                self.note_below_min(m.member_id, req.reported.app_version);
            }
            return self.bind_response(BindOutcome::BelowMinVersion);
        }

        // No member for this phone: same shape as a bad code — `Invalid` (no existence leak).
        let Some(member) = member else {
            return self.bind_response(BindOutcome::Failed(OnboardingCodeVerdict::Invalid));
        };
        let member_id = member.member_id;

        // Rate-limit bookkeeping: the count of PRIOR attempts in this window gates the verdict.
        let prior_attempts = self.hub.register_code_attempt(member_id, now);

        // No live code (never issued, or already consumed/superseded): the helper must ask the
        // admin for a fresh one — surfaced as `Consumed` (AC17).
        let Some(row) = self.store.load_live_onboarding(member_id) else {
            return self.bind_response(BindOutcome::Failed(OnboardingCodeVerdict::Consumed));
        };

        let challenge = OnboardingCodeChallenge {
            code_hash: row.code_hash,
            expires_at: row.expires_at,
            max_attempts: row.max_attempts,
            recent_attempts: prior_attempts,
            // `load_live_onboarding` returns only the live row (not consumed/superseded); the
            // single-use guarantee is the atomic `consume_onboarding_if_live` below, not these
            // flags. They are kept on the challenge as defense-in-depth in the pure decision.
            consumed: false,
            superseded: false,
        };
        let verdict =
            evaluate_onboarding_code(&challenge, &req.code, &self.config.hmac_key, &self.clock);

        match verdict {
            OnboardingCodeVerdict::Accepted => {
                // Atomic consume: a lost race means another presentation already bound — return
                // `Consumed`, never a second bind (carry-forward (a)).
                if !self.store.consume_onboarding_if_live(member_id, now) {
                    return self
                        .bind_response(BindOutcome::Failed(OnboardingCodeVerdict::Consumed));
                }
                let material =
                    self.issue_session_and_bind(member_id, req.reported, &req.device_token, now);
                self.bind_response(BindOutcome::Bound(material))
            }
            OnboardingCodeVerdict::RateLimited => {
                if self
                    .hub
                    .should_alert(member_id, AlertKind::OnboardingCodeLocked, now)
                {
                    self.alerts
                        .emit(AdminAlert::OnboardingCodeLocked { member: member_id });
                }
                self.bind_response(BindOutcome::Failed(OnboardingCodeVerdict::RateLimited))
            }
            other => self.bind_response(BindOutcome::Failed(other)),
        }
    }

    /// Record the onboarding notification-permission decision (AC14). On a decline, record the
    /// non-PII "notifications not enabled" admin flag (deduped per day) and return `true`; the
    /// flow always advances client-side either way (never block/scold). Returns whether the
    /// decline was flagged.
    pub fn record_notification_decision(&mut self, member: MemberId, granted: bool) -> bool {
        let flag = should_flag_notifications_off(granted);
        if flag
            && self
                .hub
                .should_alert(member, AlertKind::NotificationsNotEnabled, self.clock.now())
        {
            self.alerts
                .emit(AdminAlert::NotificationsNotEnabled { member });
        }
        flag
    }

    fn bind_response(&self, outcome: BindOutcome) -> BindResponse {
        BindResponse {
            version: self.requirement(),
            outcome,
        }
    }
}
