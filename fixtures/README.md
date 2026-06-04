# Boundless golden fixtures

Shared golden JSON fixtures replayed across every platform's test suite (Rust + Swift +
Kotlin, and TypeScript where applicable) so behavior can't drift between platforms
(constitution P4, ADR-0001). A shared type with no fixture — or a fixture not replayed by
all required suites — is a parity gap CI flags.

| Dir | Holds | Authored in |
|---|---|---|
| `auth/` | `signin_ok`, `phone_not_on_file`, `device_bind_ok`, `device_bind_invalid_expired`, `below_min_version`, `needs_reauth_help`, `driver_recovery_ok`, `admin_webauthn_register`, `admin_invite_expired` | spec 001 **T02** |
| `manifest/` | `verify_ok`, `verify_fail_with_cache`, `verify_fail_no_cache`, `lower_version_ignored` | spec 001 **T02** |
| `compat/` | `n_minus_1`, `n_minus_2` request fixtures (O1) | spec 001 **T02** / replayed **T08** |
| `onboarding/` | `log_lines.jsonl` for the log-scrubber replay (P2/I10) | spec 001 **T16** |

Scaffolded by spec 001 task **T01**; fixtures land in **T02** (and **T16** for the
onboarding log lines).
