# ADR-0016: Authentication & Device-Binding Model

- **Status:** Accepted
- **Date:** 2026-06-04
- **Author:** Boundless founder
- **Deciders:** Boundless founder

## Context

Spec `001-onboarding` defines the onboarding flow and the auth *surface*, but
deliberately deferred four foundational design decisions to an ADR (its open questions
OQ1, OQ2, OQ3, OQ5). ADR-0015 already settled how an Admin's *registration link* is
delivered (the channel). This ADR settles the rest of the member-and-admin
authentication model so `/speckit.plan` and the `architect` can plan against fixed
decisions rather than guesses:

- **OQ1 — Device-binding mechanism.** How does a device prove it belongs to an
  already-issued member at first launch?
- **OQ2 — Session lifetime & refresh.** How long does a member stay signed in, and what
  (if anything) forces re-authentication? This underwrites the constitutional promise
  (P10) that the elderly Rider, Maria, is *never* dropped to a sign-in form on a routine
  weekly open (spec AC15).
- **OQ3 — Device replacement / recovery.** What happens when a member gets a new phone or
  loses access?
- **OQ5 — WebAuthn parameters for Admins.** What authenticators are allowed, and how does
  an Admin recover a lost key?

The always-loaded docs constrain the space: **P11** (free, no paid dependency) and **I8**
(no third-party trackers/providers) disfavor any SMS/email gateway; the **Closed Group**
model (glossary) means accounts are issued, never self-created; **I4** requires device
tokens to be invalidated on auth change; **I3** keeps the phone number hashed for lookup;
**I12** makes deletion a first-class path; **P10** forbids surprising the Rider.

## Decision

### D1 — Device binding: an admin-issued one-time **Onboarding Code** (OQ1)

A member's device is bound at first launch by an **Onboarding Code**: a short,
single-use secret the Admin generates when issuing the member (UI in spec 008), conveyed
to the trusted helper out of band, and entered during the first-launch `DeviceBinding`
step. Properties:

- **Single-use**, **short-TTL**, **rate-limited** on attempts, and **validated
  server-side** (never against the device clock — so a wrong client clock cannot grant or
  deny access, and binding cannot complete offline).
- Carries **no PII**; it is an opaque secret tied to the `MemberId`.
- **Regenerable** by an Admin (issuing a fresh code invalidates the prior one).
- **No SMS/email provider** is introduced — this is the decisive reason the code beats
  SMS OTP and magic links (P11, I8).

On successful bind, the server issues a session and the client registers a `DeviceToken`
bound to `(member_id, platform, app_version)` (I4).

The user-facing noun **Onboarding Code** is registered in `docs/domain-glossary.md`
(code type `OnboardingCode`).

### D2 — Sessions: indefinite, with silent server-side refresh (OQ2)

Member (Rider/Driver) sessions are **indefinite** and do **not** expire from inactivity.
The client holds a long-lived refresh credential and a short-lived access token; the
access token is refreshed **silently** on each launch/refresh with refresh-token
rotation. A routine weekly open therefore **never** triggers a visible re-auth.

A member session ends **only** on one of these **admin-mediated** events:

1. **Admin revoke / logout** (admin action, or member-requested logout).
2. **New-device re-onboarding** (binding a new device invalidates the prior device's
   token, per I4).
3. **Account deletion** (I12).

When a session does end:

- A **Rider** is routed to the calm `NeedsReauthHelp` screen — **never** a sign-in form —
  and the server emits a rate-limited admin alert (spec AC15, P10).
- A **Driver** may be routed to interactive re-auth (`PhoneEntry` → `auth.signin_again`),
  because Drivers self-onboard.

**Admin** (WebAuthn) sessions are **separate and shorter-lived**: an Admin re-asserts
their passkey/security key per browser session. WebAuthn re-assertion is low-friction on
the laptop surface, so indefinite admin sessions are neither needed nor desirable.

### D3 — Recovery: admin-mediated for Riders, self-serve for Drivers (OQ3)

- **Riders** recover **only** through the Admin: the Admin re-issues an Onboarding Code,
  the new device binds, and the old device's token is invalidated (I4). Riders have **no**
  self-serve recovery path — consistent with P10 (no homework) and the closed-group model.
- **Drivers** may **self-serve**: at the Driver's onboarding, the client captures a
  single-use, driver-held **Recovery Code** (shown once, "keep this somewhere safe"). On a
  new device the Driver enters their phone number + the Recovery Code to re-bind without
  Admin involvement; the old token is invalidated (I4) and a fresh Recovery Code is issued.
  If the Driver lacks the Recovery Code, they fall back to the Admin re-issue path. No
  SMS/email is used for recovery (P11, I8).

The Recovery Code is registered in `docs/domain-glossary.md` (code type `RecoveryCode`).

### D4 — Admin WebAuthn: passkeys or hardware keys; Developer re-invite to recover (OQ5)

- **Allowed authenticators:** platform or roaming **passkeys** *or* hardware security keys
  (e.g. YubiKey). Broad device support keeps friction low for volunteer admins like Sarah.
- **Discoverable (resident) credentials** are preferred (usernameless sign-in).
- **User verification is required** (`userVerification: "required"`).
- **Attestation is not required** (`attestation: "none"`) — requiring attestation would
  exclude common platform passkeys for no proportionate benefit here.
- **Multiple credentials per Admin are allowed and encouraged** (register a backup key) to
  mitigate lost-key lockout.
