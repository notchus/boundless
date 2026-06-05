//! The member-auth composition root: [`AuthService`] bundles the I/O ports + the injected
//! [`Clock`] + the [`GroupHubState`] + config, and the endpoint methods (in `signin`/`bind`/
//! `refresh`/`recovery`) hang off it. Bundling the dependencies as fields keeps each method's
//! signature small (no 9-argument free functions) and mirrors how the Worker wires the DO once
//! per request.
//!
//! Shared helpers live here: the version-handshake config, the access-token TTL, session
//! minting + device (re)binding, and the deduped below-min / session-invalidated alerts.

use boundless_auth::{
    reonboarding_invalidation, Clock, DeviceBinding, Session, UnixSeconds, VersionRequirement,
};
use boundless_crypto::{phone_lookup_hash, refresh_token_hash, HmacKey, PhoneLookupHash};
use boundless_domain::{
    AppVersion, ClientVersion, DeviceToken, MemberId, PhoneNumber, SessionFamilyId,
};
use serde::{Deserialize, Serialize};

use crate::alerts::{AdminAlert, AlertKind};
use crate::hub::GroupHubState;
use crate::ports::{AdminAlertSink, AuthStore, DeviceStore, SecretSource, SessionMaterial};

/// The short-lived access-token lifetime (~15 minutes; ADR-0016 D2 / plan §10-D). The client
/// silently refreshes shortly before this elapses; the session itself is indefinite (AC18).
pub const ACCESS_TTL_SECS: i64 = 15 * 60;

/// A pointer to the signed KV manifest the client fetches at launch (ADR-0014). Non-PII; carried
/// in the sign-in response. The client reads `index_key` first, then the per-locale manifest at
/// `{locale_key_prefix}{locale}` (spec §C.6) — e.g. `"manifest:v1:index"` then `"manifest:v1:en"`.
/// The Worker supplies both keys. Both legs are part of the frozen wire contract
/// (`fixtures/auth/signin_ok.json`, `api/openapi.yaml` `ManifestPointer`, spec 001 T10/AC7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestPointer {
    /// The KV index key the client reads first.
    pub index_key: String,
    /// The KV key prefix for the per-locale manifest; the client appends its locale (spec §C.6).
    pub locale_key_prefix: String,
}

impl ManifestPointer {
    /// Construct a manifest pointer from its index key and per-locale key prefix.
    pub fn new(index_key: impl Into<String>, locale_key_prefix: impl Into<String>) -> Self {
        Self {
            index_key: index_key.into(),
            locale_key_prefix: locale_key_prefix.into(),
        }
    }
}

/// Per-instance auth configuration: the HMAC secret, the version requirement advertised in every
/// response (AC7), the launch manifest pointer, and the access-token TTL. Not `Debug` (it holds
/// the [`HmacKey`], which is unloggable, P2).
pub struct AuthConfig {
    /// The per-instance HMAC secret (from Secrets Store in production).
    pub hmac_key: HmacKey,
    /// `client_min_version` + `client_recommended_version`, carried in every `/api/auth/*`
    /// response (O4/O5, AC7).
    pub requirement: VersionRequirement,
    /// The launch manifest pointer (ADR-0014), returned at sign-in.
    pub manifest_pointer: ManifestPointer,
    /// Access-token lifetime in seconds (default [`ACCESS_TTL_SECS`]).
    pub access_ttl_secs: i64,
}

impl AuthConfig {
    /// Construct a config with the default access-token TTL ([`ACCESS_TTL_SECS`]).
    pub fn new(
        hmac_key: HmacKey,
        requirement: VersionRequirement,
        manifest_pointer: ManifestPointer,
    ) -> Self {
        Self {
            hmac_key,
            requirement,
            manifest_pointer,
            access_ttl_secs: ACCESS_TTL_SECS,
        }
    }
}

/// The member-auth engine. Generic over its ports + clock so tests use in-memory doubles and the
/// Worker uses Cloudflare/Postgres impls; the endpoint methods (`sign_in`, `bind_device`,
/// `refresh`, `recovery_rebind`, `record_notification_decision`) live in the sibling modules.
pub struct AuthService<St, Sk, Sec, Clk> {
    /// The persistence boundary (members, codes, sessions, devices).
    pub store: St,
    /// Where deduped, PII-free admin alerts go.
    pub alerts: Sk,
    /// The injected source of fresh secrets (no ambient randomness in core).
    pub secrets: Sec,
    /// The per-Group rate-limit + alert-dedup counters (the DO state).
    pub hub: GroupHubState,
    /// The injected time source — **server time** in production (so a wrong device clock can
    /// neither grant nor deny; binding cannot complete offline).
    pub clock: Clk,
    /// Per-instance config.
    pub config: AuthConfig,
}

