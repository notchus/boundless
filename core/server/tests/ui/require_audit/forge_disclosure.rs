//! The audited PII carrier `PiiDisclosure` is un-forgeable outside `boundless-server-core`: its
//! constructor is `pub(crate)`, so a downstream crate (the future Worker) cannot fabricate one to
//! wrap arbitrary PII without going through an audited read. Naming the constructor is a privacy error.
use boundless_server_core::PiiDisclosure;

fn main() {
    // `PiiDisclosure::new` is `pub(crate)` — referencing it from this external crate does not compile.
    let _forge = PiiDisclosure::<String>::new;
}
