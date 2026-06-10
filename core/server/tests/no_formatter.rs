//! P2 (sec-audit F1): the secret-bearing response types must expose **no** formatter and **no**
//! `Serialize` — making the "holds a tainted field, so can't derive these" property a *compile-time*
//! guarantee (mirrors `core/crypto` + `core/domain`), not a structural accident. If a maintainer
//! adds a non-tainted field plus `#[derive(Debug)]`/`Serialize` "for a test", this fails the build.

use boundless_server_core::{
    AdminInvitation, BindResponse, MemberDetailView, PiiDisclosure, RecoveryResponse,
    RefreshResponse, SessionMaterial,
};
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(SessionMaterial: core::fmt::Debug, core::fmt::Display, serde::Serialize);
assert_not_impl_any!(BindResponse: core::fmt::Debug, core::fmt::Display, serde::Serialize);
assert_not_impl_any!(RefreshResponse: core::fmt::Debug, core::fmt::Display, serde::Serialize);
assert_not_impl_any!(RecoveryResponse: core::fmt::Debug, core::fmt::Display, serde::Serialize);
// T08: the Admin invitation holds the tainted token, so it is PII-free by construction — it can
// never be logged or serialized implicitly (P2/I8/AC16). The email body the Worker builds must
// expose only `token.expose_secret()` + the opaque admin id, never this struct.
assert_not_impl_any!(AdminInvitation: core::fmt::Debug, core::fmt::Display, serde::Serialize);
// T06 (the I5 gate): the audited PII carrier `PiiDisclosure<MemberDetailView>` and the wire detail
// `MemberDetailView` are deliberately `Serialize` (so the disclosure can emit the wire body), so —
// unlike the tainted types above — we cannot pin `!Serialize` here. But they hold the **decrypted**
// name/phone/address, so a stray `{:?}` would print PII to a log. Pin `!Debug`/`!Display` explicitly,
// so a future `#[derive(Debug)]` "for diagnostics" fails the build (the silent-derive regression this
// file exists to stop — the sole PII carrier T06 adds; sibling tainted types already covered above).
assert_not_impl_any!(PiiDisclosure<MemberDetailView>: core::fmt::Debug, core::fmt::Display);
assert_not_impl_any!(MemberDetailView: core::fmt::Debug, core::fmt::Display);

// A trivial test so the file is a valid integration-test target (the assertions above run at
// compile time regardless).
#[test]
fn secret_bearing_responses_are_unformattable_and_unserializable() {}
