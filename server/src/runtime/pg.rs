//! The deployable Worker's real data path (spec 001 **T07-shell-B**, PgAuthStore slice) — wasm-only.
//!
//! This replaces the `T07-shell-B slice 1` in-memory scaffold: the Worker now talks to a real
//! Postgres over a **Hyperdrive `worker::Socket`** and runs the core [`AuthService`] over the
//! real [`PgAuthStore`] (P4 — server logic single-sourced). It holds:
//!
//! - [`connect_pg`] — the transport: `env.hyperdrive("HYPERDRIVE")?.connect()? -> worker::Socket`
//!   driven through `tokio_postgres::Config::connect_raw(socket, NoTls)`, with the `Connection`
//!   future spawned via `wasm_bindgen_futures::spawn_local` (Workers are single-threaded — no
//!   `tokio::spawn`). `NoTls` is correct because Hyperdrive terminates TLS. A fresh connection per
//!   request is exactly the shape Hyperdrive's pooler expects — and why every `PgAuthStore` query
//!   uses the unnamed `query_typed*` family (ADR-0024).
//! - [`PgService`] — the composed store the engine drives: the **real** [`PgAuthStore`] for the
//!   `AuthStore` port + an **in-memory** `DeviceStore` half. `AuthService` is generic over a store
//!   that implements *both* ports; the wired sign-in route touches only the `AuthStore` half. The
//!   Postgres `DeviceStore` (`register_device` → reversibly-encrypted token) needs the spec-008
//!   at-rest encryption primitive, so it stays in memory until then. Mirrors the proven
//!   `server/store/tests/service_pg.rs::PgService` composition verbatim.
//! - [`WorkerClock`] (server time from the JS runtime — never a device clock) and [`BufferSink`]
//!   (drains the §10-E PII-free admin alerts to the `ADMIN_ALERTS` Queue), moved here from the
//!   deleted scaffold.
//! - [`PlaceholderSecrets`] — a fail-closed [`SecretSource`]: sign-in mints nothing, so its methods
//!   are `unreachable!`. A route that actually mints (bind-device / refresh / recovery) **must**
//!   swap in the production `RngSecretSource` over an injected CSPRNG (ADR-0021) — landing with the
//!   bind-device slice — or this panics loudly rather than issue a fake credential.
//!
//! **No PII / no secrets are logged or returned** (P2): [`connect_pg`] maps every failure to a
//! generic, value-free message — the connection string (and any Postgres detail) never reach the
//! wire or a log. The deferred scrubbed `emit()` sink (T16-shell) will add operability logging.

use std::str::FromStr;

use boundless_auth::{Clock, DeviceBinding, Session, UnixSeconds};
use boundless_crypto::{CodeHash, GroupKey, HmacKey, Nonce, PhoneLookupHash, RefreshTokenHash};
use boundless_domain::{
    AccessToken, AdminInvitationToken, DeviceToken, MemberId, RecoveryCode, RefreshToken,
    SessionFamilyId,
};
use boundless_server_core::{
    AdminAlert, AdminAlertSink, AuthStore, DeviceStore, MemberRecord, OnboardingCodeRow,
    RecoveryCodeRow, RefreshClassification, SecretSource, StoreBackend,
};
use boundless_server_store::{PgAuthStore, StoreError};
use tokio_postgres::{Client, Config, NoTls};
use uuid::Uuid;
use worker::Env;

/// The Hyperdrive binding name (declared in `wrangler.toml`).
const HYPERDRIVE_BINDING: &str = "HYPERDRIVE";

/// Connect to Postgres over the Hyperdrive `Socket` and return a ready `tokio-postgres` `Client`.
///
/// Per-request connect (the pattern Hyperdrive's pooler expects). The returned `Connection` future
/// is detached with `spawn_local`; it drives socket I/O while the request's queries run and
/// completes when the `Client` (owned by the request's `AuthService`) drops.
///
/// Every failure maps to a **generic, value-free** `worker::Error` — the connection string and any
/// Postgres message are never surfaced (P2).
pub(crate) async fn connect_pg(env: &Env) -> worker::Result<Client> {
    let hd = env
        .hyperdrive(HYPERDRIVE_BINDING)
        .map_err(|_| worker::Error::RustError("HYPERDRIVE binding missing".into()))?;
    // `connect()` is synchronous + fallible (worker 0.8.3): it hands back a `Socket` that impls
    // tokio `AsyncRead` + `AsyncWrite`.
    let socket = hd
        .connect()
        .map_err(|_| worker::Error::RustError("db connect failed".into()))?;
    // `connect_raw` connects over the supplied stream and uses only user / password / dbname /
    // options / application_name from the parsed config — it ignores host/port/connect_timeout (the
    // Socket already points at the pooler). Cited: docs.rs/tokio-postgres/0.7.17 Config::connect_raw
    // ("suitable in environments where prepared statements aren't supported (such as Cloudflare
    // Workers with Hyperdrive)"). `Config::from_str` parses Hyperdrive's connection-string URL.
    let config = Config::from_str(&hd.connection_string())
        .map_err(|_| worker::Error::RustError("db config invalid".into()))?;
    let (client, connection) = config
        .connect_raw(socket, NoTls)
        .await
        .map_err(|_| worker::Error::RustError("db handshake failed".into()))?;
    worker::wasm_bindgen_futures::spawn_local(async move {
        // Drop the driver's result: a transport error surfaces as a failed query, not here.
        let _ = connection.await;
    });
    Ok(client)
}

