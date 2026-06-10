//! T06 / AC8 — a projection meant to be PII-free **rejects a tainted field at compile time**.
//!
//! `trybuild` proves that deriving `Serialize` over a struct holding a tainted PII newtype fails to
//! build (the tainted types implement no `Serialize`, `core/domain/src/tainted.rs`). This is the
//! compile guarantee `MemberSummary`'s own `#[derive(Serialize)]` relies on — pinned here as a named,
//! intentional gate rather than an implicit consequence. (Toolchain pinned — see `require_audit.rs`.)

#[test]
fn member_summary_rejects_tainted_field() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/member_summary/*.rs");
}
