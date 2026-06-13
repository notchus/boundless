//! `boundless-server-store` — the Postgres-backed data-access layer for the member-auth store
//! (spec 001 **T07-shell slice A**).
//!
//! [`PgAuthStore`] implements the `core::server` [`AuthStore`] **port** (`core/server/src/ports.rs`)
//! against a real Postgres (the T06 schema). The port is **`async` and fallible**
//! (`-> Result<_, StoreError>`) — a database backend is inherently async and you cannot
//! block-on-async in the Cloudflare Workers wasm runtime — so the ports were made async (the
//! "async-port bridge", ADR-0020) and this adapter now `impl`s them directly. The single shared
//! error is declared once via [`StoreBackend`] (`type Error = StoreError`).
//!
//! **Device tokens are a separate port.** [`PgAuthStore`] does **not** implement
//! `core::server::DeviceStore` (the device-token methods): `register_device` must persist a
//! *reversibly-encrypted* token (push needs the plaintext back, so a one-way hash will not do),
//! and that at-rest encryption primitive is deferred to spec 008. The in-memory stub implements
//! both ports; this adapter ships the session/code/member half now (see `DEFERRED.md`).
//!
//! **Why this layer earns its keep:** the in-memory stub's "atomic" methods are *trivially* atomic
//! (single-threaded), so it can only *model* the security-critical contracts. This adapter proves
//! them against a real database — single-consume under concurrency, atomic supersede-then-insert,
//! rotate-vs-replay TOCTOU, and RLS tenant isolation — in `server/store/tests`.
//!
//! **Unnamed (pooler-safe) statements — ADR-0024.** Every parameterized query here uses the
//! `tokio-postgres` **`query_typed*` / `execute_typed`** family (each `$n`'s `Type` supplied inline),
//! NOT the default `query`/`query_one`/`query_opt`/`execute(&str, …)` path. The default path issues a
//! named `Parse` and caches the prepared statement on the *connection*; across Cloudflare Hyperdrive's
//! connection pooler a cached name does not survive to the next physical connection, so it breaks. The
//! typed family sends an **unnamed** statement with explicit parameter types each time — exactly the
//! shape Hyperdrive expects (tokio-postgres documents `query_typed*` as "suitable in environments where
//! prepared statements aren't supported (such as Cloudflare Workers with Hyperdrive)"). The same code
//! runs natively in tests (over a direct `TcpStream`) and on wasm32 in the Worker (over a Hyperdrive
//! `worker::Socket`); see the target-split `tokio-postgres` features in `Cargo.toml`.
//!
//! **Tenant scoping (RLS).** Every method runs inside a transaction that first sets the per-request
//! tenant GUC via `set_config('app.current_group_id', $1, true)` (the parameterized, **transaction-
//! local** form — `SET LOCAL` cannot take a bind parameter). The T06 resolver `current_group_id()`
//! maps an unset/empty GUC to NULL, so a forgotten tenant fails **closed** (zero rows / rejected
//! writes). The production Worker must connect as a **non-superuser, non-`BYPASSRLS`** role or RLS
//! is bypassed (T06 carry-forward; the tests connect as such a role).
//!
//! **No PII is logged** (P2): this layer handles only keyed hashes (`bytea`) and opaque ids; the
//! tainted plaintext types never reach it. [`StoreError`] carries no row values.
//!
//! Row ids are minted by Postgres `gen_random_uuid()` — **built-in since PG13** (CI + local Docker +
//! the Neon origin are all PG18, the parity target), so no `pgcrypto` extension is required on the
//! PII path.
#![forbid(unsafe_code)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use boundless_auth::{RefreshPresentation, Session, SessionFamilyStatus, UnixSeconds};
use boundless_crypto::{
    admin_invitation_token_hash, refresh_token_hash, AdminInvitationTokenHash, CodeHash, HmacKey,
    PhoneLookupHash, RefreshTokenHash, HASH_LEN,
};
use boundless_domain::{AdminInvitationToken, MemberId, RefreshToken, Role, SessionFamilyId};
use boundless_server_core::{
    AdminCredential, AdminInviteRecord, AdminProvisioningStore, AdminWebAuthnStore, AuthStore,
    FamilyInfo, MemberRecord, NewAdminCredential, OnboardingCodeRow, RecoveryCodeRow,
    RefreshClassification, RegisterCompleteOutcome, StoreBackend,
};
use tokio_postgres::{types::Type, Client, Transaction};
use uuid::Uuid;

mod members;
pub use members::PgMemberStore;

