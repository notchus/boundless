//! `PgMemberStore` — the Postgres-backed implementation of the spec-008 member-management ports
//! (`core::server` `MemberStore` + `AuditStore` + `DelegatedKeyStore`), realized against the real
//! schema (migrations 0009 `delegated_keys`, 0010 `members.{name,address}_encrypted`, 0011
//! `audit_log`). The store-level twin of [`PgAuthStore`](crate::PgAuthStore): same discipline, a
//! different surface.
//!
//! **One struct, three ports.** [`MemberService`](boundless_server_core::MemberService) is generic
//! over a single `St: MemberStore + AuditStore + DelegatedKeyStore`, so all three live on one type
//! (exactly as the in-memory `MemMemberStore` test double does). The Worker (T09) composes
//! [`MemberService`] over this adapter.
//!
//! **The contracts this layer proves against a real database** (the in-memory stub can only *model*
//! them — it is single-threaded, so its "one transaction" methods are trivially atomic):
//! - **Atomic member + code mint** ([`MemberStore::insert_member`]) — the member row and its
//!   Onboarding Code are inserted in one transaction (R13). A phone conflict instead fetches the
//!   existing member's summary **and** writes the surface-and-link disclosure audit in that same
//!   transaction (I5 — the disclosure can never occur unaudited).
//! - **Atomic audited read** ([`MemberStore::read_member_detail_audited`]) — the ciphertext SELECT
//!   and the `audit_log` INSERT commit together (§7/R5); a not-found read writes no audit row.
//! - **Atomic supersede-then-insert** ([`MemberStore::regenerate_code`]) — against the
//!   `onboarding_codes_one_live_per_member` partial-unique index, serialized per member by a
//!   `pg_advisory_xact_lock` (the proven `rotate_session` / `reissue_admin_invitation` advisory-lock
//!   shape) so a concurrent regenerate cleanly supersedes rather than hitting a unique violation.
//! - **Optimistic concurrency** ([`MemberStore::edit_member`]) — `UPDATE … WHERE <token> = $expected`
//!   (0 rows ⇒ [`EditApplied::Stale`]) with no partial write (AC11).
//! - **RLS tenant isolation** — every method opens [`PgMemberStore::begin`] (the same per-request
//!   `set_config('app.current_group_id', …, true)` scoping as [`PgAuthStore`](crate::PgAuthStore));
//!   there is **no** raw-`Client` accessor, so no method can bypass it (R7).
//!
//! **Cross-cutting discipline (shared with [`PgAuthStore`](crate::PgAuthStore)):** unnamed
//! pooler-safe `query_typed*` / `execute_typed` statements (ADR-0024); ids minted by Postgres
//! `gen_random_uuid()` inline (no ambient randomness in the core, ADR-0021); **no PII is logged**
//! (P2) — this layer handles only `bytea` ciphertext / keyed hashes and opaque ids, never the
//! tainted plaintext types, and [`StoreError`] carries no row values.
//!
//! ## The `updated_at` optimistic-concurrency token (whole-second)
//!
//! The edit token is `members.updated_at`, compared at **whole-second** granularity
//! (`floor(extract(epoch from updated_at))::bigint = $expected`), and read back the same way. This
//! is faithful to the in-memory port contract, whose token is a whole-second
//! [`UnixSeconds`](boundless_auth::UnixSeconds): both have one-second granularity, so two genuine
//! edits within the same wall-clock second are indistinguishable — an acceptable bound for two human
//! admins (and identical to the model). `members.updated_at` is owned by the existing
//! `members_set_updated_at` **trigger** (server time, the DB clock) on every UPDATE, so `edit_member`
//! does not — and cannot — set it from the injected `now`; `insert_member` sets it from the injected
//! `now` (no trigger fires on INSERT). Both sources are server-side, so the "admin's clock is wrong"
//! edge is satisfied either way.
//!
//! ## Onboarding status is *derived*, not stored
//!
//! There is no `onboarding_status` column; it is computed at read time from the member's
//! `device_tokens` + `onboarding_codes` rows ([`STATUS_CASE`]). Like the store's `*_if_live` methods,
//! the derivation keys off `consumed_at`/`superseded_at` **structurally** and does **not** apply the
//! code TTL (TTL is `core::auth`'s job against server time — the `onboarding_consume_ignores_ttl`
//! discipline). The precise TTL-expired → `CodeExpiredOrLost` nuance is a later refinement (see
//! `DEFERRED.md` → spec 008 T07).

