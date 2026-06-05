//! `POST /api/auth/signin` — phone-hash lookup with **no existence leak** (AC1/AC3), the version
//! handshake (AC7/O4), and the below-min admin alert (AC8).
//!
//! The response shape is uniform whether or not the phone matched: the version requirement and
//! manifest pointer are always present, and the match outcome is carried as a
//! `boundless_auth::SignInResult` (the same core decision the client re-derives, P4). The lookup
//! does the full constant-time hash work even on a miss, and a below-minimum handshake collapses
//! the outcome to `BelowMinVersion` (revealing nothing about the phone) — so a network observer
//! cannot distinguish matched from unmatched (carry-forward (b)).

use boundless_auth::{evaluate_version, SignInResult, VersionRequirement};
use boundless_domain::{ClientVersion, PhoneNumber};
use serde::{Deserialize, Serialize};

use crate::ports::{AdminAlertSink, AuthStore, SecretSource};
use crate::service::{AuthService, ManifestPointer};

/// A sign-in request. Carries the **normalized** phone (via `crate::normalize_phone`) — a tainted
/// type, so the request is not `Serialize` (its plaintext never crosses the wire implicitly, I3).
pub struct SignInRequest {
    /// The member's phone number, already canonicalized to E.164.
    pub phone: PhoneNumber,
    /// What the client reports about itself (platform + version).
    pub reported: ClientVersion,
}

/// A sign-in response. PII-free (the result is an opaque outcome, not the member), so it derives
/// `Serialize`/`Debug` and is safe to log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignInResponse {
    /// `client_min_version` + `client_recommended_version` (AC7) — present on every response.
    pub version: VersionRequirement,
    /// The launch manifest pointer (ADR-0014).
    pub manifest_pointer: ManifestPointer,
    /// The interpreted outcome: matched / not-on-file / below-min (the client routes on this,
    /// re-deriving the same decision from `version` — P4).
    pub result: SignInResult,
}

impl SignInResponse {
    /// The stable error code for this response, or `None` on a clean match (P12).
    pub fn error_code(&self) -> Option<&'static str> {
        match self.result {
            SignInResult::MemberMatched => None,
            SignInResult::PhoneNotOnFile => Some("AUTH_PHONE_NOT_ON_FILE"),
            SignInResult::BelowMinVersion => Some("AUTH_BELOW_MIN_VERSION"),
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
    /// Handle a sign-in (spec C.2 / AC7 / AC8). The version verdict is computed first (O4); the
    /// member lookup always runs (uniform timing); a below-min handshake collapses the outcome to
    /// `BelowMinVersion` and emits the once-per-day admin alert when the member is known.
    pub async fn sign_in(&mut self, req: SignInRequest) -> Result<SignInResponse, St::Error> {
        let verdict = evaluate_version(&req.reported.app_version, &self.config.requirement);

        // Always perform the lookup — no early return on below-min or on a miss — so the path is
        // timing-uniform and never branches observably on existence (carry-forward (b)).
        let hash = self.lookup_hash(&req.phone);
        let member = self.store.find_member_by_phone(&hash).await?;
        let result = SignInResult::from_lookup(member.is_some(), verdict);

        if result == SignInResult::BelowMinVersion {
            // Alert the member's admin (deduped). When the phone did not match there is no member
            // to help, so no alert fires — and since the alert goes to the admin via Queues (not
            // the client), this keying never leaks existence to the caller.
            if let Some(m) = &member {
                self.note_below_min(m.member_id, req.reported.app_version);
            }
        }

        Ok(SignInResponse {
            version: self.requirement(),
            manifest_pointer: self.config.manifest_pointer.clone(),
            result,
        })
    }
}
