//! Real-Postgres integration tests for the I5 audit trail on [`PgMemberStore`] (spec 008 **T07**):
//! the audit row written atomically with an audited PII detail read, and the read-back through the
//! [`AuditStore`] port — proving the log stores field **names**, never values (AC9).
//!
//! Harness lives in `common`.

mod common;

use boundless_auth::UnixSeconds;
use boundless_domain::{MemberId, Role};
use boundless_server_core::{
    AuditEntry, AuditField, AuditStore, DuplicateDisclosureAudit, InsertMemberOutcome, MemberStore,
    NewMemberWrite,
};
use uuid::Uuid;

use common::*;

const NOW: i64 = 1_700_000_000;

fn write_for(created_by: MemberId, raw_phone: &str, name: &str, address: &str) -> NewMemberWrite {
    NewMemberWrite {
        created_by,
        roles: vec![Role::Rider],
        phone_lookup: phone_lookup(raw_phone),
        phone_encrypted: enc(raw_phone, 1),
        name_encrypted: enc(name, 2),
        address_encrypted: enc(address, 3),
        onboarding_code_hash: onb_code_hash("C"),
        code_expires_at: UnixSeconds::new(NOW + 72 * 60 * 60),
    }
}

/// Issue a member and return its id.
async fn issue(
    store: &mut PgMemberStoreAlias,
    admin: MemberId,
    raw: &str,
    name: &str,
    addr: &str,
) -> MemberId {
    let InsertMemberOutcome::Created(m) = store
        .insert_member(
            write_for(admin, raw, name, addr),
            DuplicateDisclosureAudit {
                request_id: "u".into(),
                fields: vec![AuditField::Name],
            },
            UnixSeconds::new(NOW),
        )
        .await
        .unwrap()
    else {
        panic!("created");
    };
    m
}

// A local alias so the helper signature reads clearly (the store type lives in the crate).
type PgMemberStoreAlias = boundless_server_store::PgMemberStore;

#[tokio::test]
async fn pg_audit_store_writes_row_on_detail_read() {
    // AC7: an audited detail read writes exactly one audit row (timestamp, admin id, member id, the
    // fields read, request id), and the AuditStore port reads it back.
    let url = url_or_skip!();
    let su = setup(&url, "s_a_write").await;
    let g = Uuid::from_u128(G);
    let admin = mid(7);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_a_write", g).await;
    let m = issue(&mut store, admin, "+15550000301", "Maria", "1 Oak").await;

    // A not-found read writes NO audit row.
    let absent = AuditEntry {
        timestamp: UnixSeconds::new(NOW),
        admin_id: admin,
        member_id: mid(999),
        fields: vec![AuditField::Name],
        request_id: "absent".into(),
    };
    assert!(store
        .read_member_detail_audited(mid(999), absent)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        audit_rows(&su, mid(999).as_uuid()).await,
        0,
        "a not-found read writes no audit"
    );

    // A real detail read writes exactly one audit row.
    let audit = AuditEntry {
        timestamp: UnixSeconds::new(NOW + 1),
        admin_id: admin,
        member_id: m,
        fields: vec![AuditField::Name, AuditField::Phone, AuditField::Address],
        request_id: "read-req".into(),
    };
    let pii = store
        .read_member_detail_audited(m, audit)
        .await
        .unwrap()
        .expect("found");
    assert_eq!(dec(&pii.phone_encrypted), "+15550000301");
    assert_eq!(
        audit_rows(&su, m.as_uuid()).await,
        1,
        "the audited read wrote one row"
    );

    // Read it back through the AuditStore port (filtered to the member).
    let entries = store.list_audit_log(Some(m)).await.unwrap();
    assert_eq!(entries.len(), 1);
    let e = &entries[0];
    assert_eq!(e.admin_id, admin);
    assert_eq!(e.member_id, m);
    assert_eq!(e.request_id, "read-req");
    assert_eq!(
        e.timestamp,
        UnixSeconds::new(NOW + 1),
        "server-time, whole seconds"
    );
    assert_eq!(
        e.fields,
        vec![AuditField::Name, AuditField::Phone, AuditField::Address]
    );

    // The whole-group log (no filter) also includes it.
    assert_eq!(store.list_audit_log(None).await.unwrap().len(), 1);
}

#[tokio::test]
async fn pg_audit_store_read_returns_no_pii() {
    // AC9: the audit log stores field NAMES, not values — reading it is not a recursive PII read.
    let url = url_or_skip!();
    let su = setup(&url, "s_a_nopii").await;
    let g = Uuid::from_u128(G);
    let admin = mid(7);
    seed_group(&su, g).await;

    let mut store = app_member_store(&url, "s_a_nopii", g).await;
    let m = issue(
        &mut store,
        admin,
        "+15550000302",
        "Maria Magdalena",
        "47 Willow Lane",
    )
    .await;
    let audit = AuditEntry {
        timestamp: UnixSeconds::new(NOW),
        admin_id: admin,
        member_id: m,
        fields: vec![AuditField::Name, AuditField::Phone, AuditField::Address],
        request_id: "r".into(),
    };
    store
        .read_member_detail_audited(m, audit)
        .await
        .unwrap()
        .expect("found");

    // The persisted row holds the field NAMES, and NONE of the member's actual PII values.
    let raw: String = su
        .query_one(
            "SELECT array_to_string(fields, ',') || '|' || request_id FROM audit_log WHERE member_id=$1",
            &[&m.as_uuid()],
        )
        .await
        .unwrap()
        .get(0);
    assert!(
        raw.contains("name") && raw.contains("phone") && raw.contains("address"),
        "field names recorded"
    );
    assert!(
        !raw.contains("Maria Magdalena"),
        "the name value is never stored in the audit log"
    );
    assert!(
        !raw.contains("Willow"),
        "the address value is never stored in the audit log"
    );
    assert!(
        !raw.contains("15550000302"),
        "the phone value is never stored in the audit log"
    );

    // The port round-trips only AuditField names (the type cannot carry a value).
    let entries = store.list_audit_log(Some(m)).await.unwrap();
    assert_eq!(
        entries[0].fields,
        vec![AuditField::Name, AuditField::Phone, AuditField::Address]
    );
}
