//! T06 — the I5 require-audit **compile** gate (spec 008; AC7 compile leg).
//!
//! `trybuild` proves the un-audited PII paths **fail to build**:
//! - `ui/require_audit/forge_disclosure.rs` — the audited carrier `PiiDisclosure` is un-forgeable
//!   outside the core (its constructor is `pub(crate)`), so the Worker cannot fabricate one.
//! - `ui/require_audit/send_unaudited.rs` — the `admin_response_body` seam's `AuditedResponse` bound
//!   rejects a hand-rolled PII body (it is not a `PiiDisclosure` nor an allowlisted PII-free type).
//!
//! The sanctioned PII-free path (`ui/require_audit/pass/audited_send.rs`) still compiles + runs.
//!
//! The gate's design is in `core/server/src/audited.rs`. The repo toolchain is pinned
//! (`rust-toolchain.toml`), so the committed `.stderr` golden files are stable; regenerate with
//! `TRYBUILD=overwrite cargo test -p boundless-server-core --test require_audit` on a toolchain bump.

#[test]
fn require_audit_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/require_audit/*.rs");
    t.pass("tests/ui/require_audit/pass/*.rs");
}
