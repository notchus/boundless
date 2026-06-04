# 001 — Onboarding (Auth Model + First-Launch Flow)

> Spec status: Clarified (ready for `/speckit.plan`) — `/clarify` pass 2026-06-04
> Author: notch
> Date: 2026-06-04

## One-paragraph summary

Boundless is a **closed group**: there is no signup form anywhere. Every account is
*issued*, never self-created — the Developer provisions Admins, and Admins issue
member accounts (Riders and Drivers). This spec defines two foundational things on
top of that model: (1) the **authentication model** — how an already-issued member
proves who they are on a device, how device push tokens and sessions are bound and
invalidated, and how the client and server negotiate version compatibility at sign-in;
and (2) the **first-launch flow** — the in-person "set up this phone" experience that a
trusted helper performs *for* a Rider (and that a Driver may perform themselves),
ending with the calm primary surface and with OS automatic updates enabled. The Rider
must never be given setup homework, and the act of onboarding must never become an
update prompt on her primary screen.

This is the bottom-of-the-stack spec: auth and the launch handshake are what every
later surface depends on. It does **not** build the Admin's member-management UI
(that is spec `008-admin-member-management`); it defines only the auth model that
issuance produces and the device-side flow that consumes it.

## User story

- As **Sarah (Admin)**, I want to accept the Developer's invitation and register a
  strong, phishing-resistant credential, so that I can manage my group without a
  password to lose or leak.
- As **a trusted helper** (family member or admin) setting up **Maria's (Rider)** phone,
  I want a short, guided, in-person flow that signs Maria in, turns on the
  notifications she needs, and enables automatic updates, so that Maria never has to do
  any of it herself and never sees an "update required" screen later.
- As **Daniel (Driver)**, I want to sign in on my own phone quickly with the identity my
  admin issued, so that I land on my driver home with auto-update already on.

## What changes for Maria? (rider primary persona)

Maria does **not** onboard herself. Someone she trusts — her daughter, or Sarah —
sits with her once and runs a short guided flow on her phone: install the app, sign
her in with the phone number that's on file, allow the notifications that let her
doorbell card work, and turn on automatic updates. From that point Maria's app opens
straight to "You're coming tonight." She is never asked to type an address, search for
anything, choose a password, or manage updates. After setup, her Settings never shows
an "automatic updates" toggle (that lives in the OS, per O3) and her primary surface
never shows an update prompt (O8).

**Maria is never returned to a sign-in form on her own.** If her session ever becomes
invalid — an admin-initiated revoke/logout, the I4 device-change case, or account
deletion (never a routine weekly open) — or if her device falls below the minimum
supported version, she sees exactly one calm screen: "This device needs Sarah's help.
Sarah has been told." She is never routed back to `PhoneEntry`/`DeviceBinding`, and the
server alerts the admin (O4). See the `NeedsReauthHelp` state and AC15.

## What changes for Daniel? (driver primary persona)

Daniel installs the app and signs in himself with his issued phone-number identity. He
grants notification permission and confirms automatic updates are on, then lands on the
driver home with the **Seat Toggle** off by default. Daniel can also onboard his Apple
Watch, but watch pairing detail is out of scope for this spec (see Out of scope). If
Daniel reinstalls or moves to a new phone, re-onboarding invalidates the device token
on the old device (I4; see AC4). Unlike a Rider, a Driver whose session expires *may* be
routed to interactive re-auth (`PhoneEntry`, showing `auth.signin_again`), because
Drivers self-onboard.

## What changes for Sarah? (admin primary persona)

Sarah receives a one-time, single-use, short-TTL **registration link** minted by the
Developer and delivered out of band via Email Workers (permitted by **ADR-0015**, which
narrows I11 — see Privacy notes). On her laptop she opens it and registers a **WebAuthn**
credential — a passkey or a hardware security key, user verification required, no
attestation demanded; she is encouraged to register a backup key (ADR-0016 D4). She never
sets a password. If she loses her key, recovery is a Developer re-invite (ADR-0015) that
revokes the prior credential. She cannot create other Admins — only the Developer can (I11). The act of Sarah issuing member
accounts (entering address, phone, role) is the subject of spec
`008-admin-member-management`; **this** spec defines only the auth artifacts that
issuance must produce (the member identity, and the first-launch device-binding secret
the flow consumes) and the audit obligations around them (I5).

Note: an Admin who is *also* a member (Sarah on weekends) holds two distinct auth
artifacts — her Admin WebAuthn credential and a separate member phone-identity. See the
"Member holds multiple roles" edge case.

