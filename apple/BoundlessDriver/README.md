# BoundlessDriver (`DriverShared`)

The Driver's native iOS **onboarding** UI (spec 001 **T12**). A SwiftPM package — a testable feature
library `DriverShared` + its test target — with **no `.xcodeproj` app bundle** (the shippable app is
the deferred shell, like `BoundlessRider`).

## What it is

The Driver self-onboarding flow, **rendered from** the `core::auth` state machine exported by
`BoundlessKit` (the Swift renders the core's decisions and never re-implements the rules — P4 /
ADR-0001). It **reuses the shared onboarding kit** from `RiderShared` (the screen model + single
`OnboardingScreenView` renderer + `L10n` + the injected protocols + the role-neutral screen
factories) and adds only the Driver deltas:

- **`onboarding.driver.intro`** — a self-directed `FreshInstall` intro (the Driver runs setup
  themselves; the Rider's "…together" is helper-facing).
- **Recovery Code capture** (`DriverOnboardingScreens.recoveryCodeCapture`) — the one-time screen that
  shows the Driver's server-minted Recovery Code so they can self-serve a device replacement later
  (ADR-0016 D3 / AC19). Shown once, right after the device is bound.
- **Interactive re-auth** — a Driver whose session expires is routed by the core to `PhoneEntry`
  (`reauth_state_for(.driver)`), led with `auth.signin_again`. Unlike a Rider, who gets the form-less
  `NeedsReauthHelp` calm screen (AC15).

`DriverOnboardingViewModel` composes `RiderShared.OnboardingViewModel(role: .driver)`, so the
Driver/Rider divergence is the core's decision, not Swift's.

## Tests

```sh
bash scripts/test-boundlessdriver.sh          # build BoundlessKit, then xcodebuild test on a sim
TEST_RUNNER_SNAPSHOT_RECORD=1 bash scripts/test-boundlessdriver.sh   # (re)record snapshot baselines
```

Closes the UI legs of **AC11** (every Driver screen ×4 a11y variants + VoiceOver order), **AC14**
(declined-permission branch), **AC15** (Driver re-auth branch), **AC1(b)** (`ios_driver_no_signup_route`),
**AC19** (Recovery Code capture). The four a11y snapshot variants (default / largest Dynamic Type /
dark / RTL) live under `Tests/DriverSharedTests/__Snapshots__/`.

## Deferred (the imperative shell — see `DEFERRED.md` → Apple / Driver UI — T12-shell)

The deployable `.xcodeproj` app bundle (`app.boundless.driver`); the OpenAPI Swift HTTP client (incl.
`/api/auth/recovery/rebind`); the real `RecoveryCodeProviding` (reads `fresh_recovery_code` off the
bind/rebind response); Keychain refresh storage; APNs; signed-manifest fetch/verify; and the
**self-serve re-bind entry UI** (phone + Recovery Code on a new device — no state in the onboarding
state machine to render, so deferred). All behind injected protocols, so the shell drops in untouched.