/// Errors from the Postgres auth store. Carries **no row values** (P2): a `Db` error wraps the
/// driver error (query text + Postgres message, never bound parameter values).
#[derive(Debug)]
pub enum StoreError {
    /// The underlying `tokio-postgres` driver returned an error.
    Db(tokio_postgres::Error),
    /// A rotate was attempted on a family with no live current credential (already rotated away
    /// or the family was revoked) — the caller must treat this as "do not issue a session".
    NoLiveCurrentToRotate,
    /// A stored `bytea`/`uuid` column had an unexpected shape (schema drift / corruption).
    MalformedColumn(&'static str),
    /// The runtime DB role has privilege that **bypasses RLS** (superuser, `BYPASSRLS`, or
    /// `REPLICATION`), so the per-Group tenant isolation would not actually apply.
    /// [`ensure_least_privilege`] returns this and the Worker must refuse to start. The payload
    /// names the condition (no PII).
    PrivilegeTooHigh(&'static str),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Db(e) => write!(f, "postgres error: {e}"),
            StoreError::NoLiveCurrentToRotate => {
                write!(
                    f,
                    "no live current credential to rotate (already rotated or revoked)"
                )
            }
            StoreError::MalformedColumn(c) => write!(f, "malformed column: {c}"),
            StoreError::PrivilegeTooHigh(what) => {
                write!(
                    f,
                    "runtime DB role bypasses RLS ({what}) — refusing to start"
                )
            }
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Db(e) => Some(e),
            _ => None,
        }
    }
}

impl From<tokio_postgres::Error> for StoreError {
    fn from(e: tokio_postgres::Error) -> Self {
        StoreError::Db(e)
    }
}

type Result<T> = std::result::Result<T, StoreError>;

/// Convert an injected server-time `UnixSeconds` to a `SystemTime` for a `timestamptz` bind.
///
/// Precondition: `now` is injected, positive server time (T04/T05). The negative branch exists only
/// for totality; it is unreachable for any real server clock and never approaches `SystemTime`'s
/// representable range.
pub(crate) fn to_pg_time(t: UnixSeconds) -> SystemTime {
    let s = t.as_secs();
    if s >= 0 {
        UNIX_EPOCH + Duration::from_secs(s as u64)
    } else {
        UNIX_EPOCH - Duration::from_secs(s.unsigned_abs())
    }
}

/// Map a Postgres `member_role` (read as text) to the domain [`Role`]. Unknown variants are
/// dropped (forward-compatible: a role this build does not model is simply ignored).
pub(crate) fn role_from_wire(s: &str) -> Option<Role> {
    match s {
        "rider" => Some(Role::Rider),
        "driver" => Some(Role::Driver),
        "admin" => Some(Role::Admin),
        _ => None,
    }
}

/// Read a `bytea` column into a fixed-size hash array.
fn hash_array(bytes: Vec<u8>, col: &'static str) -> Result<[u8; HASH_LEN]> {
    bytes
        .try_into()
        .map_err(|_| StoreError::MalformedColumn(col))
}

/// Refuse to proceed if the connected role can **bypass row-level security** — the single
/// highest-impact way the privacy model fails in production (T06 carry-forward; sec-audit W2).
///
/// Every `PgAuthStore` method scopes tenant access with RLS (`app.current_group_id`). But
/// `FORCE ROW LEVEL SECURITY` is itself bypassed by a **superuser**, any role with the
/// **`BYPASSRLS`** attribute, or — just as effectively, if less obviously — any role with
/// **`REPLICATION`** (which can open a replication connection and stream the WAL, i.e. every
/// tenant's rows, with RLS never consulted). Neon's default `neondb_owner` belongs to
/// `neon_superuser`, which carries `BYPASSRLS`, `REPLICATION`, `CREATEROLE`, and `CREATEDB` (per
/// Neon's role docs, <https://neon.com/docs/manage/roles>) — so the Worker must connect as a
/// dedicated locked-down app role, never the default owner. If it connected as such a role, RLS
/// would silently not apply and one tenant could read/write another's PII. This is
/// invisible to ordinary tests (which drop privilege via `SET ROLE`), so it must be asserted at
/// boot: the Worker (T07-shell-B) calls this immediately after connecting, before constructing any
/// [`PgAuthStore`], and **fails closed** on `Err`.
///
/// `is_superuser` reflects the *effective* role (so it is correct after `SET ROLE`); `rolbypassrls`
/// and `rolreplication` are read for the connected role (`current_user`, which `SET ROLE` updates).
/// The check is a single read-only query and runs unscoped (it is a role-attribute probe, not
/// tenant data), so it needs no transaction.
///
/// **Scope:** this catches the three *role-attribute* RLS bypasses (superuser, `BYPASSRLS`,
/// `REPLICATION`). It deliberately does **not** also reject `CREATEROLE`/`CREATEDB`: those are
/// *escalation* attributes (a role could grant itself more), not a way to read another tenant's
/// rows on the current connection — `scripts/provision-neon.sh` enforces their absence when minting
/// the role, but the boot guard's job is narrower (refuse a connection that bypasses RLS *now*). The
/// fourth Postgres bypass — a table's **owner** is exempt unless the table has `FORCE ROW LEVEL
/// SECURITY` — is covered separately by T06: every PII table is created `... ENABLE ROW LEVEL
/// SECURITY` **and** `... FORCE ROW LEVEL SECURITY`, asserted by `server/tests/migrations.rs`. So
/// this guard + FORCE-RLS-on-every-table together close all the bypasses; do not read "least
/// privilege" as also asserting non-ownership or non-escalation.
pub async fn ensure_least_privilege(client: &Client) -> Result<()> {
    let row = client
        .query_typed_one(
            "SELECT current_setting('is_superuser')::bool AS is_super, \
             COALESCE((SELECT rolbypassrls FROM pg_roles WHERE rolname = current_user), false) AS bypass_rls, \
             COALESCE((SELECT rolreplication FROM pg_roles WHERE rolname = current_user), false) AS replication",
            &[],
        )
        .await?;
    if row.get::<_, bool>("is_super") {
        return Err(StoreError::PrivilegeTooHigh("current_user is a superuser"));
    }
    if row.get::<_, bool>("bypass_rls") {
        return Err(StoreError::PrivilegeTooHigh("current_user has BYPASSRLS"));
    }
    if row.get::<_, bool>("replication") {
        return Err(StoreError::PrivilegeTooHigh("current_user has REPLICATION"));
    }
    Ok(())
}