## What changes for edge personas?

- **Margaret (edge-case rider, 82):** Her daughter performs the same in-person
  first-launch flow. Margaret never re-authenticates on her own; like Maria, if her
  session becomes invalid or her device drifts below the minimum version while her
  daughter isn't around, she sees only the calm `NeedsReauthHelp`/O4 screen (never a
  sign-in form) and the server alerts the admin — Margaret takes no action.
- **Tobias (edge-case driver, variable shifts):** Self-onboards like Daniel. No
  difference at the auth layer; his irregular usage doesn't change the session model.

## Detailed behavior

### A. Admin onboarding (Developer → Admin)

1. The Developer provisions an Admin via the developer-only, hardware-key-backed
   endpoint (I11). This mints a pending Admin record and a single-use, short-TTL
   invitation token, delivered to the Admin out of band as a registration link via
   Email Workers. This delivery is explicitly permitted by **ADR-0015**, which narrows
   I11: the link carries no PII beyond the opaque token and no credential material, and
   it only initiates WebAuthn registration — it is not a self-serve signup.
2. The Admin opens the registration link on a desktop browser (admin web is
   desktop-first). The link is single-use and time-boxed (TTL validated server-side).
3. The Admin registers a **WebAuthn** credential (passkey / security key). No password
   is ever created. The public credential is stored against the Admin record; the
   invitation token is consumed (see AC16).
4. The Admin is now active and can sign in to the admin web with that credential. Admin
   sign-in is WebAuthn assertion only.

### B. Member identity (what issuance produces)

A member account, once issued by an Admin (UI in spec 008), consists of:

- A stable `MemberId` (a (Group, Person) pair; never displayed — glossary).
- One or more `Role`s (`Rider`, `Driver`); a person may hold several across contexts.
- A phone number, persisted **twice** per I3: `phone_lookup_hash` (HMAC-SHA256 with the
  per-instance secret, constant-time compared) for auth lookup, and `phone_encrypted`
  for Admin display only (audit-logged reads, I5). The phone number is the member's
  human-facing identity.
- A first-launch **Onboarding Code** (glossary; `OnboardingCode`) — the device-binding
  secret the helper enters during first-launch to prove the device belongs to this member.
  Per **ADR-0016 D1** it is an **admin-issued one-time code**: single-use, short-TTL,
  rate-limited, server-validated, regenerable by an Admin, and carrying no PII. No SMS/email
  provider is used — the decisive reason over SMS OTP / magic link (P11, I8). See AC17.
- A **Recovery Code** (glossary; `RecoveryCode`) is additionally captured for **Drivers**
  at onboarding, enabling self-serve device replacement (recovery edge case, ADR-0016 D3).

### C. Rider / Driver first-launch ("set up this phone")

The flow is identical in structure for Rider and Driver; the Rider variant is intended
to be run by a trusted helper, the Driver variant by the Driver. Steps:

1. **Install** the app from the App Store / Play Store (done by the helper for a Rider).
2. **Sign in:** enter the member's phone number; the client sends `phone_lookup_hash`
   to the server. The server's auth response includes `client_min_version` (O4) and
   `client_recommended_version` (the latter feeds the O5 stragglers panel) plus the
   manifest pointer (ADR-0014).
3. **Verify / bind device:** enter the Onboarding Code. On success the server issues a
   session — **indefinite, silently refreshed** (ADR-0016 D2) — and the client registers a
   `DeviceToken` bound to `(member_id, platform, app_version)` (I4). The code is validated
   **server-side** (single-use, short-TTL, rate-limited), so this step cannot complete
   offline.
4. **Permissions:** request notification permission. For a Rider this *aims* to include
   **Critical Alerts** (needed for the Doorbell Notification). Because the Critical Alerts
   entitlement is still pending Apple review (DEFERRED), the interim behavior is: request
   **standard** notification permission now and treat Critical Alerts as a later
   capability upgrade gated on the entitlement (resolves OQ6). Performed by the helper for
   a Rider — never left as homework for Maria. **Onboarding never blocks or scolds on a
   permission decision:** if the helper or user *declines*, or Critical Alerts is
   unavailable-because-pending, the flow still advances to the auto-update step and the
   server records a non-PII admin flag (e.g. "doorbell notifications not enabled"),
   mirroring the skippable-but-flagged pattern of `docs/update-strategy.md`. See AC14.
5. **Enable automatic updates:** the flow presents the OS auto-update step and confirms
   completion with a screen explicitly labeled **"auto-update enabled"** (O3). This is the
   only place the concept of updates ever appears to a Rider household, and it appears
   during in-person setup, not on the primary surface.