use std::time::SystemTime;

use boundless_auth::UnixSeconds;
use boundless_crypto::CodeHash;
use boundless_domain::{MemberId, Role};
use boundless_server_core::{
    AuditEntry, AuditField, AuditStore, DelegatedKeyStore, DuplicateDisclosureAudit, EditApplied,
    InsertMemberOutcome, MemberEditWrite, MemberStore, NewMemberWrite, OnboardingStatus,
    StoreBackend, StoredMemberPii, StoredMemberSummary,
};
use tokio_postgres::{types::Type, Client, Transaction};
use uuid::Uuid;

use crate::{role_from_wire, system_time_to_unix, to_pg_time, StoreError};

type Result<T> = std::result::Result<T, StoreError>;

/// The SQL `CASE` that derives a member's [`OnboardingStatus`] (no stored column — see the module
/// docs). It references the member as alias `m`, so it is spliced into queries that `FROM members m`.
/// Active device → `onboarded`; else a live (non-consumed, non-superseded) Onboarding Code →
/// `issued_not_onboarded`; else any device ever bound (now all invalidated) → `needs_reonboarding`;
/// else `code_expired_or_lost`. The lowercase tokens are the [`OnboardingStatus`] `snake_case` wire
/// spelling, mapped back by [`status_from_wire`]. The subqueries run inside the same RLS-scoped
/// transaction, so they only see this tenant's rows.
const STATUS_CASE: &str = "CASE \
     WHEN EXISTS (SELECT 1 FROM device_tokens d WHERE d.member_id = m.id AND d.invalidated_at IS NULL) \
       THEN 'onboarded' \
     WHEN EXISTS (SELECT 1 FROM onboarding_codes o \
                  WHERE o.member_id = m.id AND o.consumed_at IS NULL AND o.superseded_at IS NULL) \
       THEN 'issued_not_onboarded' \
     WHEN EXISTS (SELECT 1 FROM device_tokens d WHERE d.member_id = m.id) \
       THEN 'needs_reonboarding' \
     ELSE 'code_expired_or_lost' END";

/// Map a derived [`STATUS_CASE`] token back to [`OnboardingStatus`]. The `CASE` is exhaustive over the
/// four known tokens, so an unknown value is schema drift → fail closed ([`StoreError::MalformedColumn`]).
fn status_from_wire(s: &str) -> Result<OnboardingStatus> {
    match s {
        "issued_not_onboarded" => Ok(OnboardingStatus::IssuedNotOnboarded),
        "onboarded" => Ok(OnboardingStatus::Onboarded),
        "code_expired_or_lost" => Ok(OnboardingStatus::CodeExpiredOrLost),
        "needs_reonboarding" => Ok(OnboardingStatus::NeedsReonboarding),
        _ => Err(StoreError::MalformedColumn("onboarding_status")),
    }
}

/// The `members.roles` (`member_role[]`) wire spelling for a domain [`Role`] (the inverse of
/// [`role_from_wire`](crate::role_from_wire)). Issuance only ever writes `Rider`/`Driver` (Admin is
/// unrepresentable at the `IssuableRole` boundary, I11), but the mapping is total.
fn role_to_wire(role: Role) -> &'static str {
    match role {
        Role::Rider => "rider",
        Role::Driver => "driver",
        Role::Admin => "admin",
    }
}

/// Map an `audit_log.fields` token back to an [`AuditField`]. Unknown tokens are dropped
/// (forward-compatible — a field name a future build records but this one does not model), mirroring
/// [`role_from_wire`](crate::role_from_wire).
fn audit_field_from_wire(s: &str) -> Option<AuditField> {
    match s {
        "name" => Some(AuditField::Name),
        "phone" => Some(AuditField::Phone),
        "address" => Some(AuditField::Address),
        _ => None,
    }
}

/// The `audit_log.fields text[]` write tokens for a set of [`AuditField`]s (names, never values, AC9).
fn audit_field_tokens(fields: &[AuditField]) -> Vec<String> {
    fields.iter().map(|f| f.as_str().to_string()).collect()
}

