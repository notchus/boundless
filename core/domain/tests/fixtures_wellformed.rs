//! Guards the golden fixtures authored in T02: every file under `fixtures/{auth,manifest,
//! compat}` must be valid JSON, and wherever a fixture carries a field that maps onto a
//! T02 type, that value must be a legal instance of the type. This ties the fixtures to
//! the domain types without coupling them to the not-yet-frozen wire DTOs (T10).

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use boundless_domain::{AppVersion, Platform, Role};

fn fixtures_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/core/domain`; fixtures live at `<repo>/fixtures`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
}

/// Recursively validate any field that maps onto a T02 domain type.
fn check_known_fields(value: &serde_json::Value, ctx: &str) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                match key.as_str() {
                    "client_min_version" | "client_recommended_version" | "app_version" => {
                        if let Some(s) = val.as_str() {
                            AppVersion::from_str(s).unwrap_or_else(|e| {
                                panic!(
                                    "{ctx}: field `{key}` = {s:?} is not a valid AppVersion: {e}"
                                )
                            });
                        }
                    }
                    "platform" if val.is_string() => {
                        serde_json::from_value::<Platform>(val.clone()).unwrap_or_else(|e| {
                            panic!("{ctx}: field `platform` = {val} is not a valid Platform: {e}")
                        });
                    }
                    "role" if val.is_string() => {
                        serde_json::from_value::<Role>(val.clone()).unwrap_or_else(|e| {
                            panic!("{ctx}: field `role` = {val} is not a valid Role: {e}")
                        });
                    }
                    "roles" if val.is_array() => {
                        serde_json::from_value::<Vec<Role>>(val.clone()).unwrap_or_else(|e| {
                            panic!("{ctx}: field `roles` = {val} is not a valid [Role]: {e}")
                        });
                    }
                    _ => {}
                }
                check_known_fields(val, ctx);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                check_known_fields(item, ctx);
            }
        }
        _ => {}
    }
}

#[test]
fn all_fixtures_are_wellformed_and_consistent_with_domain_types() {
    let subdirs = ["auth", "manifest", "compat"];
    let mut json_count = 0usize;

    for sub in subdirs {
        let dir = fixtures_dir().join(sub);
        let entries = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read fixtures dir {}: {e}", dir.display()));

        for entry in entries {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue; // skip READMEs and .gitkeep
            }
            let ctx = path.display().to_string();
            let text =
                fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {ctx}: {e}"));
            let value: serde_json::Value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("{ctx}: invalid JSON: {e}"));
            check_known_fields(&value, &ctx);
            json_count += 1;
        }
    }

    // 9 auth + 4 manifest + 2 compat = 15 JSON fixtures authored in T02.
    assert!(
        json_count >= 15,
        "expected at least 15 JSON fixtures across auth/manifest/compat, found {json_count}"
    );
}

#[test]
fn all_named_t02_fixtures_present() {
    // The exact fixture set T02 authors. Downstream waves key off these names — T03 fills
    // the manifest signature vectors, T07/T08 replay the auth/compat fixtures — so an
    // accidental rename or delete should fail HERE, not only in a later wave. (Adding NEW
    // fixtures is fine; that's why the well-formedness test uses a `>= 15` floor.)
    let expected: &[(&str, &[&str])] = &[
        (
            "auth",
            &[
                "signin_ok.json",
                "phone_not_on_file.json",
                "device_bind_ok.json",
                "device_bind_invalid_expired.json",
                "below_min_version.json",
                "needs_reauth_help.json",
                "driver_recovery_ok.json",
                "admin_webauthn_register.json",
                "admin_invite_expired.json",
            ],
        ),
        (
            "manifest",
            &[
                "verify_ok.json",
                "verify_fail_with_cache.json",
                "verify_fail_no_cache.json",
                "lower_version_ignored.json",
            ],
        ),
        ("compat", &["n_minus_1.json", "n_minus_2.json"]),
    ];

    for (sub, names) in expected {
        for name in *names {
            let path = fixtures_dir().join(sub).join(name);
            assert!(path.is_file(), "missing T02 fixture: {}", path.display());
        }
    }
}