6. **Complete:** the client fetches the manifest index (`manifest:v1:index`) then the
   per-locale manifest (`manifest:v1:<locale>`), signature-verifies it (ADR-0014), and
   applies copy/translations/tokens. On verification failure it falls back per ADR-0014's
   tiers — the previously-cached manifest if one exists, else the bundled-in-binary
   catalog — and ignores any manifest whose `manifest_version` is lower than the cached
   one. It then routes to the role's primary surface — the Rider's "You're coming
   tonight," or the Driver's home with the Seat Toggle off. Completion is **silent** —
   there is no "all set" screen (voice-and-tone: no celebration of plumbing).

### D. Version-compatibility handshake (every sign-in, not just first launch)

- Every `/api/auth/*` response and every WebSocket open handshake carries
  `client_min_version` (O4) and the server supports N-2 minor versions (O1).
- If the reporting client is **below** `client_min_version`, the client shows the single
  calm graceful-degradation screen and nothing else; the server emits a rate-limited
  admin alert via Queues (O4). No "Update Now" button — the Rider cannot action it (O8).
  This degradation is reachable from **any** auth response or WebSocket open handshake —
  including a returning session that never enters `PhoneEntry`, not only the first-launch
  sign-in path.
- **Source of `{adminName}` at the degradation moment:** the per-Group admin name is
  non-PII content delivered via the signed KV manifest (ADR-0014), which the client reads
  from cache at launch before any phone entry — so the name is available even pre-sign-in
  (resolves OQ7). If no manifest is cached yet (true first launch while offline), the
  client uses the name-less fallback string `auth.below_min_version_generic`, aligned with
  the name-less canonical screen in `docs/update-strategy.md`.
- Onboarding and auth never gate matching: a too-old client still does not stop the
  driver from arriving (O6 is upstream of this, but the auth layer must not introduce a
  new dependency on client version beyond the O4 degradation gate).

## States and transitions

Device-side onboarding state machine (Rider/Driver):

| State | Entered when | Exits to |
|---|---|---|
| `FreshInstall` | App launched, no session, never onboarded | `PhoneEntry` |
| `PhoneEntry` | Sign-in begins | `DeviceBinding` (lookup ok) · `PhoneNotOnFile` (no match) · `BelowMinVersion` (handshake fails O4) · `Offline` (overlay) |
| `DeviceBinding` | Phone matched | `Permissions` (secret ok) · `BindingFailed` (bad/expired secret) · `Offline` (overlay) |
| `Permissions` | Device bound, session issued | `AutoUpdateStep` (granted) · `AutoUpdateStep` (declined → proceed + non-PII admin flag) |
| `AutoUpdateStep` | Permission decision recorded | `Complete` |
| `Complete` | Manifest applied (or fallback tier per ADR-0014) | Role primary surface |
| `PhoneNotOnFile` | Lookup miss | back to `PhoneEntry` (shows `onboarding.signin.phone_not_on_file`) |
| `BindingFailed` | Secret invalid/expired | back to `DeviceBinding` (shows `onboarding.binding.code_invalid`; helper path) |
| `BelowMinVersion` | Client < `client_min_version`, from **any** auth response / WS handshake | terminal calm screen; admin alerted (O4) |
| `NeedsReauthHelp` | A previously-valid **Rider** session expired/was invalidated, no helper present | terminal calm screen (`auth.below_min_version` pattern — never a sign-in form); admin alerted. **Drivers** instead route to `PhoneEntry` for interactive re-auth (`auth.signin_again`) |
| `Offline` | No network — an **overlay** on `PhoneEntry`/`DeviceBinding`, not a separate node | stays on the current step (bundled-catalog sign-in UI shown); the network-dependent action (lookup/bind/manifest) is deferred until connectivity, then resumes the same step. Binding cannot complete offline (server-validated) |
| `ManifestFailReturning` | Returning device, manifest fetch/verify fails (not first launch) | uses the previously-cached manifest (ADR-0014 tier 2), **not** the bundled catalog; never blocks the primary surface |

Admin onboarding state machine:

| State | Entered when | Exits to |
|---|---|---|
| `Invited` | Developer provisions admin | `Registering` (link opened) · `InviteExpired` |
| `Registering` | Admin opens single-use link | `Active` (WebAuthn registered) · `InviteExpired` |
| `Active` | Credential registered | admin sign-in (WebAuthn assertion) |
| `InviteExpired` | TTL elapsed / link reused | terminal; Developer must re-invite |

## Acceptance criteria (testable)