/// A Postgres-backed implementation of the `core::server` `AuthStore` contract.
///
/// Holds an owned connection and the tenant `group_id` it is scoped to (one Boundless install =
/// one Group). Construct with [`PgAuthStore::new`]; the caller owns connecting + spawning the
/// `tokio-postgres` `Connection` (native `TcpStream` in tests; a Hyperdrive `Socket` in the
/// Worker, T07-shell-B).
pub struct PgAuthStore {
    client: Client,
    group_id: Uuid,
}

impl PgAuthStore {
    /// Wrap an established client, scoped to `group_id`.
    pub fn new(client: Client, group_id: Uuid) -> Self {
        Self { client, group_id }
    }

    /// Open a transaction and scope it to this store's tenant (RLS). **Every method begins here** —
    /// there is exactly one place that sets `app.current_group_id`, and it is always transaction-
    /// local (`set_config(..., true)`), so a pooled connection never leaks a prior tenant and a
    /// statement issued outside a `begin()` txn would run unscoped → fail-closed. No raw-`Client`
    /// accessor is exposed precisely so no method can bypass this.
    async fn begin(&mut self) -> Result<Transaction<'_>> {
        let group = self.group_id.to_string();
        let tx = self.client.transaction().await?;
        tx.execute_typed(
            "SELECT set_config('app.current_group_id', $1, true)",
            &[(&group, Type::TEXT)],
        )
        .await?;
        Ok(tx)
    }
}

/// The single shared store error (declared once for both ports; this adapter implements only
/// [`AuthStore`] — device tokens are deferred, see the module docs).
impl StoreBackend for PgAuthStore {
    type Error = StoreError;
}

/// The `core::server` [`AuthStore`] port, realized over Postgres. Every method opens a tenant-
/// scoped transaction via the inherent [`PgAuthStore::begin`] (per-request RLS, fail-closed).
impl AuthStore for PgAuthStore {
    /// Look up a member by phone-lookup hash (uniform indexed equality on the keyed HMAC — never
    /// the plaintext phone). `None` when no member matches in this tenant.
    async fn find_member_by_phone(
        &mut self,
        hash: &PhoneLookupHash,
    ) -> Result<Option<MemberRecord>> {
        let needle = hash.as_bytes().to_vec();
        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt(
                "SELECT id, roles::text[] AS roles FROM members WHERE phone_lookup_hash = $1",
                &[(&needle, Type::BYTEA)],
            )
            .await?;
        tx.commit().await?;
        Ok(row.map(|r| {
            let id: Uuid = r.get("id");
            let roles: Vec<String> = r.get("roles");
            MemberRecord {
                member_id: MemberId::from_uuid(id),
                roles: roles.iter().filter_map(|s| role_from_wire(s)).collect(),
            }
        }))
    }

    /// The member's live Onboarding Code, if any (`None` = none / consumed / superseded — the
    /// partial-unique index guarantees at most one live row).
    async fn load_live_onboarding(
        &mut self,
        member: MemberId,
    ) -> Result<Option<OnboardingCodeRow>> {
        let mid = member.as_uuid();
        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt(
                "SELECT code_hash, expires_at, max_attempts FROM onboarding_codes \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[(&mid, Type::UUID)],
            )
            .await?;
        tx.commit().await?;
        match row {
            None => Ok(None),
            Some(r) => {
                let code_hash: Vec<u8> = r.get("code_hash");
                let expires_at: SystemTime = r.get("expires_at");
                let max_attempts: i32 = r.get("max_attempts");
                Ok(Some(OnboardingCodeRow {
                    code_hash: CodeHash::from_bytes(hash_array(code_hash, "code_hash")?),
                    expires_at: system_time_to_unix(expires_at),
                    max_attempts: max_attempts.max(0) as u32,
                }))
            }
        }
    }

    /// Atomically consume the member's live Onboarding Code **iff still live**; `true` iff THIS
    /// call consumed it. The partial-unique index bounds the live set to one row, so the single
    /// `UPDATE … WHERE … live` either hits 1 row (consumed here) or 0 (lost the race — the caller
    /// treats `false` as "already consumed", never a second bind; carry-forward (a)).
    async fn consume_onboarding_if_live(
        &mut self,
        member: MemberId,
        now: UnixSeconds,
    ) -> Result<bool> {
        let mid = member.as_uuid();
        let now_ts = to_pg_time(now);
        let tx = self.begin().await?;
        let n = tx
            .execute_typed(
                "UPDATE onboarding_codes SET consumed_at = $2 \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[(&mid, Type::UUID), (&now_ts, Type::TIMESTAMPTZ)],
            )
            .await?;
        tx.commit().await?;
        Ok(n == 1)
    }

