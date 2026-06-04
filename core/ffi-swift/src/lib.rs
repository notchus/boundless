//! `boundless-ffi-swift` — UniFFI binding crate that produces the `BoundlessKit`
//! XCFramework for the Apple platforms (docs/architecture.md §1).
//!
//! Re-exports the core domain/auth/sync surface across the UniFFI boundary; the Swift
//! clients hold no hand-rolled auth logic (P4). Output is consumed by
//! `apple/BoundlessKit/`.
//!
//! Scaffolded by spec 001 task **T01**; the UniFFI surface + XCFramework build land at
//! the contract freeze in **T10**.