- [ ] **AC1** — There is no public signup or "request access" path on any client; the
      only way to obtain a member account is Admin issuance, and the only way to obtain an
      Admin account is Developer provisioning (asserts I11). Tests: (a) an integration test
      asserts unauthenticated and admin-authenticated requests to the Admin-creation
      endpoint are both rejected; (b) a per-platform client-surface inspection test
      (SwiftUI Rider/Driver, Compose, SvelteKit admin) asserts the onboarding entry flow
      exposes no "sign up" / "create account" / "request access" route (verifiable against
      the state machine, which has no signup state). The non-Admin-cannot-create-*member*
      endpoint authz test is owned by spec 008 (member issuance is out of scope here).
- [ ] **AC2** — Admin registration uses WebAuthn; no password field exists anywhere in
      the admin auth flow. Test: the admin auth surface contains no password input;
      registration completes only with a WebAuthn credential.
- [ ] **AC3** — Phone-number auth lookup uses `phone_lookup_hash` with a constant-time
      compare; the plaintext phone is never sent in an auth lookup beyond the hashing
      boundary, and never logged (asserts I3, P2). Test: `i3_phone_lookup_constant_time`
      plus a log-scrubber replay of the onboarding fixtures.
- [ ] **AC4** — On successful first-launch, a `DeviceToken` is registered bound to
      `(member_id, platform, app_version)`; on re-onboarding the same member on a new
      device, the prior device's token is invalidated (asserts the **onboarding-layer
      portion of I4** — token binding + invalidation on new-device re-onboarding; the full
      enumeration of auth-change invalidation triggers is settled in ADR-0016 D2).
      Test: a dedicated `i4_tokens_invalidated_on_reonboarding` test, distinct from
      `i4_tokens_invalidated_on_logout`.
- [ ] **AC5** — The first-launch flow contains a step that enables OS automatic updates,
      and a snapshot test of the flow includes a screen labeled "auto-update enabled"
      (asserts O3).
- [ ] **AC6** — The Rider's Settings UI does not surface an "automatic updates" toggle
      (asserts O3). Snapshot/inspection test of Rider settings.