- **Recovery** of a lost/all-lost credential is **Developer re-invite**: the Developer
  mints a fresh registration link (ADR-0015); completing registration **revokes the prior
  credential(s)** for that Admin. This is the only Admin-recovery path (no email/SMS reset,
  no security questions).

Note: the Developer's *own* auth remains hardware-key-backed per I11; D4 governs **Admins**
(Sarah), not the Developer.

## Considered alternatives

### For D1 (device binding)
- **SMS one-time code (OTP)** — familiar, but introduces a paid SMS provider and transmits
  the phone number to a third-party gateway. Rejected: violates the spirit of P11 and I8,
  and the helper (not the Rider) runs setup anyway, so SMS convenience is moot.
- **Magic link (email/SMS)** — same provider/PII concerns; additionally awkward for elderly
  Riders who don't manage email. Rejected.
- **Chosen: admin-issued Onboarding Code** — no provider, no new PII surface, fits the
  closed-group issuance model.

### For D2 (sessions)
- **Short sessions with periodic re-auth** — simplest server/security model, but breaks the
  Maria guarantee (P10). Rejected for the rider surface.
- **Time-boxed (e.g. 90-day) sessions with silent refresh** — bounds the blast radius of a
  stolen refresh token, but a long-dormant Rider device would still eventually hit the help
  screen with no triggering admin action. Rejected as the default; the indefinite model is a
  better persona fit, and the I4/revoke/delete events already bound risk. (Token theft is
  mitigated by refresh-token rotation + device binding, not by forced expiry.)
- **Chosen: indefinite + silent refresh**, ended only by admin-mediated events.

### For D3 (recovery)
- **Admin re-issue for everyone** — one code path, simplest to secure, but pushes every
  driver phone-swap through the admin. Rejected as the default because capable Drivers
  (Daniel, Tobias) can safely self-serve and admins are volunteers we shouldn't bottleneck.
- **Self-serve for everyone** — rejected: Riders must stay admin-mediated (P10), and a
  rider-facing self-serve recovery is exactly the homework the personas forbid.
- **Chosen: Riders via Admin, Drivers self-serve (Recovery Code) with Admin fallback.**

### For D4 (admin WebAuthn)
- **Hardware security keys only, attestation required** — highest assurance and mirrors the
  Developer's model, but imposes cost/friction on volunteer admins and excludes platform
  passkeys. Rejected as the default; an Admin *may* still choose a hardware key.
- **Chosen: passkeys or hardware keys, UV required, no attestation, Developer re-invite
  recovery, backup credential encouraged.**

## Consequences

### Positive
- The Rider never faces a sign-in form on a routine open — the spec's AC15 guarantee is now
  backed by a concrete session model (P10).
- No SMS/email provider anywhere in auth or recovery (P11, I8).
- Drivers get convenient self-service recovery without weakening the Rider's privacy model.
- Volunteer admins onboard with whatever authenticator they have; a backup key prevents
  lockout; recovery is a clean Developer re-invite.

### Negative / costs
- **Indefinite refresh tokens** are a long-lived secret on the device; mitigated by
  refresh-token rotation, device binding (I4), and admin revoke. A lost/stolen *unlocked*
  phone retains access until the Admin revokes — acceptable given the closed-group, low-value
  threat model, and addressable later with optional device-level biometric lock (out of scope,
  spec 001).
- **Driver Recovery Code** is a new secret a Driver must keep; if lost, they fall back to the
  Admin path (graceful, not a lockout).
- **Two recovery paths** (rider/driver) mean two code paths to build and test (spec AC19).

### Neutral / follow-ups
- The concrete TTLs (Onboarding Code, access-token lifetime), rate-limit thresholds, and the
  refresh-rotation wire format are implementation details for `/speckit.plan`, not fixed here.
- Auth/identity/token logic lives in the Rust core (`core::auth`, ADR-0001/P4); WebAuthn
  server verification is server-side; libsodium is the shared crypto (matching ADR-0014).
- This ADR closes spec 001's remaining open questions (OQ1/2/3/5) and the Onboarding-Code
  glossary registration.

## Compliance
- **Constitution:** P10 (Rider never surprised — D2/D3), P11 (no paid dependency — D1/D3),
  P4 (logic in core), P9 (each new property is testable — spec AC17–AC20).
- **Privacy invariants:** I3 (phone hashed), I4 (token invalidation on device change/auth
  change — D2/D3), I8 (no third parties — D1/D3), I12 (deletion ends sessions — D2). I11 is
  unaffected here (D4 governs Admins, not Developer provisioning); ADR-0015 governs the
  Admin invite channel.
- **Glossary:** adds `Onboarding Code` and `Recovery Code`.
- **Spec:** `specs/001-onboarding/spec.md` cites this ADR; new ACs AC17–AC20 make D1–D4
  testable; open questions OQ1/2/3/5 moved to Resolved.

## References
- `specs/001-onboarding/spec.md` — onboarding flow, AC15, AC17–AC20
- ADR-0015 (admin invitation channel) — the Developer re-invite path used for D4 recovery
- ADR-0014 (server-driven config) — shared libsodium crypto; manifest fetched at launch
- ADR-0001 (Rust core) — `core::auth` ownership of auth logic
- `docs/privacy-invariants.md` — I3, I4, I8, I11, I12
- `docs/personas.md` — Maria (P10 driver of D2/D3), Daniel/Tobias (driver self-serve)
- [WebAuthn / Web Authentication](https://www.w3.org/TR/webauthn-2/)