impl<St, Sk, Sec, Clk> AuthService<St, Sk, Sec, Clk>
where
    St: AuthStore,
    Sk: AdminAlertSink,
    Sec: SecretSource,
    Clk: Clock,
{
    /// Assemble the engine from its parts, starting with an empty [`GroupHubState`].
    pub fn new(store: St, alerts: Sk, secrets: Sec, clock: Clk, config: AuthConfig) -> Self {
        Self {
            store,
            alerts,
            secrets,
            hub: GroupHubState::new(),
            clock,
            config,
        }
    }

    /// The version requirement this instance advertises (AC7) — every endpoint response carries it.
    pub(crate) fn requirement(&self) -> VersionRequirement {
        self.config.requirement
    }

    /// The phone-lookup hash for `phone` under this instance's secret (I3).
    pub(crate) fn lookup_hash(&self, phone: &PhoneNumber) -> PhoneLookupHash {
        phone_lookup_hash(&self.config.hmac_key, phone)
    }

    /// Emit the below-`client_min_version` admin alert for `member`, at most once per day (O4/AC8).
    pub(crate) fn note_below_min(&mut self, member: MemberId, reported: AppVersion) {
        if self
            .hub
            .should_alert(member, AlertKind::BelowMinVersion, self.clock.now())
        {
            self.alerts.emit(AdminAlert::BelowMinVersion {
                member,
                reported_version: reported,
            });
        }
    }

    /// Record that `member`'s session was invalidated — emit at most one admin alert per member
    /// per day (AC15) so the admin can help a Rider re-establish auth. Called by the
    /// refresh-replay path here, and by the admin revoke/logout + deletion paths (spec 008 /
    /// `core::deletion`).
    ///
    /// This emits **only the alert** — the caller is responsible for the state change
    /// (`store.revoke_family` + the `invalidation_for` token scope). The refresh-replay path
    /// revokes the family *before* calling this; the admin-revoke/deletion orchestration legs
    /// land with spec 008 / `core::deletion`.
    pub fn note_session_invalidated(&mut self, member: MemberId) {
        if self
            .hub
            .should_alert(member, AlertKind::SessionInvalidated, self.clock.now())
        {
            self.alerts.emit(AdminAlert::SessionInvalidated { member });
        }
    }

    /// Mint a fresh session for `member`: a new refresh credential (stored hashed) + a fresh
    /// access token expiring at `now + access_ttl`. The refresh secret never touches the store in
    /// plaintext — only its [`refresh_token_hash`] (ADR-0016 D2 / I4).
    pub(crate) async fn mint_session(
        &mut self,
        member: MemberId,
        now: UnixSeconds,
    ) -> Result<SessionMaterial, St::Error> {
        let refresh = self.secrets.fresh_refresh();
        let access = self.secrets.fresh_access();
        let refresh_hash = refresh_token_hash(&self.config.hmac_key, &refresh);
        let access_expires_at = now.saturating_add_secs(self.config.access_ttl_secs);
        let session: Session = self
            .store
            .create_session_family(member, refresh_hash, access_expires_at, now)
            .await?;
        Ok(SessionMaterial {
            session,
            access,
            refresh,
        })
    }

    /// Rotate the family's refresh credential and mint a fresh access token (the `Rotate` arm of
    /// `/api/auth/refresh`).
    pub(crate) async fn rotate_session(
        &mut self,
        family: SessionFamilyId,
        now: UnixSeconds,
    ) -> Result<SessionMaterial, St::Error> {
        let refresh = self.secrets.fresh_refresh();
        let access = self.secrets.fresh_access();
        let refresh_hash = refresh_token_hash(&self.config.hmac_key, &refresh);
        let access_expires_at = now.saturating_add_secs(self.config.access_ttl_secs);
        let session = self
            .store
            .rotate_session(family, refresh_hash, access_expires_at, now)
            .await?;
        Ok(SessionMaterial {
            session,
            access,
            refresh,
        })
    }
}

impl<St, Sk, Sec, Clk> AuthService<St, Sk, Sec, Clk>
where
    St: AuthStore + DeviceStore,
    Sk: AdminAlertSink,
    Sec: SecretSource,
    Clk: Clock,
{
    /// (Re)bind the member's device and mint a session. Invalidates **all** the member's prior
    /// device bindings (decision: single active device per member; F5 "invalidate all" — no
    /// stale token survives) before registering the new one; the invalidation is silent
    /// (`AUTH_DEVICE_TOKEN_INVALIDATED`, never client-facing). Used by both device-bind (AC4) and
    /// recovery re-bind (AC19).
    ///
    /// Requires the [`DeviceStore`] port in addition to [`AuthStore`]; everything it touches goes
    /// through one backend, so `St::Error` is the single shared [`StoreBackend::Error`].
    pub(crate) async fn issue_session_and_bind(
        &mut self,
        member: MemberId,
        reported: ClientVersion,
        token: &DeviceToken,
        now: UnixSeconds,
    ) -> Result<SessionMaterial, St::Error> {
        let new_binding = DeviceBinding::new(member, reported.platform, reported.app_version);
        for prior in self.store.current_device_bindings(member).await? {
            if reonboarding_invalidation(&prior, &new_binding).is_some() {
                self.store.invalidate_device(&prior, now).await?;
            }
        }
        self.store.register_device(&new_binding, token, now).await?;
        self.mint_session(member, now).await
    }
}