- [ ] **AC7** — Every `/api/auth/*` response and the WebSocket open handshake include
      `client_min_version` (required, asserts O4) and `client_recommended_version`
      (asserts O5's straggler signal) as schema fields. Contract test against the
      OpenAPI schema and the proto handshake.
- [ ] **AC8** — A client reporting a version below `client_min_version` receives only the
      calm degradation screen (no "Update Now" control), and the server emits exactly one
      admin alert per member per day for it (asserts O4, O8). Integration + snapshot test.
- [ ] **AC9** — The server accepts request fixtures from the current minor and the two
      previous supported minors for all auth endpoints (asserts O1). Replay test in
      `server/tests/compat/`.
- [ ] **AC10** — On launch the client fetches the manifest index then the per-locale
      manifest, verifies its libsodium signature, and applies it before showing the primary
      surface. On verification failure it falls back per ADR-0014's tiers — the
      previously-cached manifest if one exists, and only the bundled-in-binary catalog when
      none exists (true first launch) — and ignores any manifest with a lower
      `manifest_version` than the cached one (asserts ADR-0014, O2). Tests:
      verify-fail-with-cache → uses cache; verify-fail-no-cache → uses bundled;
      lower-version-manifest → ignored; offline-first-launch → bundled catalog.
- [ ] **AC11** — Every **native-platform** onboarding screen passes the a11y bar at all
      four required snapshot variants (default, largest text, dark mode, RTL) on each
      platform (asserts P1, native half). Includes VoiceOver/TalkBack traversal of the
      helper steps.
- [ ] **AC11b** — The **admin web** onboarding screens (registration-link landing, WebAuthn
      registration ceremony, `InviteExpired` terminal, WebAuthn sign-in) pass the web a11y
      bar per `docs/a11y-bar.md` (asserts P1, web half): axe-core passes with zero
      violations in CI; the WebAuthn registration/assertion ceremony is fully
      keyboard-operable with visible focus and logical tab order; form labels and error
      messages are programmatically associated (`label-for` / `aria-describedby`); the
      invite-expired and binding-error states are announced via `aria-live`; layout reflows
      at 200% font and 400% zoom without horizontal scroll; RTL and dark mode render
      correctly. Test: Playwright + axe-core on each admin onboarding route, plus a
      keyboard-only walkthrough of the WebAuthn ceremony.
- [ ] **AC12** — No user-visible onboarding string is a literal in code; all resolve from
      the catalog and render in pseudo-locale (`zz-ZZ`) without truncation (asserts P8).
- [ ] **AC13** — No third-party analytics/SDK is added by the onboarding/auth code;
      the network allow-list check passes (asserts I8).
- [ ] **AC14** — On notification-permission denial (or Critical Alerts
      unavailable-because-pending), the onboarding flow still reaches the role's primary
      surface AND the server receives a non-PII admin flag recording that doorbell
      notifications are not enabled (never block/scold; relates O8, P10). Integration +
      snapshot test of the declined branch.
- [ ] **AC15** — A **Rider** client whose session has expired or been invalidated shows
      the calm `NeedsReauthHelp` screen (no phone-entry field, no sign-in form) at all four
      a11y variants, and the server emits exactly one admin alert per member per day
      (parallels AC8; asserts P10). A **Driver** client in the same state may instead route
      to interactive re-auth. Integration + snapshot test.
- [ ] **AC16** — The Admin invitation token is single-use and server-side TTL-bounded: a
      reused or expired link is rejected and routes to `InviteExpired`; the token is
      consumed on successful WebAuthn registration (asserts I11 as narrowed by ADR-0015).
      Integration test; TTL validated against server time, not the device clock.
- [ ] **AC17** — The Onboarding Code is single-use, short-TTL, rate-limited, and validated
      server-side; it is consumed on successful device bind, and a regenerated code
      invalidates the prior one (asserts ADR-0016 D1; relates I4). Integration test; TTL and
      attempt-limit validated against server time, not the device clock.
- [ ] **AC18** — A member session is refreshed silently and does not expire from
      inactivity; it is invalidated only by admin revoke/logout, new-device re-onboarding
      (I4), or account deletion (I12) (asserts ADR-0016 D2). Tests: a long-idle Rider session
      still opens straight to the primary surface (no re-auth); each invalidation event ends
      the session and drops a Rider to `NeedsReauthHelp` (not a sign-in form).
- [ ] **AC19** — A Driver can self-serve re-bind a new device with a valid Recovery Code
      (no Admin action), invalidating the old device's token (I4) and issuing a fresh
      Recovery Code; a Rider has no self-serve recovery path (Admin re-issue only) (asserts
      ADR-0016 D3). Integration test of both paths.
- [ ] **AC20** — Admin WebAuthn registration requires user verification, accepts passkeys
      or hardware keys, does not require attestation, and supports more than one credential
      per Admin; lost-credential recovery is a Developer re-invite (ADR-0015) that revokes
      the prior credential(s) (asserts ADR-0016 D4). Integration + admin-web test.

## Edge cases

- **No driver available / matching state:** out of scope here — onboarding precedes any
  matching and must not depend on it (O6).
- **Rider opts out at the last second:** unrelated to onboarding (spec 002).
- **Driver drops out post-match:** unrelated to onboarding (spec 009).
- **Network is down at first launch:** the client uses the bundled catalog (ADR-0014),
  shows the offline-aware sign-in, and completes the manifest fetch on next connectivity;
  no error register, calm copy. Device-binding **cannot complete offline** (the secret is
  validated server-side, per the wrong-clock case below) — `Offline` is an overlay on the
  current sign-in step, deferring the lookup/bind/manifest action until connectivity, then
  resuming the same step (resolves OQ8).
- **Network/manifest fails on a returning device (not first launch):** the client falls
  back to the previously-cached manifest (ADR-0014 tier 2), never the bundled catalog, and
  never blocks the primary surface (`ManifestFailReturning`).
- **Notification / Critical Alerts permission declined (or unavailable-because-pending):**
  the flow advances to the primary surface anyway and the server records a non-PII admin
  flag; onboarding never blocks or scolds (AC14).
- **User's clock is wrong:** device-binding secret TTL and session expiry (and the Admin
  invitation token TTL) must validate server-side, not against device clock, so a wrong
  client clock cannot grant or wrongly deny access.
- **Dynamic Type at xxxLarge / 200% font scale:** every onboarding screen reflows; touch
  targets stay ≥ 44pt (iOS) / 48dp (Android); covered by AC11/AC11b.
- **VoiceOver / TalkBack / Switch Control:** the entire flow is operable; the helper may
  be sighted/capable, but the resulting Rider surface still meets the bar (AC11).
- **Helper not present / Rider tries to self-onboard:** the flow is designed to be
  completable by a capable helper, but must degrade gracefully — never blocking, never
  scolding; copy invites calling the admin.
- **Device-binding secret expired or mistyped:** clear, warm recovery path back to the
  binding step (`onboarding.binding.code_invalid`); option to "call {adminName}."