    /// Classify a presented refresh credential within its session lineage. Hashes the presented
    /// token (keyed HMAC) and looks it up by the unique hash index: a live row ⇒ `Current`, a
    /// rotated row ⇒ `Superseded`, no match ⇒ `Unknown`. The family status comes from the matched
    /// row's `revoked_at` (revoke stamps every row of the family), so a credential in a revoked
    /// family reports `Revoked` regardless of its lineage position.
    ///
    /// **Constant-time realization (R6):** the `AuthStore` trait names a constant-time
    /// `refresh_token_matches`. Here that contract is realized as an **indexed equality on the
    /// keyed HMAC** (`WHERE refresh_token_hash = $1`, `$1` = the keyed hash of the presented token),
    /// the same pattern as `phone_lookup_hash`. This is timing-safe because the value compared is a
    /// secret-keyed 256-bit HMAC, not the credential. Do **not** "optimize" this by loading the
    /// stored hashes into memory and `==`-ing them — that would be the non-constant-time membership
    /// oracle the hash types forbid by having no `PartialEq`.
    async fn classify_refresh(
        &mut self,
        presented: &RefreshToken,
        key: &HmacKey,
    ) -> Result<RefreshClassification> {
        let needle = refresh_token_hash(key, presented).as_bytes().to_vec();
        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt(
                "SELECT family_id, member_id, rotated_at, revoked_at \
                 FROM sessions WHERE refresh_token_hash = $1",
                &[(&needle, Type::BYTEA)],
            )
            .await?;
        tx.commit().await?;
        Ok(match row {
            None => RefreshClassification {
                presentation: RefreshPresentation::Unknown,
                family: None,
            },
            Some(r) => {
                let family_id: Uuid = r.get("family_id");
                let member_id: Uuid = r.get("member_id");
                let rotated_at: Option<SystemTime> = r.get("rotated_at");
                let revoked_at: Option<SystemTime> = r.get("revoked_at");
                let presentation = if rotated_at.is_none() {
                    RefreshPresentation::Current
                } else {
                    RefreshPresentation::Superseded
                };
                let status = if revoked_at.is_some() {
                    SessionFamilyStatus::Revoked
                } else {
                    SessionFamilyStatus::Active
                };
                RefreshClassification {
                    presentation,
                    family: Some(FamilyInfo {
                        id: SessionFamilyId::from_uuid(family_id),
                        status,
                        member: MemberId::from_uuid(member_id),
                    }),
                }
            }
        })
    }

