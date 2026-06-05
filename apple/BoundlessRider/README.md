# BoundlessRider — the Rider's native iOS onboarding UI

`RiderShared` is the Rider onboarding feature module (spec 001 **T11**). Every screen **renders**
the `core::auth` onboarding state machine exported by [`BoundlessKit`](../BoundlessKit) — the Swift
never re-implements the rules (constitution **P4** / ADR-0001). It is a Swift Package (library +
tests), driven on the iOS simulator via `xcodebuild test`; there is **no `.xcodeproj` app bundle**
here (see "Deferred" below).

## What's in it

- `Sources/RiderShared/Onboarding/` — one screen per `OnboardingState` (helper intro → phone entry →
  Onboarding Code → permissions(+declined) → auto-update step/enabled → silent complete), the calm
  `BelowMinVersion`/`NeedsReauthHelp` screens (never a form, never an "Update Now" CTA), the `Offline`
  overlay, the `OnboardingViewModel` (drives `BoundlessKit.onEvent`), and `OnboardingRouter`.
- `Sources/RiderShared/Localization/Onboarding.xcstrings` — the String Catalog (all user-visible copy;
  no string literals in views — **P8**), resolved via `L10n` from `Bundle.module`.
- `Sources/RiderShared/Settings/` — Rider Settings, with **no** automatic-updates toggle (AC6, O3).
- `Tests/RiderSharedTests/` — the four required a11y snapshot variants per screen (default / largest
  Dynamic Type / dark / RTL — AC11) plus the VoiceOver-order, no-signup-route (AC1(b)), no-toggle
  (AC6), AC5/AC8/AC14/AC15 logic tests. Baselines live in `__Snapshots__/` (committed).

## Test (iOS simulator)

```sh
bash scripts/test-boundlessrider.sh          # builds BoundlessKit first, then runs the suite
```

To re-record snapshot baselines after an intentional UI change:

```sh
TEST_RUNNER_SNAPSHOT_RECORD=1 bash scripts/test-boundlessrider.sh   # records, then re-run to verify
```

The simulator is auto-detected (override with `BOUNDLESS_SIM="iPhone 17 Pro"`). If Xcode is installed
but not selected, the script falls back to `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`.

## Deferred (the imperative shell — DEFERRED.md → Apple / Rider UI T11)

The deployable `.xcodeproj` app bundle, the `swift-openapi-generator` HTTP client (the real
`OnboardingNetworking`), Keychain refresh-token storage (§10-F), APNs registration, and the signed
KV-manifest fetch/verify (the real `ManifestProviding`). All side effects are behind injected
`@MainActor` protocols, so the shell drops in without touching the views.