- **Phone number not on file:** "That number doesn't match what's on file. Try again or
  call {adminName}." (voice-and-tone exemplar) — never reveals whether the number exists.
- **Device replacement / recovery:** a **Rider** recovers only via the Admin re-issuing an
  Onboarding Code; the new device binds and the old token is invalidated (I4). A **Driver**
  may self-serve with their **Recovery Code** (phone number + code → re-bind, old token
  invalidated, fresh Recovery Code issued); if lost, the Driver falls back to the Admin
  path. No SMS/email is used (ADR-0016 D3). See AC19.
- **Session expires / is invalidated with no helper present:** a Rider sees the calm
  `NeedsReauthHelp` screen, never a sign-in form (AC15); a Driver may re-auth
  interactively. The set of events that can invalidate a Rider session is constrained in
  OQ2 to admin-mediated ones (revoke/logout, I4 device change, account deletion).
- **Member holds multiple roles:** onboarding signs the device into the member identity;
  role-specific surfaces are selected post-onboarding without re-authentication. An Admin
  who is *also* a member (Sarah on weekends) holds **two distinct auth artifacts**: her
  Developer-issued Admin WebAuthn credential (laptop, Section A) and a separate
  Admin-issued member phone-identity + device-binding (phone, Sections B/C); she onboards
  her phone as a member exactly like Daniel. Whether the two artifacts link to a single
  `MemberId` is a data-model detail for spec 008 (member/admin records).
- **Below `client_min_version` mid-life:** O4 calm screen; admin alerted; no rider action.

## Privacy notes

Invariants touched and how they are preserved:

- **I3 (phone hashed for lookup, encrypted for display):** auth uses `phone_lookup_hash`
  (constant-time); plaintext phone never logged; display is Admin-only and audit-logged.
- **I4 (device tokens scoped per device/version, invalidated on auth change):** this spec
  enforces the onboarding-layer cases (token registration + invalidation on new-device
  re-onboarding, AC4); the full enumeration of I4's "any auth change" triggers is settled
  in ADR-0016 D2 alongside the session model.
- **I5 (Admin PII reads audit-logged):** any Admin view of a member's phone during
  issuance/recovery emits an audit event. (Issuance UI is spec 008; the audit obligation
  is asserted here.)
- **I11 (Admins issued only by the Developer; WebAuthn, no signup):** enforced at the
  developer-only Admin-creation endpoint and the absence of any signup surface.
  **ADR-0015 narrows I11**: it bans self-serve/public/member-initiated access (signup
  form, request-access link) but permits the Developer to deliver a single-use, short-TTL,
  developer-minted Admin *registration* link out of band via Email Workers — carrying no
  PII beyond the opaque token and no credential material, and only initiating WebAuthn
  registration. This reconciles I11 with `docs/stack-matrix.md` ("Email Workers (admin
  invites)"). Token single-use + server-side TTL is asserted by AC16.
- **I8 (no third-party trackers):** onboarding adds none; AC13.
- **P2 (no PII in logs):** the phone number is a tainted type; onboarding code paths use
  only `redacted_summary()`; log-scrubber replay covers onboarding fixtures.
- **I1 (addresses encrypted at rest):** addresses are entered during admin issuance
  (spec 008), not on the onboarded device; noted here only to mark the boundary.

## A11y notes

- Required snapshot variants: the standard four (default, largest text, dark mode, RTL)
  for every onboarding screen on each native platform. No extra variants anticipated, but
  the device-binding/code-entry screen should additionally be snapshotted with a visible
  software-keyboard state where the platform shows one.
- VoiceOver/TalkBack: each step has a clear, ordered reading; the "auto-update enabled"
  confirmation is announced as a completed state, not a button.
- Switch Control / Switch Access: the whole flow is reachable; the primary advance
  affordance is a single large control per step.
- The graceful-degradation (`BelowMinVersion`) and `NeedsReauthHelp` screens must
  themselves meet the a11y bar.
- The **admin web** onboarding screens follow the web a11y bar (WCAG 2.2 AA, axe-core,
  keyboard-complete WebAuthn ceremony) per **AC11b**, not the native four-variant snapshot
  model — the web bar differs in kind.

## i18n notes

New catalog keys this introduces (placeholder English; sentence case; no exclamation
marks; periods only on full sentences):

