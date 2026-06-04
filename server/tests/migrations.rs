//! Static schema-convention test for the spec-001 migrations (T06).
//!
//! This is the **always-on** half of the migration test strategy: it runs everywhere (pre-push +
//! CI) with **no Postgres and no dependencies**, encoding the schema conventions as enforced
//! invariants rather than prose. It parses the SQL text in `server/migrations/` and asserts:
//!
//! * numbering `0001`–`0008` is contiguous and each version has both a `.up.sql` and `.down.sql`
//!   (sqlx reversible convention; version must be `> 0`);
//! * every `CREATE TABLE` carries the audit columns `created_at` / `updated_at` / `created_by`
//!   (stack-matrix "Schema conventions");
//! * every table has `ENABLE` + `FORCE ROW LEVEL SECURITY` and a group-isolation `CREATE POLICY`
//!   (privacy posture; one uniform tenant policy per table);
//! * no PII/secret column is `text` — anything named like a phone, token, address, `_hash`, or
//!   `_encrypted` is `bytea` (P2/I3);
//! * `device_tokens` is keyed on exactly `(member_id, platform, app_version)` (I4);
//! * every table a migration creates is dropped by its `down`;
//! * no `pgcrypto`/`crypt(`/`digest(`/`pgp_`/`CREATE EXTENSION` (crypto is core-owned, §10-H) and
//!   no in-file `BEGIN;`/`COMMIT;`/`START TRANSACTION` (the migration runner wraps each file).
//!
//! The **live** half — that the SQL actually applies, that RLS truly isolates, and that the downs
//! cleanly revert — runs against a real Postgres via `scripts/test-migrations.sh` (self-skipping
//! locally; wired into CI's `server-migrations` job). A future task (T07) adds the sqlx `Migrator`
//! programmatic harness when it pulls in sqlx for queries.
//!
//! Parsing note: these migrations are pure ASCII outside comments, contain no `--` inside string
//! literals, and use balanced parentheses — so the small hand-rolled scanner below is sufficient
//! without an SQL-parser dependency.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// The eight migration versions this slice ships, in order.
const EXPECTED_VERSIONS: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

