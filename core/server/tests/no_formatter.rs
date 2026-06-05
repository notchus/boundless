//! P2 (sec-audit F1): the secret-bearing response types must expose **no** formatter and **no**
//! `Serialize` — making the "holds a tainted field, so can't derive these" property a *compile-time*
//! guarantee (mirrors `core/crypto` + `core/domain`), not a structural accident. If a maintainer
//! adds a non-tainted field plus `#[derive(Debug)]`/`Serialize` "for a test", this fails the build.

use boundless_server_core::{
    AdminInvitation, BindResponse, RecoveryResponse, RefreshResponse, SessionMaterial,
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

// A trivial test so the file is a valid integration-test target (the assertions above run at
// compile time regardless).
#[test]
fn secret_bearing_responses_are_unformattable_and_unserializable() {}