| Key | English | Notes |
|---|---|---|
| `onboarding.helper.intro` | Let's set up this phone together. | Helper-facing; warm, not childish |
| `onboarding.signin.phone_prompt` | What's the phone number on file? | First-person, anti-administrative |
| `onboarding.signin.phone_not_on_file` | That number doesn't match what's on file. Try again or call {adminName}. | Voice-and-tone exemplar; no existence leak; also reused for the `PhoneNotOnFile` return banner |
| `onboarding.binding.code_prompt` | Enter the Onboarding Code from {adminName}. | "Onboarding Code" is a registered glossary noun (ADR-0016 D1) |
| `onboarding.binding.code_invalid` | That code didn't work. Ask {adminName} for a new one. | Warm recovery; also the `BindingFailed` helper-path copy |
| `onboarding.permissions.notifications_why` | This lets you know when your driver is at the door. | Explains value in human terms |
| `onboarding.permissions.notifications_declined` | We'll let {adminName} know notifications aren't on yet. | Shown only on decline; calm, no scolding (AC14) |
| `onboarding.autoupdate.step` | Turn on automatic updates. | The O3 step |
| `onboarding.autoupdate.enabled` | Automatic updates are on. | The screen AC5 asserts ("auto-update enabled") |
| `auth.signin_again` | Let's sign in again. Your phone number works. | Driver interactive re-auth entry (a Driver whose session expired reaches `PhoneEntry`); voice-and-tone exemplar |
| `auth.below_min_version` | This device needs {adminName}'s help. {adminName} has been told. | O4 / `NeedsReauthHelp` calm screen. {adminName} is a personal name — **repeat the name, never a pronoun** (admins may be any gender; target locales inflect by grammatical gender) |
| `auth.below_min_version_generic` | This device needs your group's help. They've been told. | Name-less fallback when no manifest/admin name is available (offline first launch); aligns with `docs/update-strategy.md` |
| `admin.onboarding.register_credential` | Set up your security key or passkey. | WebAuthn; admin web; desktop |
| `admin.onboarding.invite_expired` | This invitation has expired. Ask the developer for a new one. | Single-use/TTL path (`InviteExpired`) |

All keys are catalog entries with translator context; Swiss German (`gsw`) and RTL
locales are first-class; pseudo-locale must render every screen (AC12). The `Offline`
overlay reuses the `onboarding.signin.*` keys from the bundled catalog (no new keys).

## Voice and tone check

- Completion is **silent** — the flow routes straight to the primary surface, no "all
  set" screen (voice-and-tone: "no celebration of plumbing"). ✅
- "That number doesn't match what's on file. Try again or call {adminName}." — honest,
  warm, names the admin, no apology, no existence leak. ✅ (verbatim exemplar)
- "This device needs {adminName}'s help. {adminName} has been told." — calm, removes rider
  burden, repeats the name (no gendered pronoun, so it translates correctly). ✅
- "This device needs your group's help. They've been told." — name-less fallback;
  declarative, no apology. ✅
- "We'll let {adminName} know notifications aren't on yet." — honest, no scolding on a
  declined permission. ✅
- "Let's set up this phone together." — invites, doesn't instruct; fits a helper sitting
  beside the rider. ✅
- "Automatic updates are on." — declarative, no celebration. ✅
- No emoji in primary text; no exclamation marks; sentence case throughout. ✅

## Constitution principles touched

- [ ] **P1 (Accessibility is the product):** every native onboarding screen at four
      variants (AC11); admin web screens at the web bar (AC11b).
- [ ] **P2 (No PII in logs):** phone tainted type; scrubber covers onboarding (AC3).
- [ ] **P5 (Spec before code):** this spec + `/clarify` precede any implementation.
- [ ] **P7 (Native UI per platform):** SwiftUI / Compose / SvelteKit onboarding, no
      shared UI.
