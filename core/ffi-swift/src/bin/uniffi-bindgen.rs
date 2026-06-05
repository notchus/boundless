//! Pinned `uniffi-bindgen` CLI for this crate's exact `uniffi` version (library mode).
//!
//! Built ONLY with `--features bindgen` (host-only — see Cargo.toml `[[bin]] required-features`),
//! so clap & friends never compile into the iOS staticlib. Invoked by
//! `scripts/build-boundlesskit.sh`:
//!
//! ```text
//! cargo run --release --features bindgen --bin uniffi-bindgen -- \
//!     generate --library <host .dylib> --language swift --out-dir <dir>
//! ```

fn main() {
    uniffi::uniffi_bindgen_main()
}