/// Drop `-- …` line comments. Safe for these files: no `--` appears inside any string literal or
/// identifier, so cutting each line at its first `--` only removes comments. Also strips the
/// multi-byte arrows used in comments, leaving pure ASCII for the byte-indexed scanners below.
fn strip_comments(sql: &str) -> String {
    sql.lines()
        .map(|line| match line.split_once("--") {
            Some((code, _)) => code,
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collapse all runs of whitespace to single spaces and lowercase — for substring assertions that
/// should not depend on formatting.
fn normalize(sql: &str) -> String {
    strip_comments(sql)
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every `(table_name, body)` for each `CREATE TABLE name ( … )` in `sql`, with `body` the text
/// between the outermost balanced parentheses. Operates on comment-stripped (ASCII) text.
fn create_tables(sql: &str) -> Vec<(String, String)> {
    let sql = strip_comments(sql);
    let lower = sql.to_ascii_lowercase();
    let b = sql.as_bytes();
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = lower[from..].find("create table") {
        let mut i = from + rel + "create table".len();
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        let name_start = i;
        while i < b.len() {
            let c = b[i] as char;
            if c.is_ascii_alphanumeric() || c == '_' {
                i += 1;
            } else {
                break;
            }
        }
        let name = sql[name_start..i].to_string();
        while i < b.len() && b[i] as char != '(' {
            i += 1;
        }
        let body_start = i + 1;
        let mut depth = 0i32;
        while i < b.len() {
            match b[i] as char {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        out.push((name, sql[body_start..i].to_string()));
        from = (i + 1).max(from + rel + 1);
    }
    out
}

/// Split a `CREATE TABLE` body into its top-level (depth-0) comma-separated column/constraint
/// segments.
fn top_level_segments(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in body.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// True for a table-constraint segment (vs. a column definition).
fn is_constraint(segment: &str) -> bool {
    let first = segment
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        first.as_str(),
        "constraint" | "primary" | "foreign" | "unique" | "check" | "exclude" | "like"
    )
}

/// True if a column name denotes PII or a secret that must be stored as `bytea` (P2/I3).
fn is_pii_or_secret(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("phone")
        || n.contains("token")
        || n.contains("address")
        || n.ends_with("_hash")
        || n.ends_with("_encrypted")
}

/// The normalized text of the `{table}_group_isolation` policy statement — from `create policy …`
/// up to and including the terminating `;` — or `None` if no such policy exists. Lets the RLS
/// assertions inspect the policy *predicate*, not just its name.
fn policy_statement<'a>(norm: &'a str, table: &str) -> Option<&'a str> {
    let start = norm.find(&format!(
        "create policy {table}_group_isolation on {table} "
    ))?;
    let rest = &norm[start..];
    let end = rest.find(';').map(|e| e + 1).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Identifiers created by `create <keyword> <name>` statements (e.g. `create type member_role`,
/// `create function set_updated_at(`), reading the identifier up to the next non-identifier char.
/// Used to assert that `down` migrations also revert enum types and the shared trigger function,
/// not just tables. Operates on comment-stripped (ASCII) text.
fn created_object_names(sql: &str, keyword: &str) -> Vec<String> {
    let sql = strip_comments(sql);
    let lower = sql.to_ascii_lowercase();
    let b = sql.as_bytes();
    let needle = format!("create {keyword} ");
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = lower[from..].find(&needle) {
        let mut i = from + rel + needle.len();
        while i < b.len() && (b[i] as char).is_whitespace() {
            i += 1;
        }
        let start = i;
        while i < b.len() {
            let c = b[i] as char;
            if c.is_ascii_alphanumeric() || c == '_' {
                i += 1;
            } else {
                break;
            }
        }
        if i > start {
            out.push(sql[start..i].to_ascii_lowercase());
        }
        from = i.max(from + rel + 1);
    }
    out
}

/// All `.up.sql` / `.down.sql` files grouped by integer version.
fn migration_files() -> BTreeMap<u32, (Option<String>, Option<String>)> {
    let mut by_version: BTreeMap<u32, (Option<String>, Option<String>)> = BTreeMap::new();
    for entry in fs::read_dir(migrations_dir()).expect("migrations dir exists") {
        let path = entry.expect("dir entry").path();
        let fname = path.file_name().unwrap().to_string_lossy().to_string();
        let (kind, prefix) = if let Some(p) = fname.strip_suffix(".up.sql") {
            ("up", p)
        } else if let Some(p) = fname.strip_suffix(".down.sql") {
            ("down", p)
        } else {
            panic!("unexpected file in migrations/: {fname} (only NNNN_*.{{up,down}}.sql allowed)");
        };
        let num = prefix
            .split('_')
            .next()
            .expect("filename has a numeric prefix");
        assert_eq!(
            num.len(),
            4,
            "version prefix must be zero-padded 4 digits: {fname}"
        );
        let version: u32 = num
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric prefix: {fname}"));
        assert!(
            version > 0,
            "migration version must be > 0 (sqlx rejects 0000): {fname}"
        );
        let body = fs::read_to_string(&path).expect("read migration");
        let slot = by_version.entry(version).or_default();
        match kind {
            "up" => slot.0 = Some(body),
            _ => slot.1 = Some(body),
        }
    }
    by_version
}

#[test]
fn migrations_are_numbered_0001_to_0008_with_up_and_down() {
    let files = migration_files();
    let versions: Vec<u32> = files.keys().copied().collect();
    assert_eq!(
        versions, EXPECTED_VERSIONS,
        "expected exactly migrations 0001..0008, contiguous"
    );
    for (v, (up, down)) in &files {
        assert!(up.is_some(), "migration {v:04} is missing its .up.sql");
        assert!(down.is_some(), "migration {v:04} is missing its .down.sql");
    }
}

#[test]
fn every_table_has_audit_columns_and_forced_rls_policy() {
    let files = migration_files();
    let mut total_tables = 0;
    for (v, (up, _)) in &files {
        let up = up.as_ref().unwrap();
        let norm = normalize(up);
        for (table, body) in create_tables(up) {
            total_tables += 1;
            let body_l = body.to_ascii_lowercase();
            for col in ["created_at", "updated_at", "created_by"] {
                assert!(
                    body_l.contains(col),
                    "table `{table}` (migration {v:04}) is missing the audit column `{col}`"
                );
            }
            assert!(
                norm.contains(&format!("alter table {table} enable row level security")),
                "table `{table}` (migration {v:04}) does not ENABLE row level security"
            );
            assert!(
                norm.contains(&format!("alter table {table} force row level security")),
                "table `{table}` (migration {v:04}) does not FORCE row level security"
            );
            // Assert the policy EXISTS and that its predicate is actually keyed on the tenant
            // resolver with a WITH CHECK (write) leg — not merely that a same-named policy is
            // present (which a `USING (true)` fail-open would satisfy).
            let policy = policy_statement(&norm, &table).unwrap_or_else(|| {
                panic!("table `{table}` (migration {v:04}) has no `{table}_group_isolation` RLS policy")
            });
            assert!(
                policy.contains("current_group_id()"),
                "table `{table}` (migration {v:04}) RLS policy is not keyed on the current_group_id() tenant resolver"
            );
            assert!(
                policy.contains("with check"),
                "table `{table}` (migration {v:04}) RLS policy has no WITH CHECK (write-path) clause"
            );
        }
    }
    assert_eq!(
        total_tables, 8,
        "expected exactly 8 tables across the migrations"
    );
}

#[test]
fn pii_and_secret_columns_are_bytea_never_text() {
    let files = migration_files();
    let mut checked = 0;
    for (v, (up, _)) in &files {
        for (table, body) in create_tables(up.as_ref().unwrap()) {
            for seg in top_level_segments(&body) {
                if is_constraint(&seg) {
                    continue;
                }
                let mut toks = seg.split_whitespace();
                let name = toks.next().unwrap_or("");
                let typ = toks.next().unwrap_or("");
                if is_pii_or_secret(name) {
                    checked += 1;
                    assert!(
                        typ.eq_ignore_ascii_case("bytea"),
                        "`{table}.{name}` (migration {v:04}) is `{typ}`, must be `bytea` (P2/I3)"
                    );
                }
            }
        }
    }
    // Guard against the patterns silently matching nothing (e.g. a refactor renaming the columns).
    assert!(
        checked >= 6,
        "expected to have checked the known PII/secret columns, saw {checked}"
    );
}

#[test]
fn rls_tenant_resolver_maps_unset_and_empty_to_null() {
    // The single fail-closed point behind every RLS policy: current_group_id() must turn BOTH an
    // unset GUC and an empty string into NULL (via NULLIF), so a connection that never set the
    // tenant — or a pooled connection reset to '' — sees zero rows instead of a `''::uuid` error.
    // Defined once in 0001.
    let up = migration_files()[&1].0.clone().expect("0001 up exists");
    let norm = normalize(&up);
    assert!(
        norm.contains("create function current_group_id()"),
        "0001 must define the current_group_id() RLS tenant resolver"
    );
    assert!(
        norm.contains("nullif(current_setting('app.current_group_id', true), '')"),
        "current_group_id() must map an unset/empty GUC to NULL (fail-closed) via NULLIF"
    );
}

#[test]
fn device_tokens_is_keyed_on_the_i4_binding_tuple() {
    let files = migration_files();
    let up = files[&5].0.as_ref().expect("0005 up exists");
    assert!(
        normalize(up).contains("primary key (member_id, platform, app_version)"),
        "device_tokens must be PRIMARY KEY (member_id, platform, app_version) per I4/AC4"
    );
}

#[test]
fn each_down_reverts_every_object_its_up_creates() {
    let files = migration_files();
    for (v, (up, down)) in &files {
        let up = up.as_ref().unwrap();
        let down_norm = normalize(down.as_ref().unwrap());
        for (table, _) in create_tables(up) {
            assert!(
                down_norm.contains(&format!("drop table if exists {table}")),
                "migration {v:04} down does not drop table `{table}` its up creates"
            );
        }
        // Enum types and the shared trigger function must be reverted too, else a re-run fails on
        // `CREATE TYPE`/`CREATE FUNCTION` (a hazard the table-only check would miss).
        for typ in created_object_names(up, "type") {
            assert!(
                down_norm.contains(&format!("drop type if exists {typ}")),
                "migration {v:04} down does not drop type `{typ}` its up creates"
            );
        }
        for func in created_object_names(up, "function") {
            assert!(
                down_norm.contains(&format!("drop function if exists {func}")),
                "migration {v:04} down does not drop function `{func}` its up creates"
            );
        }
    }
}

#[test]
fn migrations_use_no_pgcrypto_extensions_or_in_file_transactions() {
    let files = migration_files();
    for (v, (up, down)) in &files {
        for (which, sql) in [
            ("up", up.as_ref().unwrap()),
            ("down", down.as_ref().unwrap()),
        ] {
            let norm = normalize(sql); // comment-stripped, whitespace-collapsed, lowercased
            for forbidden in ["pgcrypto", "crypt(", "digest(", "pgp_", "create extension"] {
                assert!(
                    !norm.contains(forbidden),
                    "migration {v:04} {which} uses `{forbidden}` — crypto is core-owned (§10-H)"
                );
            }
            // Transaction control only — the plpgsql `begin … end` block (no trailing `;` after
            // `begin`) is deliberately not matched; the runner wraps each file in its own txn.
            for forbidden in [
                "begin;",
                "begin ;",
                "commit;",
                "commit ;",
                "start transaction",
                "begin transaction",
                "end transaction",
            ] {
                assert!(
                    !norm.contains(forbidden),
                    "migration {v:04} {which} contains in-file `{forbidden}` (the runner manages the transaction)"
                );
            }
        }
    }
}