- [ ] **P8 (i18n not afterthought):** all strings from catalog; pseudo-locale (AC12).
- [ ] **P10 (Don't surprise the elderly user):** Maria's onboarding is done for her; no
      homework, no self-auth burden; a lone Rider never faces a sign-in form (AC15).
- [ ] **P11 (Free, open, donation-supported):** the auth mechanism must not introduce a
      paid/locked dependency (favoring an admin-issued code over a paid SMS gateway — OQ1).
- [ ] **P12 (Operability):** structured (PII-free) logging and OTel on every auth state
      transition; stable error codes for auth failures.
- [ ] **P13 (Updates are nearly invisible):** O3 auto-update step + O4 degradation + O8
      no rider-surface prompt (AC5–AC8).

## ADRs referenced

- **ADR-0014** (Server-Driven Configuration via Cloudflare KV) — the client fetches and
  signature-verifies the manifest at launch (index → per-locale, tiered fallback);
  `client_min_version` and the per-Group admin name live in the manifest.
- **ADR-0015** (Admin invitation channel) — narrows I11 to permit a developer-minted,
  single-use, short-TTL Admin registration link delivered out of band via Email Workers;
  reconciles I11 with `docs/stack-matrix.md`. Authored alongside this clarification.
- **ADR-0001** (Rust core) — auth/identity/token lifecycle logic lives in the core
  (`core::auth`), generated to Swift/Kotlin; clients hold no hand-rolled auth logic (P4).
- **ADR-0016** (Authentication & device-binding model) — settles the member auth model:
  admin-issued one-time **Onboarding Code** for device binding (D1, OQ1); **indefinite
  sessions with silent refresh**, ended only by admin-mediated events (D2, OQ2); recovery
  that is admin-mediated for Riders and self-serve (**Recovery Code**) for Drivers (D3,
  OQ3); and Admin WebAuthn allowing passkeys or hardware keys with Developer-re-invite
  recovery (D4, OQ5).

## Out of scope (explicitly)

- The Admin's member-management UI (issuing/editing members, role swaps, audit viewer) —
  that is spec `008-admin-member-management`. This spec defines only the auth artifacts
  issuance produces and the audit obligation around them.
- The matching engine and any ride/chain behavior (specs 004+).
- The Doorbell Notification implementation (spec 007); onboarding only requests the
  permission it needs.
- Apple Watch / Wear OS device pairing detail (later spec); onboarding lands the phone.
- Biometric / passcode app-lock on the member device (separate spec if desired).
- Account deletion / "forgetting" (governed by I12; separate flow).
- The admin Devices/version-distribution panel (O5, spec 012).

## Open questions

### Resolved during `/clarify` (2026-06-04)

- **Admin invitation channel (was the critical contradiction).** I11 vs.
  `stack-matrix.md` reconciled by **ADR-0015**: a developer-minted, single-use, short-TTL
  registration link via Email Workers is permitted. See AC16 and Privacy notes.
- **OQ4 — Driver self-issuance.** Resolved: all members are admin-*issued* (no
  self-issuance — glossary, I11, closed-group). Drivers self-*complete* first-launch on
  their own phone; Riders' first-launch is helper-run. The distinction (self-issuance: no;
  self-onboard-the-device: yes for Drivers) is stated in the persona/flow sections.
- **OQ6 — Critical Alerts while the entitlement is pending.** Resolved: request standard
  notification permission now; treat Critical Alerts as a later capability upgrade gated
  on the entitlement; never block onboarding (C.4, AC14).
- **OQ7 — Source of `{adminName}` at degradation.** Resolved: from the per-Group, non-PII
  signed KV manifest read at launch (ADR-0014), available pre-sign-in; a name-less
  fallback (`auth.below_min_version_generic`) covers the no-cached-manifest case
  (Section D).
- **OQ8 — Offline first-launch limits.** Resolved: device-binding is server-validated and
  cannot complete offline; `Offline` is an overlay that shows the bundled-catalog sign-in
  UI and defers the network action until connectivity (Edge cases, state machine).

### Resolved via ADR-0016 (2026-06-04)

1. **OQ1 — Device-binding mechanism.** Resolved: admin-issued one-time **Onboarding Code**
   (single-use, short-TTL, server-validated, no SMS/email — P11/I8). Glossary noun
   registered (`OnboardingCode`). See Section B, step C.3, AC17.
2. **OQ2 — Session lifetime & refresh.** Resolved: **indefinite** member sessions with
   silent server-side refresh; ended only by admin revoke/logout, new-device re-onboarding
   (I4), or account deletion (I12). Riders → `NeedsReauthHelp` (never a form); Drivers may
   re-auth. Admin (WebAuthn) sessions are separate and shorter. See AC18; backs AC15.
3. **OQ3 — Device replacement / recovery.** Resolved: Riders recover via the Admin
   re-issuing an Onboarding Code; Drivers self-serve with a **Recovery Code** (glossary;
   `RecoveryCode`), Admin fallback if lost. Old token invalidated (I4). See AC19.
4. **OQ5 — WebAuthn for Admins.** Resolved: passkeys *or* hardware keys; user verification
   required; no attestation; backup credential encouraged; lost-key recovery is a Developer
   re-invite (ADR-0015) that revokes the prior credential. See AC20.
5. **Onboarding Code glossary registration.** Done — `Onboarding Code` and `Recovery Code`
   are registered in `docs/domain-glossary.md`.

**All open questions are resolved.** The spec is ready for `/speckit.plan`.
