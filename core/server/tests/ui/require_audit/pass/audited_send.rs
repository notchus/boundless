//! The sanctioned path still compiles + runs: an allowlisted PII-free response — the member list
//! `Vec<MemberSummary>` (AC8) — goes through the `admin_response_body` seam with no audit, because
//! listing is not an audited read (name-alone is not the P2-sensitive unit).
use boundless_server_core::{admin_response_body, MemberSummary};

fn main() {
    let list: Vec<MemberSummary> = Vec::new();
    let body = admin_response_body(&list).expect("serialize empty member list");
    assert_eq!(body, "[]");
}
