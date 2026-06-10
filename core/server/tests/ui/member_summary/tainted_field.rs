//! A projection meant to be PII-free cannot smuggle a tainted PII type: deriving `Serialize` over a
//! struct with a tainted field fails to build, because the tainted newtypes implement no `Serialize`
//! (`core/domain/src/tainted.rs`). This is the compile guarantee `MemberSummary`'s own
//! `#[derive(Serialize)]` relies on (AC8).
use boundless_domain::MemberName;

#[derive(serde::Serialize)]
struct LeakySummary {
    member_id: u64,
    // ERROR: `MemberName: Serialize` is not satisfied — a tainted PII type cannot derive into a
    // serializable projection.
    name: MemberName,
}

fn main() {
    let _ = LeakySummary {
        member_id: 1,
        name: MemberName::new("Maria"),
    };
}