/// Load the per-instance HMAC key (I3) from the `HMAC_KEY` binding (a 64-char hex string). At deploy
/// this is a `wrangler secret`; for local/CI tests a `[vars]` entry — `env.var` reads both as
/// plaintext. Production sources this from Secrets Store (forbidden-patterns: no hardcoded secrets).
pub(crate) fn load_hmac_key(env: &Env) -> worker::Result<HmacKey> {
    let raw = env
        .var("HMAC_KEY")
        .map_err(|_| worker::Error::RustError("HMAC_KEY missing".into()))?
        .to_string();
    let bytes =
        decode_hex_32(&raw).ok_or_else(|| worker::Error::RustError("HMAC_KEY malformed".into()))?;
    Ok(HmacKey::from_bytes(bytes))
}

/// Load this install's single Group id (glossary: one install = one Group) from the `GROUP_ID`
/// binding — the tenant every `PgAuthStore` query is RLS-scoped to.
pub(crate) fn load_group_id(env: &Env) -> worker::Result<Uuid> {
    let raw = env
        .var("GROUP_ID")
        .map_err(|_| worker::Error::RustError("GROUP_ID missing".into()))?
        .to_string();
    Uuid::parse_str(raw.trim()).map_err(|_| worker::Error::RustError("GROUP_ID malformed".into()))
}

/// Decode exactly 32 bytes from a 64-char hex string (no dependency — `hex`/`base64` are not Worker
/// deps). Returns `None` on any non-hex byte or a wrong length.
fn decode_hex_32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let b = s.as_bytes();
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = (b[2 * i] as char).to_digit(16)?;
        let lo = (b[2 * i + 1] as char).to_digit(16)?;
        *slot = (hi * 16 + lo) as u8;
    }
    Some(out)
}

// ===== the composed store (PgAuthStore + in-memory DeviceStore) =================================
// Mirrors server/store/tests/service_pg.rs::PgService — keep the two in step until the real
// `PgDeviceStore` (spec-008 token encryption) replaces the in-memory device half.

struct DeviceEntry {
    binding: DeviceBinding,
    invalidated: bool,
}

/// The store the Worker's `AuthService` drives: the **real** `PgAuthStore` (`AuthStore`) + an
/// in-memory `DeviceStore`. The shared [`StoreError`] lets the orchestration's `?` unify the ports.
pub(crate) struct PgService {
    pg: PgAuthStore,
    devices: Vec<DeviceEntry>,
}

impl PgService {
    /// Wrap a `PgAuthStore` with an empty in-memory device half.
    pub(crate) fn new(pg: PgAuthStore) -> Self {
        Self {
            pg,
            devices: Vec::new(),
        }
    }
}

impl StoreBackend for PgService {
    type Error = StoreError;
}