    /// Atomically rotate: supersede the family's current credential and install `new_refresh_hash`
    /// as the new current, in one transaction. The supersede `UPDATE` must affect **exactly one**
    /// row; if zero (the family was already rotated by a racing request, or revoked), the txn rolls
    /// back and this returns [`StoreError::NoLiveCurrentToRotate`] — never a second valid rotation.
    /// The `sessions_one_current_per_family` partial-unique index is the backstop.
    async fn rotate_session(
        &mut self,
        family: SessionFamilyId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<Session> {
        let fam = family.as_uuid();
        let fam_text = fam.to_string();
        let group = self.group_id;
        let now_ts = to_pg_time(now);
        let new_hash = new_refresh_hash.as_bytes().to_vec();

        let tx = self.begin().await?;
        // Serialize rotate vs revoke for this family (carry-forward (b)). Without this, a rotate
        // that commits a NEW current row while a concurrent revoke is in flight leaves that row
        // outside the revoke's READ COMMITTED snapshot — a live credential surviving a family-kill.
        // The advisory xact lock (released at commit/rollback) forces one to fully precede the
        // other; both are family-scoped and acquire it before any row lock, so there is no deadlock.
        // `hashtextextended` derives a full 64-bit key; a hash collision would only over-serialize
        // two unrelated families (a transient throughput cost) — it can never under-lock.
        tx.execute_typed(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[(&fam_text, Type::TEXT)],
        )
        .await?;
        let superseded = tx
            .query_typed_opt(
                "UPDATE sessions SET rotated_at = $2 \
                 WHERE family_id = $1 AND rotated_at IS NULL AND revoked_at IS NULL \
                 RETURNING id, member_id",
                &[(&fam, Type::UUID), (&now_ts, Type::TIMESTAMPTZ)],
            )
            .await?;
        let (parent_id, member_id) = match superseded {
            Some(r) => (r.get::<_, Uuid>("id"), r.get::<_, Uuid>("member_id")),
            None => return Err(StoreError::NoLiveCurrentToRotate), // tx drops → rollback
        };
        tx.execute_typed(
            "INSERT INTO sessions (id, group_id, member_id, family_id, refresh_token_hash, parent_id) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)",
            &[
                (&group, Type::UUID),
                (&member_id, Type::UUID),
                (&fam, Type::UUID),
                (&new_hash, Type::BYTEA),
                (&parent_id, Type::UUID),
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(Session {
            member_id: MemberId::from_uuid(member_id),
            family_id: family,
            access_token_expires_at: access_expires_at,
            family_status: SessionFamilyStatus::Active,
        })
    }

    /// Atomically revoke the entire family (replay detected, or an admin-mediated event). Stamps
    /// `revoked_at` on every still-live row of the family; a revoked family never rotates again.
    async fn revoke_family(&mut self, family: SessionFamilyId, now: UnixSeconds) -> Result<()> {
        let fam = family.as_uuid();
        let fam_text = fam.to_string();
        let now_ts = to_pg_time(now);
        let tx = self.begin().await?;
        // Same family lock as rotate_session (carry-forward (b)): once we hold it, any in-flight
        // rotation has either fully committed (so its new current row is visible to the UPDATE
        // below and gets revoked too) or has not started (so it will later find no live current).
        tx.execute_typed(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[(&fam_text, Type::TEXT)],
        )
        .await?;
        tx.execute_typed(
            "UPDATE sessions SET revoked_at = $2 WHERE family_id = $1 AND revoked_at IS NULL",
            &[(&fam, Type::UUID), (&now_ts, Type::TIMESTAMPTZ)],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Create a brand-new `Active` session family (device bind / recovery re-bind) with
    /// `new_refresh_hash` as its current credential.
    async fn create_session_family(
        &mut self,
        member: MemberId,
        new_refresh_hash: RefreshTokenHash,
        access_expires_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<Session> {
        let mid = member.as_uuid();
        let group = self.group_id;
        let new_hash = new_refresh_hash.as_bytes().to_vec();
        let tx = self.begin().await?;
        let row = tx
            .query_typed_one(
                "INSERT INTO sessions (id, group_id, member_id, family_id, refresh_token_hash) \
                 VALUES (gen_random_uuid(), $1, $2, gen_random_uuid(), $3) RETURNING family_id",
                &[
                    (&group, Type::UUID),
                    (&mid, Type::UUID),
                    (&new_hash, Type::BYTEA),
                ],
            )
            .await?;
        let family_id: Uuid = row.get("family_id");
        tx.commit().await?;
        Ok(Session {
            member_id: member,
            family_id: SessionFamilyId::from_uuid(family_id),
            access_token_expires_at: access_expires_at,
            family_status: SessionFamilyStatus::Active,
        })
    }

    /// The member's live Recovery Code, if any (Drivers only in practice).
    async fn load_live_recovery(&mut self, member: MemberId) -> Result<Option<RecoveryCodeRow>> {
        let mid = member.as_uuid();
        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt(
                "SELECT code_hash FROM recovery_codes \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[(&mid, Type::UUID)],
            )
            .await?;
        tx.commit().await?;
        match row {
            None => Ok(None),
            Some(r) => {
                let code_hash: Vec<u8> = r.get("code_hash");
                Ok(Some(RecoveryCodeRow {
                    code_hash: CodeHash::from_bytes(hash_array(code_hash, "code_hash")?),
                }))
            }
        }
    }

    /// Atomically consume the live Recovery Code **and** install the fresh one (`fresh_hash`) as the
    /// member's new live code — rotated on use (ADR-0016 D3) — in one transaction; `true` iff THIS
    /// call did it. If no live code exists (lost the race) the txn rolls back and returns `false`.
    async fn consume_and_rotate_recovery(
        &mut self,
        member: MemberId,
        fresh_hash: CodeHash,
        now: UnixSeconds,
    ) -> Result<bool> {
        let mid = member.as_uuid();
        let group = self.group_id;
        let now_ts = to_pg_time(now);
        let fresh = fresh_hash.as_bytes().to_vec();
        let tx = self.begin().await?;
        let consumed = tx
            .execute_typed(
                "UPDATE recovery_codes SET consumed_at = $2, superseded_at = $2 \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[(&mid, Type::UUID), (&now_ts, Type::TIMESTAMPTZ)],
            )
            .await?;
        if consumed == 0 {
            // Lost the race — drop the txn (rollback) without inserting a fresh code.
            return Ok(false);
        }
        tx.execute_typed(
            "INSERT INTO recovery_codes (id, group_id, member_id, code_hash) \
             VALUES (gen_random_uuid(), $1, $2, $3)",
            &[
                (&group, Type::UUID),
                (&mid, Type::UUID),
                (&fresh, Type::BYTEA),
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}

/// The `core::server` [`AdminProvisioningStore`] port, realized over Postgres (T08 — developer
/// Admin creation + invitation mint). Invitations store only an at-rest hash + opaque ids (no PII,
/// no plaintext token), so this ships without the field-level encryption the device-token port
/// awaits. Both methods run in a single tenant-scoped transaction via [`PgAuthStore::begin`].
impl AdminProvisioningStore for PgAuthStore {
    /// Provision a pending Admin (role `admin`, **no phone** — Admins authenticate via WebAuthn)
    /// and mint its first registration invitation in one transaction, returning the DB-minted
    /// [`MemberId`] (`gen_random_uuid()` → no ambient randomness in the core). The token never
    /// reaches Postgres — only its `token_hash` and the server-time `expires_at`.
    async fn create_pending_admin_with_invitation(
        &mut self,
        token_hash: AdminInvitationTokenHash,
        expires_at: UnixSeconds,
    ) -> Result<MemberId> {
        let group = self.group_id;
        let exp = to_pg_time(expires_at);
        let hash = token_hash.as_bytes().to_vec();
        let tx = self.begin().await?;
        // The pending Admin member. RLS `WITH CHECK` requires group_id = current_group_id() (set by
        // `begin`), so this can only write into this tenant.
        let row = tx
            .query_typed_one(
                "INSERT INTO members (id, group_id, roles) \
                 VALUES (gen_random_uuid(), $1, ARRAY['admin']::member_role[]) RETURNING id",
                &[(&group, Type::UUID)],
            )
            .await?;
        let admin_id: Uuid = row.get("id");
        // The invitation. The composite FK (admin_id, group_id) → members(id, group_id) is satisfied
        // by the row just inserted in this same transaction.
        tx.execute_typed(
            "INSERT INTO admin_invitations (id, group_id, admin_id, token_hash, expires_at) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4)",
            &[
                (&group, Type::UUID),
                (&admin_id, Type::UUID),
                (&hash, Type::BYTEA),
                (&exp, Type::TIMESTAMPTZ),
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(MemberId::from_uuid(admin_id))
    }

    /// Re-invite an existing pending Admin: **supersede then insert** in one transaction. The prior
    /// live invitation is stamped `consumed_at = now` (freeing the `admin_invitations_one_live_per_
    /// admin` partial-unique index) **before** the fresh row is inserted, so the one-live invariant
    /// holds and the index never raises a unique violation (the DEFERRED "T08 admin invite re-issue"
    /// ordering). Returns `false` (rolled back, nothing minted) when no pending Admin matches.
    async fn reissue_admin_invitation(
        &mut self,
        admin_id: MemberId,
        token_hash: AdminInvitationTokenHash,
        expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<bool> {
        let aid = admin_id.as_uuid();
        let aid_text = aid.to_string();
        let group = self.group_id;
        let exp = to_pg_time(expires_at);
        let now_ts = to_pg_time(now);
        let hash = token_hash.as_bytes().to_vec();
        let tx = self.begin().await?;
        // Serialize concurrent re-issues of the SAME admin (same pattern as `rotate_session`): the
        // one-live partial-unique index already makes two live invitations impossible, but without
        // this an unlucky concurrent re-issue would *error* with a unique violation instead of
        // cleanly superseding. The admin-scoped advisory xact lock makes the second re-issue see the
        // first's committed live row, so it supersedes it and succeeds. Released at commit/rollback.
        tx.execute_typed(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[(&aid_text, Type::TEXT)],
        )
        .await?;
        // The admin must exist *in this tenant* and hold the Admin role (a non-admin id is a no-op,
        // never a stray invitation). RLS scopes the visibility to this group.
        let exists = tx
            .query_typed_opt(
                "SELECT 1 FROM members WHERE id = $1 AND roles @> ARRAY['admin']::member_role[]",
                &[(&aid, Type::UUID)],
            )
            .await?
            .is_some();
        if !exists {
            return Ok(false); // tx drops → rollback
        }
        // Supersede the prior live invitation(s) FIRST (frees the one-live partial index)...
        tx.execute_typed(
            "UPDATE admin_invitations SET consumed_at = $2 \
             WHERE admin_id = $1 AND consumed_at IS NULL",
            &[(&aid, Type::UUID), (&now_ts, Type::TIMESTAMPTZ)],
        )
        .await?;
        // ...THEN insert the fresh one.
        tx.execute_typed(
            "INSERT INTO admin_invitations (id, group_id, admin_id, token_hash, expires_at) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4)",
            &[
                (&group, Type::UUID),
                (&aid, Type::UUID),
                (&hash, Type::BYTEA),
                (&exp, Type::TIMESTAMPTZ),
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}

/// Build an [`AdminCredential`] from a 7-column `admin_webauthn_credentials` projection (the shape
/// both reads share). No fallible decode — any `bytea`/`text[]` shape is a valid credential field
/// (unlike the fixed-length keyed hashes), so this is infallible.
fn credential_from_row(r: tokio_postgres::Row) -> AdminCredential {
    let admin_id: Uuid = r.get("admin_id");
    let revoked_at: Option<SystemTime> = r.get("revoked_at");
    AdminCredential {
        credential_id: r.get("credential_id"),
        admin_id: MemberId::from_uuid(admin_id),
        public_key: r.get("public_key"),
        sign_count: r.get("sign_count"),
        transports: r.get("transports"),
        aaguid: r.get("aaguid"),
        revoked_at: revoked_at.map(system_time_to_unix),
    }
}

/// Insert one admin credential within an open transaction (shared by `insert_credential` and the
/// `register_complete` txn). `created_at`/`updated_at` default to the DB clock (record-keeping; the
/// injected server-time `now` is reserved for the logic fields — consumed/revoked). The
/// `credential_id` unique index rejects a duplicate (surfaces as a `Db` error, never a silent replace).
async fn insert_credential_in_tx(
    tx: &Transaction<'_>,
    group: Uuid,
    admin_id: Uuid,
    cred: &NewAdminCredential,
) -> Result<()> {
    tx.execute_typed(
        "INSERT INTO admin_webauthn_credentials \
           (id, group_id, admin_id, credential_id, public_key, sign_count, transports, aaguid) \
         VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7)",
        &[
            (&group, Type::UUID),
            (&admin_id, Type::UUID),
            (&cred.credential_id, Type::BYTEA),
            (&cred.public_key, Type::BYTEA),
            (&cred.sign_count, Type::INT8),
            (&cred.transports, Type::TEXT_ARRAY),
            (&cred.aaguid, Type::BYTEA),
        ],
    )
    .await?;
    Ok(())
}

/// Revoke all of an admin's active credentials within an open transaction (single-sourced so the
/// standalone `revoke_all_for_admin` and the `register_complete` revoke-priors leg can never drift).
async fn revoke_all_for_admin_in_tx(
    tx: &Transaction<'_>,
    admin_id: Uuid,
    now: SystemTime,
) -> Result<()> {
    tx.execute_typed(
        "UPDATE admin_webauthn_credentials SET revoked_at = $2 \
         WHERE admin_id = $1 AND revoked_at IS NULL",
        &[(&admin_id, Type::UUID), (&now, Type::TIMESTAMPTZ)],
    )
    .await?;
    Ok(())
}

/// The `core::server` [`AdminWebAuthnStore`] port, realized over Postgres (spec 009 **T02**,
/// ADR-0027 — the Option B1 admin-passkey persistence the SvelteKit edge drives over the ADR-0026
/// BFF). Like [`AdminProvisioningStore`], the rows are PII-free (opaque WebAuthn bytes + counters +
/// server-time instants), so this ships without field-level encryption. Every method runs in a
/// single tenant-scoped transaction via [`PgAuthStore::begin`] (per-request RLS, fail-closed — D3),
/// over the unnamed `query_typed*`/`execute_typed` family (ADR-0024). The byte columns
/// (`credential_id`/`public_key`/`aaguid`) are `bytea`; `transports` is `text[]`.
impl AdminWebAuthnStore for PgAuthStore {
    /// Resolve a presented token to its invitation row. The keyed at-rest hash is computed **in the
    /// core** (`admin_invitation_token_hash` — the ADR-0017 P4 carve-out, not edge-TS; AC4b) and
    /// looked up by the unique `token_hash` index. The indexed equality is timing-safe because the
    /// compared value is a secret-keyed 256-bit HMAC, not the token — the same pattern as
    /// [`find_member_by_phone`](AuthStore::find_member_by_phone) /
    /// [`classify_refresh`](AuthStore::classify_refresh). A cross-tenant / unknown token resolves to
    /// `None` (RLS scopes the row to this tenant) — no existence oracle.
    async fn resolve_invitation_by_token(
        &mut self,
        key: &HmacKey,
        token: &AdminInvitationToken,
    ) -> Result<Option<AdminInviteRecord>> {
        let needle = admin_invitation_token_hash(key, token).as_bytes().to_vec();
        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt(
                "SELECT admin_id, group_id, expires_at, consumed_at FROM admin_invitations \
                 WHERE token_hash = $1",
                &[(&needle, Type::BYTEA)],
            )
            .await?;
        tx.commit().await?;
        Ok(row.map(|r| {
            let admin_id: Uuid = r.get("admin_id");
            let group_id: Uuid = r.get("group_id");
            let expires_at: SystemTime = r.get("expires_at");
            let consumed_at: Option<SystemTime> = r.get("consumed_at");
            AdminInviteRecord {
                admin_id: MemberId::from_uuid(admin_id),
                group_id,
                expires_at: system_time_to_unix(expires_at),
                consumed_at: consumed_at.map(system_time_to_unix),
            }
        }))
    }

    /// Atomically consume an invitation iff still live. One conditional `UPDATE … WHERE consumed_at
    /// IS NULL` either hits 1 row (consumed here) or 0 (already consumed / unknown / cross-tenant) —
    /// single-use under concurrency (R15; the same shape as `consume_onboarding_if_live`).
    async fn consume_invitation(
        &mut self,
        key: &HmacKey,
        token: &AdminInvitationToken,
        now: UnixSeconds,
    ) -> Result<bool> {
        let needle = admin_invitation_token_hash(key, token).as_bytes().to_vec();
        let now_ts = to_pg_time(now);
        let tx = self.begin().await?;
        let n = tx
            .execute_typed(
                "UPDATE admin_invitations SET consumed_at = $2 \
                 WHERE token_hash = $1 AND consumed_at IS NULL",
                &[(&needle, Type::BYTEA), (&now_ts, Type::TIMESTAMPTZ)],
            )
            .await?;
        tx.commit().await?;
        Ok(n == 1)
    }

    /// The admin's active (non-revoked) credentials, oldest first (AC20 — passkey + hardware backup).
    async fn list_active_credentials(&mut self, admin: MemberId) -> Result<Vec<AdminCredential>> {
        let aid = admin.as_uuid();
        let tx = self.begin().await?;
        let rows = tx
            .query_typed(
                "SELECT credential_id, admin_id, public_key, sign_count, transports, aaguid, \
                 revoked_at FROM admin_webauthn_credentials \
                 WHERE admin_id = $1 AND revoked_at IS NULL ORDER BY created_at",
                &[(&aid, Type::UUID)],
            )
            .await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(credential_from_row).collect())
    }

    /// The single active credential with this `credential_id` (the usernameless sign-in lookup), or
    /// `None` (revoked / unknown / cross-tenant — RLS scopes the read despite the global unique index).
    async fn find_active_credential(
        &mut self,
        credential_id: &[u8],
    ) -> Result<Option<AdminCredential>> {
        let cid = credential_id.to_vec();
        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt(
                "SELECT credential_id, admin_id, public_key, sign_count, transports, aaguid, \
                 revoked_at FROM admin_webauthn_credentials \
                 WHERE credential_id = $1 AND revoked_at IS NULL",
                &[(&cid, Type::BYTEA)],
            )
            .await?;
        tx.commit().await?;
        Ok(row.map(credential_from_row))
    }

    /// Insert a newly-registered credential (never replaces — multiple active per admin, AC20).
    async fn insert_credential(
        &mut self,
        admin: MemberId,
        credential: NewAdminCredential,
    ) -> Result<()> {
        let group = self.group_id;
        let tx = self.begin().await?;
        insert_credential_in_tx(&tx, group, admin.as_uuid(), &credential).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Revoke all of an admin's active credentials (ADR-0016 D4 lost-key recovery).
    async fn revoke_all_for_admin(&mut self, admin: MemberId, now: UnixSeconds) -> Result<()> {
        let now_ts = to_pg_time(now);
        let tx = self.begin().await?;
        revoke_all_for_admin_in_tx(&tx, admin.as_uuid(), now_ts).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Bump a credential's signature counter only-if-strictly-greater (clone-detection backstop, R10).
    /// `sign_count < $2` makes a stale/equal counter a no-op (0 rows; not an error). `revoked_at IS
    /// NULL` is defence-in-depth: a revoked credential is never asserted against (the sign-in path
    /// reads via `find_active_credential`, which filters revoked rows), so its counter must never
    /// advance. No `now` parameter: the `updated_at` column is trigger-stamped on UPDATE, so unlike
    /// the TS `CredentialStore.bumpSignCount(_, _, now)` port this has no server-time analogue.
    async fn bump_sign_count(&mut self, credential_id: &[u8], new_count: i64) -> Result<()> {
        let cid = credential_id.to_vec();
        let tx = self.begin().await?;
        tx.execute_typed(
            "UPDATE admin_webauthn_credentials SET sign_count = $2 \
             WHERE credential_id = $1 AND sign_count < $2 AND revoked_at IS NULL",
            &[(&cid, Type::BYTEA), (&new_count, Type::INT8)],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Complete a registration in one transaction (R11): atomically consume the invitation (`consumed_at
    /// IS NULL`, `RETURNING admin_id`), revoke the admin's prior credentials (D4), and insert the new
    /// one. The admin id is **derived from the consumed row** — never a web-supplied value, so the
    /// credential binds to exactly the admin the live invitation names. A token matching no live
    /// invitation in this tenant returns [`RegisterCompleteOutcome::InviteNotConsumable`] with the txn
    /// rolled back (nothing written). At most one live invitation per admin exists (the
    /// `admin_invitations_one_live_per_admin` index), so concurrent completions for one admin cannot
    /// both consume; the conditional `UPDATE` is the single-use guard (no advisory lock needed).
    async fn register_complete(
        &mut self,
        key: &HmacKey,
        token: &AdminInvitationToken,
        credential: NewAdminCredential,
        now: UnixSeconds,
    ) -> Result<RegisterCompleteOutcome> {
        let needle = admin_invitation_token_hash(key, token).as_bytes().to_vec();
        let now_ts = to_pg_time(now);
        let group = self.group_id;
        let tx = self.begin().await?;
        let consumed = tx
            .query_typed_opt(
                "UPDATE admin_invitations SET consumed_at = $2 \
                 WHERE token_hash = $1 AND consumed_at IS NULL RETURNING admin_id",
                &[(&needle, Type::BYTEA), (&now_ts, Type::TIMESTAMPTZ)],
            )
            .await?;
        let admin_id: Uuid = match consumed {
            Some(r) => r.get("admin_id"),
            None => return Ok(RegisterCompleteOutcome::InviteNotConsumable), // tx drops → rollback
        };
        // Revoke the admin's prior active credentials (D4), then insert the new one — same txn (R11).
        revoke_all_for_admin_in_tx(&tx, admin_id, now_ts).await?;
        insert_credential_in_tx(&tx, group, admin_id, &credential).await?;
        tx.commit().await?;
        Ok(RegisterCompleteOutcome::Completed {
            admin_id: MemberId::from_uuid(admin_id),
        })
    }
}

/// Convert a `timestamptz` (`SystemTime`) back to server-time `UnixSeconds`.
///
/// Whole-second contract: issuance writes whole-second TTLs and the core's TTL gate is whole-second,
/// so flooring sub-second precision here is lossless in practice and consistent with `UnixSeconds`
/// being an integer.
pub(crate) fn system_time_to_unix(t: SystemTime) -> UnixSeconds {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => UnixSeconds::new(d.as_secs() as i64),
        Err(e) => UnixSeconds::new(-(e.duration().as_secs() as i64)),
    }
}
