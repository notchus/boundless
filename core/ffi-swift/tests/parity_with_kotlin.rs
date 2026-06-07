//! Parity gate: `core/ffi-swift` ⇄ `core/ffi-kotlin` must expose the **identical** UniFFI surface.
//!
//! ## Why this exists
//! Both crates **mirror** the same `boundless_auth` state machine for the two native platforms (ADR-0022).
//! A change to the *core* is already caught at compile time — adding/renaming a core variant breaks both
//! crates' exhaustive `From` `match` until updated. What that compile guard does **not** catch is an
//! **FFI-only divergence**: a `#[uniffi::export]` fn or a mirror enum/variant added, renamed, or
//! signature-changed on **one** side only. The core is unchanged, both crates still compile, but the iOS
//! and Android client surfaces silently diverge — a P4 (single-source) / P7 (native-parity) violation that
//! previously only the human `platform-parity` reviewer could catch. This test makes the lock-step
//! mechanical (DEFERRED.md → Android bring-up: "surface parity is a convention, not yet a gate").
//!
//! ## What it asserts
//! The two `lib.rs` **production regions** (everything before `#[cfg(test)]`) must be **byte-identical
//! after stripping comments and blank lines**. Whole-region identity is strictly stronger than comparing
//! just the enum/variant/fn sets — it also pins the symmetric `From` mappings — and needs no Rust parser,
//! because the post-edit hook runs `cargo fmt` so both files stay canonically formatted. Doc and inline
//! comments are exempt, so the legitimate Swift-vs-Kotlin wording in the headers is free to differ. The
//! `#[cfg(test)]` modules are deliberately excluded: tests are not part of the exported surface and could
//! legitimately diverge per platform later.

const SWIFT: &str = include_str!("../src/lib.rs");
// Relative to this file (core/ffi-swift/tests/), reach the sibling mirror crate. `include_str!` registers
// the path as a build dependency, so editing ffi-kotlin's lib.rs re-triggers this test.
const KOTLIN: &str = include_str!("../../ffi-kotlin/src/lib.rs");

/// The line that opens the `#[cfg(test)]` module — the boundary between the exported surface (above) and
/// the host round-trip tests (below, excluded from the parity check).
const TEST_MARKER: &str = "#[cfg(test)]";

/// A known declaration that must survive normalization — guards against a normalizer bug that would let
/// the test pass vacuously (e.g. by reducing both surfaces to the empty string).
const ANCHOR: &str = "pub fn on_event";

/// Reduce a crate's `lib.rs` to its exported surface, normalized for comparison: keep only the region
/// before the `#[cfg(test)]` module, drop every *full-line* comment (`//!` / `///` / `//`) and blank
/// line, and trim trailing whitespace. The result is what must match byte-for-byte across the two
/// mirrors. Note that *inline* trailing comments are deliberately NOT stripped — they are part of the
/// compared surface, so a divergent inline note on a code line is a (rare, intentional) mismatch; keep
/// such notes on their own `//` line if they must differ between the mirrors.
fn surface(src: &str, label: &str) -> String {
    // The marker must appear EXACTLY once — the boundary to the `mod tests` block. Zero → we cannot
    // isolate the surface. More than one → a `#[cfg(test)]`-gated *production* item was added above
    // `mod tests`, which would let `find` truncate early and silently shrink the compared region (a
    // false-pass). Both are loud failures that demand a conscious update to this gate. (See F3.)
    let marker_count = src.matches(TEST_MARKER).count();
    assert_eq!(
        marker_count, 1,
        "{label}: expected exactly one `{TEST_MARKER}` (the test-module boundary), found {marker_count} \
         — if you added a cfg(test)-gated production item, update this gate so the compared surface \
         cannot silently shrink"
    );
    let end = src.find(TEST_MARKER).expect("marker_count asserted == 1");
    let production = &src[..end];

    let normalized: String = production
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !normalized.is_empty(),
        "{label}: normalized surface is empty — the normalizer or the source is broken"
    );
    assert!(
        normalized.contains(ANCHOR),
        "{label}: normalized surface is missing the `{ANCHOR}` anchor — the normalizer is broken"
    );
    normalized
}

/// The exported UniFFI surfaces of the Swift and Kotlin mirror crates are identical (ADR-0022, P4/P7).
#[test]
fn ffi_swift_and_kotlin_surfaces_are_identical() {
    let swift = surface(SWIFT, "ffi-swift");
    let kotlin = surface(KOTLIN, "ffi-kotlin");

    if swift != kotlin {
        let (i, (s, k)) = swift
            .lines()
            .zip(kotlin.lines())
            .enumerate()
            .find(|(_, (s, k))| s != k)
            // No differing pair found among the common prefix → one surface is longer than the other.
            .unwrap_or((
                swift.lines().count().min(kotlin.lines().count()),
                ("<none>", "<none>"),
            ));
        panic!(
            "core/ffi-swift and core/ffi-kotlin exported surfaces diverge (P4/P7 parity).\n\
             First difference at surface line {i}:\n  \
             ffi-swift : {s}\n  ffi-kotlin: {k}\n\
             (swift lines: {}, kotlin lines: {})\n\
             Keep the two mirror crates' lib.rs surfaces in lock-step — see ADR-0022.",
            swift.lines().count(),
            kotlin.lines().count(),
        );
    }
}
