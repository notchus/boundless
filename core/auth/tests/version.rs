//! Client-version compatibility tests (O4 below-minimum gate, O1 N-2 window).

use boundless_auth::{evaluate_version, minimum_supported, VersionRequirement, VersionVerdict};
use boundless_domain::AppVersion;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, .. ProptestConfig::default() })]

    /// O1/O4 as one property: with `min` derived from an N-2 window over `current` and the
    /// recommended version set to `current`, the verdict is exactly the three-way ordering,
    /// and `BelowMinimum` happens **iff** the reported version is below the window floor.
    #[test]
    fn prop_n_minus_2_version_window(
        cur_major in 0u32..4,
        cur_minor in 0u32..20,
        cur_patch in 0u32..5,
        rep_major in 0u32..4,
        rep_minor in 0u32..20,
        rep_patch in 0u32..5,
    ) {
        let current = AppVersion::new(cur_major, cur_minor, cur_patch);
        let min = minimum_supported(current, 2);
        let recommended = current; // server recommends its current version
        let req = VersionRequirement::new(min, recommended);
        let reported = AppVersion::new(rep_major, rep_minor, rep_patch);

        let verdict = evaluate_version(&reported, &req);
        let expected = if reported < min {
            VersionVerdict::BelowMinimum
        } else if reported < recommended {
            VersionVerdict::SupportedButOutdated
        } else {
            VersionVerdict::Supported
        };
        prop_assert_eq!(verdict, expected);
        prop_assert_eq!(verdict.is_below_minimum(), reported < min);
    }
}

#[test]
fn n_minus_2_window_floor_is_in_support() {
    // A server on 1.5.x supports down to 1.3.0 (current minor − 2). The floor itself is
    // in-window (a straggler, not degraded); one patch below the floor's minor is out.
    let current = AppVersion::new(1, 5, 2);
    let min = minimum_supported(current, 2);
    assert_eq!(min, AppVersion::new(1, 3, 0));

    let req = VersionRequirement::new(min, current);
    assert_eq!(
        evaluate_version(&AppVersion::new(1, 3, 0), &req),
        VersionVerdict::SupportedButOutdated
    );
    assert_eq!(
        evaluate_version(&AppVersion::new(1, 2, 9), &req),
        VersionVerdict::BelowMinimum
    );
    assert_eq!(
        evaluate_version(&AppVersion::new(1, 5, 2), &req),
        VersionVerdict::Supported
    );
}

#[test]
fn semantic_not_lexicographic_at_the_boundary() {
    // 1.10.0 must rank ABOVE 1.2.0 — a string compare would get this wrong.
    let req = VersionRequirement::new(AppVersion::new(1, 2, 0), AppVersion::new(1, 9, 0));
    assert_ne!(
        evaluate_version(&AppVersion::new(1, 10, 0), &req),
        VersionVerdict::BelowMinimum
    );
    assert_eq!(
        evaluate_version(&AppVersion::new(1, 10, 0), &req),
        VersionVerdict::Supported
    );
}
