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
//! Row ids are minted by Postgres `gen_random_uuid()` — **built-in since PG13** (Neon is PG15/16;
//! the test target is `postgres:16`), so no `pgcrypto` extension is required on the PII path.
#![forbid(unsafe_code)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use boundless_auth::{RefreshPresentation, Session, SessionFamilyStatus, UnixSeconds};
use boundless_crypto::{
    refresh_token_hash, CodeHash, HmacKey, PhoneLookupHash, RefreshTokenHash, HASH_LEN,
};
use boundless_domain::{MemberId, RefreshToken, Role, SessionFamilyId};
use boundless_server_core::{
    AuthStore, FamilyInfo, MemberRecord, OnboardingCodeRow, RecoveryCodeRow, RefreshClassification,
    StoreBackend,
};
use tokio_postgres::{Client, Transaction};
use uuid::Uuid;

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
fn to_pg_time(t: UnixSeconds) -> SystemTime {
    let s = t.as_secs();
    if s >= 0 {
        UNIX_EPOCH + Duration::from_secs(s as u64)
    } else {
        UNIX_EPOCH - Duration::from_secs(s.unsigned_abs())
    }
}

/// Map a Postgres `member_role` (read as text) to the domain [`Role`]. Unknown variants are
/// dropped (forward-compatible: a role this build does not model is simply ignored).
fn role_from_wire(s: &str) -> Option<Role> {
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
        tx.execute(
            "SELECT set_config('app.current_group_id', $1, true)",
            &[&group],
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
            .query_opt(
                "SELECT id, roles::text[] AS roles FROM members WHERE phone_lookup_hash = $1",
                &[&needle],
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
            .query_opt(
                "SELECT code_hash, expires_at, max_attempts FROM onboarding_codes \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[&mid],
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
            .execute(
                "UPDATE onboarding_codes SET consumed_at = $2 \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[&mid, &now_ts],
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
            .query_opt(
                "SELECT family_id, member_id, rotated_at, revoked_at \
                 FROM sessions WHERE refresh_token_hash = $1",
                &[&needle],
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
        tx.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&fam_text],
        )
        .await?;
        let superseded = tx
            .query_opt(
                "UPDATE sessions SET rotated_at = $2 \
                 WHERE family_id = $1 AND rotated_at IS NULL AND revoked_at IS NULL \
                 RETURNING id, member_id",
                &[&fam, &now_ts],
            )
            .await?;
        let (parent_id, member_id) = match superseded {
            Some(r) => (r.get::<_, Uuid>("id"), r.get::<_, Uuid>("member_id")),
            None => return Err(StoreError::NoLiveCurrentToRotate), // tx drops → rollback
        };
        tx.execute(
            "INSERT INTO sessions (id, group_id, member_id, family_id, refresh_token_hash, parent_id) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)",
            &[&group, &member_id, &fam, &new_hash, &parent_id],
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
        tx.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&fam_text],
        )
        .await?;
        tx.execute(
            "UPDATE sessions SET revoked_at = $2 WHERE family_id = $1 AND revoked_at IS NULL",
            &[&fam, &now_ts],
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
            .query_one(
                "INSERT INTO sessions (id, group_id, member_id, family_id, refresh_token_hash) \
                 VALUES (gen_random_uuid(), $1, $2, gen_random_uuid(), $3) RETURNING family_id",
                &[&group, &mid, &new_hash],
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
            .query_opt(
                "SELECT code_hash FROM recovery_codes \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[&mid],
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
            .execute(
                "UPDATE recovery_codes SET consumed_at = $2, superseded_at = $2 \
                 WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
                &[&mid, &now_ts],
            )
            .await?;
        if consumed == 0 {
            // Lost the race — drop the txn (rollback) without inserting a fresh code.
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO recovery_codes (id, group_id, member_id, code_hash) \
             VALUES (gen_random_uuid(), $1, $2, $3)",
            &[&group, &mid, &fresh],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}

/// Convert a `timestamptz` (`SystemTime`) back to server-time `UnixSeconds`.
///
/// Whole-second contract: issuance writes whole-second TTLs and the core's TTL gate is whole-second,
/// so flooring sub-second precision here is lossless in practice and consistent with `UnixSeconds`
/// being an integer.
fn system_time_to_unix(t: SystemTime) -> UnixSeconds {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => UnixSeconds::new(d.as_secs() as i64),
        Err(e) => UnixSeconds::new(-(e.duration().as_secs() as i64)),
    }
}
