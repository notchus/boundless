//! P2 (sec-audit F1): the secret-bearing response types must expose **no** formatter and **no**
//! `Serialize` — making the "holds a tainted field, so can't derive these" property a *compile-time*
//! guarantee (mirrors `core/crypto` + `core/domain`), not a structural accident. If a maintainer
//! adds a non-tainted field plus `#[derive(Debug)]`/`Serialize` "for a test", this fails the build.

use boundless_server_core::{BindResponse, RecoveryResponse, RefreshResponse, SessionMaterial};
use static_assertions::assert_not_impl_any;

assert_not_impl_any!(SessionMaterial: core::fmt::Debug, core::fmt::Display, serde::Serialize);
assert_not_impl_any!(BindResponse: core::fmt::Debug, core::fmt::Display, serde::Serialize);
assert_not_impl_any!(RefreshResponse: core::fmt::Debug, core::fmt::Display, serde::Serialize);
assert_not_impl_any!(RecoveryResponse: core::fmt::Debug, core::fmt::Display, serde::Serialize);

// A trivial test so the file is a valid integration-test target (the assertions above run at
// compile time regardless).
#[test]
fn secret_bearing_responses_are_unformattable_and_unserializable() {}