/// A Postgres-backed implementation of the `core::server` member-management ports
/// ([`MemberStore`] + [`AuditStore`] + [`DelegatedKeyStore`]).
///
/// Holds an owned connection and the tenant `group_id` it is scoped to (one Boundless install = one
/// Group). Construct with [`PgMemberStore::new`]; the caller owns connecting + spawning the
/// `tokio-postgres` `Connection` (native `TcpStream` in tests; a Hyperdrive `Socket` in the Worker).
pub struct PgMemberStore {
    client: Client,
    group_id: Uuid,
}

impl PgMemberStore {
    /// Wrap an established client, scoped to `group_id`.
    pub fn new(client: Client, group_id: Uuid) -> Self {
        Self { client, group_id }
    }

    /// Open a transaction and scope it to this store's tenant (RLS). **Every method begins here** —
    /// the single place `app.current_group_id` is set, always transaction-local
    /// (`set_config(..., true)`), so a pooled connection never leaks a prior tenant and a statement
    /// outside a `begin()` txn runs unscoped → fail-closed. No raw-`Client` accessor is exposed so no
    /// method can bypass this (R7). Mirrors [`PgAuthStore::begin`](crate::PgAuthStore).
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

/// The single shared store error (declared once for all three member ports).
impl StoreBackend for PgMemberStore {
    type Error = StoreError;
}

impl MemberStore for PgMemberStore {
    /// Create a member + mint its Onboarding Code in **one transaction** (R13). On a phone conflict
    /// (`(group_id, phone_lookup_hash)` unique), instead fetch the existing member's summary **and**
    /// write the `disclosure` audit row in the **same transaction** (I5 surface-and-link). The
    /// conflict is detected with `ON CONFLICT … DO NOTHING RETURNING id` (no pre-check TOCTOU): a
    /// returned id is the new member; an empty result is the conflict branch.
    async fn insert_member(
        &mut self,
        write: NewMemberWrite,
        disclosure: DuplicateDisclosureAudit,
        now: UnixSeconds,
    ) -> Result<InsertMemberOutcome> {
        let group = self.group_id;
        let roles: Vec<String> = write
            .roles
            .iter()
            .map(|r| role_to_wire(*r).to_string())
            .collect();
        let phone_lookup = write.phone_lookup.as_bytes().to_vec();
        let now_ts = to_pg_time(now);
        let code_exp = to_pg_time(write.code_expires_at);
        let code_hash = write.onboarding_code_hash.as_bytes().to_vec();
        let created_by = write.created_by.as_uuid();

        let tx = self.begin().await?;
        // updated_at = the injected server `now` (no trigger fires on INSERT) so the optimistic token
        // starts at a known server-time value; subsequent edits are bumped by the members_set_updated_at
        // trigger (server time, DB clock).
        let inserted = tx
            .query_typed_opt(
                "INSERT INTO members \
                   (id, group_id, roles, phone_lookup_hash, phone_encrypted, name_encrypted, \
                    address_encrypted, created_by, updated_at) \
                 VALUES (gen_random_uuid(), $1, $2::text[]::member_role[], $3, $4, $5, $6, $7, $8) \
                 ON CONFLICT (group_id, phone_lookup_hash) DO NOTHING \
                 RETURNING id",
                &[
                    (&group, Type::UUID),
                    (&roles, Type::TEXT_ARRAY),
                    (&phone_lookup, Type::BYTEA),
                    (&write.phone_encrypted, Type::BYTEA),
                    (&write.name_encrypted, Type::BYTEA),
                    (&write.address_encrypted, Type::BYTEA),
                    (&created_by, Type::UUID),
                    (&now_ts, Type::TIMESTAMPTZ),
                ],
            )
            .await?;

        if let Some(r) = inserted {
            let mid: Uuid = r.get("id");
            // Mint the Onboarding Code in the same transaction (R13 atomicity).
            tx.execute_typed(
                "INSERT INTO onboarding_codes (id, group_id, member_id, code_hash, expires_at, created_by) \
                 VALUES (gen_random_uuid(), $1, $2, $3, $4, $5)",
                &[
                    (&group, Type::UUID),
                    (&mid, Type::UUID),
                    (&code_hash, Type::BYTEA),
                    (&code_exp, Type::TIMESTAMPTZ),
                    (&created_by, Type::UUID),
                ],
            )
            .await?;
            tx.commit().await?;
            return Ok(InsertMemberOutcome::Created(MemberId::from_uuid(mid)));
        }

        // Phone conflict: fetch the existing member's summary AND write the disclosure audit, same txn.
        let existing = tx
            .query_typed_one(
                &format!(
                    "SELECT m.id, m.name_encrypted, m.roles::text[] AS roles, {STATUS_CASE} AS status \
                     FROM members m WHERE m.phone_lookup_hash = $1"
                ),
                &[(&phone_lookup, Type::BYTEA)],
            )
            .await?;
        let existing_id: Uuid = existing.get("id");
        let name_encrypted: Option<Vec<u8>> = existing.get("name_encrypted");
        let Some(name_encrypted) = name_encrypted else {
            return Err(StoreError::MalformedColumn(
                "existing member name_encrypted",
            ));
        };
        let roles_existing: Vec<String> = existing.get("roles");
        let status_wire: String = existing.get("status");
        let summary = StoredMemberSummary {
            member_id: MemberId::from_uuid(existing_id),
            name_encrypted,
            roles: roles_existing
                .iter()
                .filter_map(|s| role_from_wire(s))
                .collect(),
            onboarding_status: status_from_wire(&status_wire)?,
        };
        let fields = audit_field_tokens(&disclosure.fields);
        tx.execute_typed(
            "INSERT INTO audit_log (id, group_id, admin_id, member_id, fields, request_id, created_at) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4::text[], $5, $6)",
            &[
                (&group, Type::UUID),
                (&created_by, Type::UUID),
                (&existing_id, Type::UUID),
                (&fields, Type::TEXT_ARRAY),
                (&disclosure.request_id, Type::TEXT),
                (&now_ts, Type::TIMESTAMPTZ),
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(InsertMemberOutcome::DuplicatePhone(summary))
    }

    /// The group's Rider/Driver member list (**excludes Admins**, I11). Returns the encrypted name +
    /// PII-free fields; the orchestration decrypts the name. **Not** an audited read.
    async fn list_members(&mut self) -> Result<Vec<StoredMemberSummary>> {
        let tx = self.begin().await?;
        let rows = tx
            .query_typed(
                &format!(
                    "SELECT m.id, m.name_encrypted, m.roles::text[] AS roles, {STATUS_CASE} AS status \
                     FROM members m \
                     WHERE NOT (m.roles @> ARRAY['admin']::member_role[]) \
                     ORDER BY m.created_at ASC, m.id ASC"
                ),
                &[],
            )
            .await?;
        tx.commit().await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let name_encrypted: Option<Vec<u8>> = r.get("name_encrypted");
            let Some(name_encrypted) = name_encrypted else {
                return Err(StoreError::MalformedColumn("member name_encrypted"));
            };
            let roles: Vec<String> = r.get("roles");
            let status_wire: String = r.get("status");
            out.push(StoredMemberSummary {
                member_id: MemberId::from_uuid(r.get::<_, Uuid>("id")),
                name_encrypted,
                roles: roles.iter().filter_map(|s| role_from_wire(s)).collect(),
                onboarding_status: status_from_wire(&status_wire)?,
            });
        }
        Ok(out)
    }

    /// Read a member's full PII ciphertext **and** write the `audit` row in **one transaction** (I5/§7).
    /// Order: SELECT ciphertext → INSERT audit → COMMIT. A not-found member writes **no** audit row
    /// (returns `None`); a member that exists but lacks PII (an Admin / unbackfilled row) is a
    /// malformed read — surfaced as an error **without** an audit (no PII was disclosable).
    async fn read_member_detail_audited(
        &mut self,
        member_id: MemberId,
        audit: AuditEntry,
    ) -> Result<Option<StoredMemberPii>> {
        let mid = member_id.as_uuid();
        let group = self.group_id;
        let admin = audit.admin_id.as_uuid();
        let fields = audit_field_tokens(&audit.fields);
        let created = to_pg_time(audit.timestamp);

        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt(
                &format!(
                    "SELECT m.name_encrypted, m.phone_encrypted, m.address_encrypted, \
                            m.roles::text[] AS roles, {STATUS_CASE} AS status, \
                            floor(extract(epoch from m.updated_at))::bigint AS updated_secs \
                     FROM members m WHERE m.id = $1"
                ),
                &[(&mid, Type::UUID)],
            )
            .await?;
        let Some(r) = row else {
            // No such member → no PII read, no audit row written.
            tx.commit().await?;
            return Ok(None);
        };
        let name_encrypted: Option<Vec<u8>> = r.get("name_encrypted");
        let phone_encrypted: Option<Vec<u8>> = r.get("phone_encrypted");
        let address_encrypted: Option<Vec<u8>> = r.get("address_encrypted");
        let (Some(name_encrypted), Some(phone_encrypted), Some(address_encrypted)) =
            (name_encrypted, phone_encrypted, address_encrypted)
        else {
            // Member exists but has no PII (Admin / unbackfilled). Surface a malformed read; tx drops
            // → rollback, so no spurious audit row is written for a non-disclosable read.
            return Err(StoreError::MalformedColumn("member PII"));
        };
        let roles: Vec<String> = r.get("roles");
        let status_wire: String = r.get("status");
        let updated_secs: i64 = r.get("updated_secs");

        // Write the audit row in the SAME transaction as the SELECT (I5/§7 — a failed audit INSERT
        // rolls back the read, so PII is never served without its audit row).
        tx.execute_typed(
            "INSERT INTO audit_log (id, group_id, admin_id, member_id, fields, request_id, created_at) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4::text[], $5, $6)",
            &[
                (&group, Type::UUID),
                (&admin, Type::UUID),
                (&mid, Type::UUID),
                (&fields, Type::TEXT_ARRAY),
                (&audit.request_id, Type::TEXT),
                (&created, Type::TIMESTAMPTZ),
            ],
        )
        .await?;
        tx.commit().await?;

        Ok(Some(StoredMemberPii {
            member_id,
            name_encrypted,
            phone_encrypted,
            address_encrypted,
            roles: roles.iter().filter_map(|s| role_from_wire(s)).collect(),
            onboarding_status: status_from_wire(&status_wire)?,
            updated_at: UnixSeconds::new(updated_secs),
        }))
    }

    /// Apply an edit under optimistic concurrency (AC11): update only the `Some` fields, **iff** the
    /// row's whole-second `updated_at` token still equals `expected_updated_at`; otherwise 0 rows ⇒
    /// [`EditApplied::Stale`] with no write. The `members_set_updated_at` trigger bumps `updated_at`
    /// (server time) on a successful update — so the injected `now` is unused here (the trigger owns
    /// the column; see the module-level note on the whole-second token).
    async fn edit_member(
        &mut self,
        member_id: MemberId,
        write: MemberEditWrite,
        expected_updated_at: UnixSeconds,
        _now: UnixSeconds,
    ) -> Result<EditApplied> {
        let mid = member_id.as_uuid();
        let expected = expected_updated_at.as_secs();
        let phone_lookup = write.phone_lookup.map(|h| h.as_bytes().to_vec());
        let roles = write.roles.map(|rs| {
            rs.iter()
                .map(|r| role_to_wire(*r).to_string())
                .collect::<Vec<String>>()
        });

        let tx = self.begin().await?;
        let n = tx
            .execute_typed(
                "UPDATE members SET \
                   name_encrypted    = COALESCE($2, name_encrypted), \
                   phone_lookup_hash = COALESCE($3, phone_lookup_hash), \
                   phone_encrypted   = COALESCE($4, phone_encrypted), \
                   address_encrypted = COALESCE($5, address_encrypted), \
                   roles             = COALESCE($6::text[]::member_role[], roles) \
                 WHERE id = $1 AND floor(extract(epoch from updated_at))::bigint = $7",
                &[
                    (&mid, Type::UUID),
                    (&write.name_encrypted, Type::BYTEA),
                    (&phone_lookup, Type::BYTEA),
                    (&write.phone_encrypted, Type::BYTEA),
                    (&write.address_encrypted, Type::BYTEA),
                    (&roles, Type::TEXT_ARRAY),
                    (&expected, Type::INT8),
                ],
            )
            .await?;
        tx.commit().await?;
        Ok(if n == 1 {
            EditApplied::Updated
        } else {
            EditApplied::Stale
        })
    }

    /// Supersede the member's prior live Onboarding Code and install `new_code_hash` as the new live
    /// one in **one transaction** (AC6). The `pg_advisory_xact_lock` on the member serializes
    /// concurrent regenerate so the second one supersedes the first's committed live row rather than
    /// racing the `onboarding_codes_one_live_per_member` partial-unique index into a violation (the
    /// proven `rotate_session` / `reissue_admin_invitation` advisory-lock shape). Returns `false`
    /// (rolled back) if the member is absent from this tenant.
    async fn regenerate_code(
        &mut self,
        member_id: MemberId,
        new_code_hash: CodeHash,
        code_expires_at: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<bool> {
        let mid = member_id.as_uuid();
        let mid_text = mid.to_string();
        let group = self.group_id;
        let now_ts = to_pg_time(now);
        let exp = to_pg_time(code_expires_at);
        let hash = new_code_hash.as_bytes().to_vec();

        let tx = self.begin().await?;
        tx.execute_typed(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[(&mid_text, Type::TEXT)],
        )
        .await?;
        // The member must exist in this tenant (RLS scopes the visibility).
        let exists = tx
            .query_typed_opt("SELECT 1 FROM members WHERE id = $1", &[(&mid, Type::UUID)])
            .await?
            .is_some();
        if !exists {
            return Ok(false); // tx drops → rollback
        }
        // Supersede the prior live code(s) FIRST (frees the one-live partial index)...
        tx.execute_typed(
            "UPDATE onboarding_codes SET superseded_at = $2 \
             WHERE member_id = $1 AND consumed_at IS NULL AND superseded_at IS NULL",
            &[(&mid, Type::UUID), (&now_ts, Type::TIMESTAMPTZ)],
        )
        .await?;
        // ...THEN insert the fresh one.
        tx.execute_typed(
            "INSERT INTO onboarding_codes (id, group_id, member_id, code_hash, expires_at) \
             VALUES (gen_random_uuid(), $1, $2, $3, $4)",
            &[
                (&group, Type::UUID),
                (&mid, Type::UUID),
                (&hash, Type::BYTEA),
                (&exp, Type::TIMESTAMPTZ),
            ],
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}

impl AuditStore for PgMemberStore {
    /// The audit log for the group (AC9), optionally filtered to one member, oldest first. Returns
    /// field **names**, never values — reading it is not itself a recursive PII read.
    async fn list_audit_log(&mut self, member: Option<MemberId>) -> Result<Vec<AuditEntry>> {
        let m = member.map(|x| x.as_uuid());
        let tx = self.begin().await?;
        let rows = tx
            .query_typed(
                "SELECT admin_id, member_id, fields::text[] AS fields, request_id, created_at \
                 FROM audit_log WHERE ($1::uuid IS NULL OR member_id = $1) \
                 ORDER BY created_at ASC, id ASC",
                &[(&m, Type::UUID)],
            )
            .await?;
        tx.commit().await?;
        Ok(rows
            .iter()
            .map(|r| {
                let fields: Vec<String> = r.get("fields");
                let created_at: SystemTime = r.get("created_at");
                AuditEntry {
                    timestamp: system_time_to_unix(created_at),
                    admin_id: MemberId::from_uuid(r.get::<_, Uuid>("admin_id")),
                    member_id: MemberId::from_uuid(r.get::<_, Uuid>("member_id")),
                    fields: fields
                        .iter()
                        .filter_map(|s| audit_field_from_wire(s))
                        .collect(),
                    request_id: r.get("request_id"),
                }
            })
            .collect())
    }
}

impl DelegatedKeyStore for PgMemberStore {
    /// The group's KEK-wrapped secretbox key (`delegated_keys.wrapped_key`), or `None` if the Group
    /// was never bootstrapped (issuance then fails closed, AC12). RLS scopes the SELECT to this
    /// tenant's single row (PK `group_id`), so the wrapped bytes — never the plaintext key — are all
    /// that leaves the store layer (the KEK lives in `MemberConfig`, unwrapped by the orchestration).
    async fn current_wrapped_key(&mut self) -> Result<Option<Vec<u8>>> {
        let tx = self.begin().await?;
        let row = tx
            .query_typed_opt("SELECT wrapped_key FROM delegated_keys", &[])
            .await?;
        tx.commit().await?;
        Ok(row.map(|r| r.get::<_, Vec<u8>>("wrapped_key")))
    }
}
