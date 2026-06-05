//! `POST /api/dev/admins` — developer-only Admin provisioning (I11 / ADR-0015; AC1, AC16).
//!
//! Admins are issued **only** by the Developer (I11); there is no signup surface anywhere. This
//! module is the *functional core* of that endpoint: the authorization decision (AC1 — unauth and
//! admin-auth are both rejected) and the invitation mint (AC16 — a single-use, server-time-TTL,
//! PII-free registration token).
//!
//! ## Authorization is enforced by a capability type, not a comment
//!
//! [`create_admin`](crate::AuthService::create_admin) takes a [`DeveloperAuthority`] **by
//! reference**, and that type's only constructor is [`authorize_developer`] — which yields it for a
//! [`DevCaller::Developer`] and nothing else. So minting an Admin without developer authorization is
//! not merely policy; it does not type-check (P-style "enforce through code").
//!
//! ## What is here vs the deferred Worker shell (T08-shell)
//!
//! Here: the authz decision, the token mint + at-rest hashing, and the store contract (atomic
//! create / re-issue). **Deferred** (→ `DEFERRED.md`): the deployable `#[event]` route, the
//! **Developer hardware-key WebAuthn** verification that actually *establishes* a
//! [`DevCaller::Developer`] (and so constructs the authority), **Email Workers** delivery of the
//! registration link, and the invite **consume** on first WebAuthn registration (T09, edge TS).
//! The `created_by` audit actor (the Developer's identity) lands with that dev-auth verification.

use boundless_auth::{Clock, UnixSeconds};
use boundless_crypto::admin_invitation_token_hash;
use boundless_domain::{AdminInvitationToken, MemberId};

use crate::ports::{AdminProvisioningStore, SecretSource};
use crate::service::AuthService;

/// The Admin registration-invitation TTL: 72 hours (plan §10-D; AC16). Validated against **server
/// time** (the injected [`Clock`]), so a wrong device clock can neither extend nor deny it.
pub const INVITE_TTL_SECS: i64 = 72 * 60 * 60;

/// Who is calling a `/api/dev/*` endpoint, as classified by the Worker from the request. Only
/// [`DevCaller::Developer`] (hardware-key WebAuthn verified, established in the deferred shell) may
/// provision an Admin; every other caller is rejected (AC1, I11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevCaller {
    /// No credential presented.
    Unauthenticated,
    /// An authenticated **member** (Rider/Driver) — not the Developer.
    Member,
    /// An authenticated **Admin** — explicitly *cannot* create other Admins (only the Developer
    /// can; glossary, I11).
    Admin,
    /// The Developer, hardware-key (WebAuthn) verified.
    Developer,
}

/// The stable rejection of a non-Developer caller at the Admin-creation endpoint (AC1 / I11). The
/// code matches `docs/error-codes.md` (P12); it has no client surface (there is no signup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevAdminCreateForbidden;

impl DevAdminCreateForbidden {
    /// The stable error code (`docs/error-codes.md`, P12).
    pub const fn error_code(self) -> &'static str {
        "DEV_ADMIN_CREATE_FORBIDDEN"
    }
}

/// Proof that the caller is the Developer — the capability that gates Admin provisioning.
///
/// The single private field makes this **un-forgeable outside this module**: the only way to obtain
/// one is [`authorize_developer`]. Pass it by reference to [`AuthService::create_admin`] /
/// [`AuthService::reissue_admin_invite`]; holding one *is* the authorization (I11).
#[derive(Debug, Clone, Copy)]
pub struct DeveloperAuthority(());

/// Authorize a `/api/dev/*` caller: yield a [`DeveloperAuthority`] iff the caller is the Developer,
/// else [`DevAdminCreateForbidden`] (AC1 — unauthenticated **and** admin-authenticated are both
/// rejected; I11).
pub fn authorize_developer(
    caller: DevCaller,
) -> Result<DeveloperAuthority, DevAdminCreateForbidden> {
    match caller {
        DevCaller::Developer => Ok(DeveloperAuthority(())),
        DevCaller::Unauthenticated | DevCaller::Member | DevCaller::Admin => {
            Err(DevAdminCreateForbidden)
        }
    }
}

/// A freshly-minted Admin registration invitation, to be delivered out of band (Email Workers,
/// deferred). **PII-free by construction**: an opaque [`MemberId`] + an opaque single-use token +
/// an expiry — no phone, no name. It **holds the tainted [`AdminInvitationToken`]**, so it is
/// deliberately not `Debug`/`Serialize`: the Worker reveals the token only at the wire boundary
/// (`expose_secret`) to build the registration URL, and only its at-rest hash is ever persisted.
pub struct AdminInvitation {
    /// The pending Admin this invitation registers (opaque; never displayed).
    pub admin_id: MemberId,
    /// The single-use registration token (tainted; opaque to the recipient — ADR-0015).
    pub token: AdminInvitationToken,
    /// Server-side expiry instant (`now + `[`INVITE_TTL_SECS`]).
    pub expires_at: UnixSeconds,
}

impl<St, Sk, Sec, Clk> AuthService<St, Sk, Sec, Clk>
where
    St: AdminProvisioningStore,
    Sec: SecretSource,
    Clk: Clock,
{
    /// Provision a new pending Admin + mint its registration invitation (AC16). Requires a
    /// [`DeveloperAuthority`] **by type** — there is no way to call this without developer
    /// authorization (AC1, I11). The token is minted from the injected CSPRNG, hashed at rest
    /// (`admin_invitation_token_hash`), and returned in the clear **once** for out-of-band delivery;
    /// only the hash is stored.
    pub async fn create_admin(
        &mut self,
        _developer: &DeveloperAuthority,
    ) -> Result<AdminInvitation, St::Error> {
        let now = self.clock.now();
        let token = self.secrets.fresh_admin_invitation();
        let token_hash = admin_invitation_token_hash(&self.config.hmac_key, &token);
        let expires_at = now.saturating_add_secs(INVITE_TTL_SECS);
        let admin_id = self
            .store
            .create_pending_admin_with_invitation(token_hash, expires_at)
            .await?;
        Ok(AdminInvitation {
            admin_id,
            token,
            expires_at,
        })
    }

    /// Re-invite an existing pending Admin (lost-key recovery, ADR-0015): mint a fresh invitation
    /// that **supersedes** the admin's prior live one (single-use is preserved — the prior link is
    /// invalidated). Returns `None` (no invitation minted) iff `admin_id` is unknown, so a bad id is
    /// a no-op. Also developer-gated.
    pub async fn reissue_admin_invite(
        &mut self,
        _developer: &DeveloperAuthority,
        admin_id: MemberId,
    ) -> Result<Option<AdminInvitation>, St::Error> {
        let now = self.clock.now();
        let token = self.secrets.fresh_admin_invitation();
        let token_hash = admin_invitation_token_hash(&self.config.hmac_key, &token);
        let expires_at = now.saturating_add_secs(INVITE_TTL_SECS);
        let existed = self
            .store
            .reissue_admin_invitation(admin_id, token_hash, expires_at, now)
            .await?;
        Ok(existed.then_some(AdminInvitation {
            admin_id,
            token,
            expires_at,
        }))
    }
}
