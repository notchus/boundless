//! Pinned `uniffi-bindgen` CLI for this crate's exact `uniffi` version (library mode).
//!
//! Built ONLY with `--features bindgen` (host-only — see Cargo.toml `[[bin]] required-features`),
//! so clap & friends never compile into the Android `.so`. Invoked by
//! `scripts/build-corebridge.sh`:
//!
//! ```text
//! cargo run --release --features bindgen --bin uniffi-bindgen -- \
//!     generate --library <host cdylib> --language kotlin --out-dir <dir>
//! ```

fn main() {
    uniffi::uniffi_bindgen_main()
}
