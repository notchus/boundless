//! AC9 / O1 — the N-2 compatibility replay harness (spec 001 **T08**).
//!
//! The server supports the current client minor and the two previous **minor** versions (O1). This
//! harness replays the frozen request fixtures in `fixtures/compat/**` — real `/api/auth/signin`
//! request shapes captured for the current minor and the two prior ones — through the **core**
//! version policy (`boundless_auth::evaluate_version` against `minimum_supported(current, 2)`), and
//! asserts each is **accepted** (never the below-minimum degradation) while a client below the floor
//! **is** degraded (O4/O8). Because the version handshake is the only version-dependent gate (O6:
//! matching never reads the client version, and the auth endpoints don't otherwise branch on it),
//! this *is* "the server accepts N-2 for all auth endpoints" at the decision layer; the end-to-end
//! proof through the real `AuthService::sign_in` orchestration lives in `core/server/tests/compat.rs`.
//!
//! **Scope (what this does and does not catch).** This harness exercises only the **version
//! handshake** — it parses each fixture's `client_version` and asserts the support-window decision.
//! It deliberately does **not** replay the request *body* against a wire DTO: the sign-in body has no
//! core `Serialize` type yet (`SignInRequest` holds a tainted `PhoneNumber`, I3 — its plaintext never
//! crosses the wire implicitly), so request-shape fidelity for older minors is owned by the OpenAPI
//! contract test (AC7 / T10) and the deployable-Worker replay (T08-shell), not here. The `body`
//! fields in the fixtures are illustrative until then. Bumping the server minor means bumping
//! [`CURRENT`] and adding a matching `current.json` (the prior `current` becomes the new `n_minus_1`).

use std::fs;
use std::path::PathBuf;

use boundless_auth::{evaluate_version, minimum_supported, VersionRequirement, VersionVerdict};
use boundless_domain::AppVersion;
use serde_json::Value;

/// The "current" server minor this harness is pinned to. The N-2 window is computed from it.
const CURRENT: AppVersion = AppVersion::new(1, 2, 0);

/// The number of previous **minor** versions the server supports beyond the current one (O1: N-2).
const N_MINUS: u32 = 2;

/// The version requirement the server advertises: `client_min_version = minimum_supported(current,
/// 2)` (the O1 window), recommended = current.
fn requirement() -> VersionRequirement {
    VersionRequirement::new(minimum_supported(CURRENT, N_MINUS), CURRENT)
}

/// `<repo>/fixtures/compat` (CARGO_MANIFEST_DIR is `<repo>/server`).
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join("compat")
}

/// Parse a compat fixture's reported `client_version.app_version`.
fn fixture_version(name: &str) -> AppVersion {
    let path = fixtures_dir().join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let v: Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let s = v["client_version"]["app_version"]
        .as_str()
        .unwrap_or_else(|| {
            panic!(
                "{}: client_version.app_version must be a string",
                path.display()
            )
        });
    s.parse::<AppVersion>()
        .unwrap_or_else(|e| panic!("{}: malformed app_version {s:?}: {e:?}", path.display()))
}

#[test]
fn ac9_auth_endpoints_nminus2() {
    let req = requirement();

    // The current minor + the two previous supported minors are all ACCEPTED (not below-min).
    for name in ["current.json", "n_minus_1.json", "n_minus_2.json"] {
        let v = fixture_version(name);
        assert_eq!(
            v.major, CURRENT.major,
            "{name}: a supported fixture must be within the current major"
        );
        assert!(
            v.minor <= CURRENT.minor && v.minor + N_MINUS >= CURRENT.minor,
            "{name}: minor {} is outside the N-{N_MINUS} window of current minor {}",
            v.minor,
            CURRENT.minor
        );
        assert_ne!(
            evaluate_version(&v, &req),
            VersionVerdict::BelowMinimum,
            "{name}: a client within the N-{N_MINUS} window must be accepted (O1)"
        );
    }

    // The floor itself is supported (boundary); one minor below the floor is degraded (O4/O8 — the
    // only rejection the version handshake makes).
    let floor = minimum_supported(CURRENT, N_MINUS);
    assert_ne!(
        evaluate_version(&floor, &req),
        VersionVerdict::BelowMinimum,
        "the floor (client_min_version) is itself supported"
    );
    let under_floor = AppVersion::new(0, 9, 9);
    assert_eq!(
        evaluate_version(&under_floor, &req),
        VersionVerdict::BelowMinimum,
        "a client below client_min_version must be degraded (O4)"
    );
}