impl AuthStore for PgService {
    async fn find_member_by_phone(
        &mut self,
        hash: &PhoneLookupHash,
    ) -> Result<Option<MemberRecord>, StoreError> {
        self.pg.find_member_by_phone(hash).await
    }
    async fn load_live_onboarding(
        &mut self,
        member: MemberId,
    ) -> Result<Option<OnboardingCodeRow>, StoreError> {
        self.pg.load_live_onboarding(member).await
    }
    async fn consume_onboarding_if_live(
        &mut self,
        member: MemberId,
        now: UnixSeconds,
    ) -> Result<bool, StoreError> {
        self.pg.consume_onboarding_if_live(member, now).await
    }
    async fn classify_refresh(
        &mut self,
        presented: &RefreshToken,
        key: &HmacKey,
    ) -> Result<RefreshClassification, StoreError> {
        self.pg.classify_refresh(presented, key).await
    }
    async fn rotate_session(
        &mut self,
        family: SessionFamilyId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<Session, StoreError> {
        self.pg
            .rotate_session(family, new_refresh_hash, access_expires_at, now)
            .await
    }
    async fn revoke_family(
        &mut self,
        family: SessionFamilyId,
        now: UnixSeconds,
    ) -> Result<(), StoreError> {
        self.pg.revoke_family(family, now).await
    }
    async fn create_session_family(
        &mut self,
        member: MemberId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<Session, StoreError> {
        self.pg
            .create_session_family(member, new_refresh_hash, access_expires_at, now)
            .await
    }
    async fn load_live_recovery(
        &mut self,
        member: MemberId,
    ) -> Result<Option<RecoveryCodeRow>, StoreError> {
        self.pg.load_live_recovery(member).await
    }
    async fn consume_and_rotate_recovery(
        &mut self,
        member: MemberId,
        fresh_hash: CodeHash,
        now: UnixSeconds,
    ) -> Result<bool, StoreError> {
        self.pg
            .consume_and_rotate_recovery(member, fresh_hash, now)
            .await
    }
}

// In-memory device half (the Postgres impl is deferred — see the module docs). Same
// upsert-on-(member,platform,app_version) + invalidate-all semantics as the core/server test stub.
impl DeviceStore for PgService {
    async fn current_device_bindings(
        &mut self,
        member: MemberId,
    ) -> Result<Vec<DeviceBinding>, StoreError> {
        Ok(self
            .devices
            .iter()
            .filter(|d| d.binding.member_id == member && !d.invalidated)
            .map(|d| d.binding)
            .collect())
    }
    async fn invalidate_device(
        &mut self,
        binding: &DeviceBinding,
        _now: UnixSeconds,
    ) -> Result<(), StoreError> {
        for d in &mut self.devices {
            if &d.binding == binding {
                d.invalidated = true;
            }
        }
        Ok(())
    }
    async fn register_device(
        &mut self,
        binding: &DeviceBinding,
        _token: &DeviceToken,
        _now: UnixSeconds,
    ) -> Result<(), StoreError> {
        self.devices.retain(|d| &d.binding != binding);
        self.devices.push(DeviceEntry {
            binding: *binding,
            invalidated: false,
        });
        Ok(())
    }
}

// ===== non-store ports ==========================================================================

/// Buffers the §10-E PII-free admin alerts the core decides to emit, so the sign-in handler can
/// drain them to the `ADMIN_ALERTS` Queue after the call.
#[derive(Default)]
pub(crate) struct BufferSink {
    alerts: Vec<AdminAlert>,
}

impl BufferSink {
    /// Take the alerts emitted during the last endpoint call, leaving the buffer empty.
    pub(crate) fn drain(&mut self) -> Vec<AdminAlert> {
        core::mem::take(&mut self.alerts)
    }
}

impl AdminAlertSink for BufferSink {
    fn emit(&mut self, alert: AdminAlert) {
        self.alerts.push(alert);
    }
}

/// A fail-closed [`SecretSource`]: the wired sign-in route mints no credentials, so every minting
/// method is `unreachable!`. A route that actually mints (bind-device / refresh / recovery) **must**
/// inject the production `RngSecretSource` over a CSPRNG (ADR-0021) — it lands with the bind-device
/// slice — or this panics rather than ever issuing a fixed/fake credential.
pub(crate) struct PlaceholderSecrets;

impl SecretSource for PlaceholderSecrets {
    fn fresh_refresh(&mut self) -> RefreshToken {
        unreachable!("sign-in mints no secrets; RngSecretSource lands with the bind-device slice")
    }
    fn fresh_access(&mut self) -> AccessToken {
        unreachable!("sign-in mints no secrets; RngSecretSource lands with the bind-device slice")
    }
    fn fresh_recovery_code(&mut self) -> RecoveryCode {
        unreachable!("sign-in mints no secrets; RngSecretSource lands with the bind-device slice")
    }
    fn fresh_admin_invitation(&mut self) -> AdminInvitationToken {
        unreachable!("sign-in mints no secrets; RngSecretSource lands with the bind-device slice")
    }
    fn fresh_nonce(&mut self) -> Nonce {
        unreachable!("sign-in encrypts no fields; RngSecretSource lands with the issuance slice")
    }
    fn fresh_group_key(&mut self) -> GroupKey {
        // Group bootstrap is operator-run provisioning (plan §13.4), never a Worker request path —
        // and even the issuance slice injects `RngSecretSource` over a CSPRNG. So this never fires;
        // it panics rather than ever minting a fixed/fake key.
        unreachable!("Group bootstrap is operator-run provisioning, never a Worker path (spec 008)")
    }
}

/// Server time from the JS runtime clock (`Date.now()`) — **never a device clock** (a wrong device
/// clock can neither grant nor deny; binding cannot complete offline, plan §10).
pub(crate) struct WorkerClock;

impl Clock for WorkerClock {
    fn now(&self) -> UnixSeconds {
        UnixSeconds::new((worker::Date::now().as_millis() / 1000) as i64)
    }
}
