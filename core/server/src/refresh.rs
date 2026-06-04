//! `POST /api/auth/refresh` — silent refresh-credential rotation with replay detection (AC18,
//! ADR-0016 D2), the version handshake (AC7/O4), and the rejected-refresh throttle counter.
//!
//! The client-facing outcome is deliberately **uniform** for every non-rotating case — unknown
//! credential, already-revoked family, **and** a detected replay all return `Invalidated` — so a
//! caller cannot distinguish "this credential was once valid" from "this credential is unknown"
//! (no lineage-existence leak; carry-forward (a)/(e)). The replay vs unknown distinction lives
//! only server-side: replay revokes the whole family and alerts the admin (AC15), and the exact
//! verdict is exposed as `server_verdict` for the Worker to **log** (never to return to the
//! client).

use boundless_auth::{
    evaluate_refresh, evaluate_version, RefreshVerdict, SessionFamilyStatus, VersionRequirement,
};
use boundless_domain::{ClientVersion, RefreshToken};

use crate::ports::{AdminAlertSink, AuthStore, SecretSource, SessionMaterial, SourceKey};
use crate::service::AuthService;

/// A refresh request. Holds the tainted presented credential, so it is not `Serialize`.
pub struct RefreshRequest {
    /// The refresh credential the client presented.
    pub presented: RefreshToken,
    /// What the client reports about itself (platform + version).
    pub reported: ClientVersion,
    /// An opaque per-source key (e.g. hashed client IP) for the rejected-refresh throttle.
    pub source: SourceKey,
}

/// The **client-facing** outcome of a refresh.
pub enum RefreshOutcome {
    /// Accepted: rotated to a fresh credential + access token.
    Rotated(SessionMaterial),
    /// Not accepted — unknown credential, revoked family, **or** a detected replay, all uniform
    /// (no lineage leak). The session is over; a Rider routes to help, a Driver may re-auth.
    Invalidated,
    /// The reporting client is below `client_min_version`; the session was left untouched (O4/O8).
    BelowMinVersion,
}

/// A refresh response.
pub struct RefreshResponse {
    /// `client_min_version` + `client_recommended_version` (AC7).
    pub version: VersionRequirement,
    /// The client-facing outcome (uniform on every rejection — no lineage leak).
    pub outcome: RefreshOutcome,
    /// **Server-side only** (operability): the exact rotation verdict. The Worker logs
    /// `server_verdict.error_code()` via the PII-free `emit()` path but **never** returns it to
    /// the client — returning the replay-vs-unknown distinction would leak lineage existence
    /// (carry-forward (a)/(e)). `None` when the version gate short-circuited before the policy ran.
    pub server_verdict: Option<RefreshVerdict>,
}

impl<St, Sk, Sec, Clk> AuthService<St, Sk, Sec, Clk>
where
    St: AuthStore,
    Sk: AdminAlertSink,
    Sec: SecretSource,
    Clk: boundless_auth::Clock,
{
    /// Handle a refresh (spec D / AC18). The classification (constant-time hash compare over the
    /// lineage) is the store's; the rotation/replay **policy** is `evaluate_refresh` (P4). Replay
    /// revokes the family atomically and alerts the admin once/day (AC15).
    pub fn refresh(&mut self, req: RefreshRequest) -> RefreshResponse {
        let now = self.clock.now();
        let verdict_v = evaluate_version(&req.reported.app_version, &self.config.requirement);
        let class = self
            .store
            .classify_refresh(&req.presented, &self.config.hmac_key);

        if verdict_v.is_below_minimum() {
            // Degrade without touching the session (the app is merely too old). Alert the
            // member's admin if the family identified one.
            if let Some(f) = &class.family {
                self.note_below_min(f.member, req.reported.app_version);
            }
            return RefreshResponse {
                version: self.requirement(),
                outcome: RefreshOutcome::BelowMinVersion,
                server_verdict: None,
            };
        }

        // An unknown credential has no family/status; treat it as revoked so the policy rejects it
        // (uniform with an already-revoked family).
        let status = class
            .family
            .as_ref()
            .map(|f| f.status)
            .unwrap_or(SessionFamilyStatus::Revoked);
        let verdict = evaluate_refresh(status, class.presentation);

        let outcome = match verdict {
            RefreshVerdict::Rotate => {
                let family = class
                    .family
                    .as_ref()
                    .expect("Rotate implies a classified Active family")
                    .id;
                RefreshOutcome::Rotated(self.rotate_session(family, now))
            }
            RefreshVerdict::ReplayDetectedKillFamily => {
                let family = class
                    .family
                    .as_ref()
                    .expect("replay implies a classified family");
                let (family_id, member) = (family.id, family.member);
                self.store.revoke_family(family_id, now);
                // The legitimate holder is now locked out → the admin is told (AC15), deduped.
                self.note_session_invalidated(member);
                RefreshOutcome::Invalidated
            }
            RefreshVerdict::Rejected => {
                // Throttle the source (the Worker enforces 429 above a threshold); the response
                // shape is identical to a revoked-family reject — no lineage leak.
                self.hub.note_refresh_rejection(req.source, now);
                RefreshOutcome::Invalidated
            }
        };

        RefreshResponse {
            version: self.requirement(),
            outcome,
            server_verdict: Some(verdict),
        }
    }
}
