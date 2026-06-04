# ADR-0015: Admin Invitation Channel (narrowing I11)

- **Status:** Accepted
- **Date:** 2026-06-04
- **Author:** Boundless founder
- **Deciders:** Boundless founder

## Context

Spec `001-onboarding` defines how a new Admin (e.g. Sarah) gets provisioned: the
Developer mints a pending Admin record and the Admin then registers a WebAuthn
credential (passkey / hardware security key). To register, the Admin needs to *reach*
a registration page — the Developer has to deliver a one-time link or token out of band.

The spec proposed delivering that link via **Cloudflare Email Workers**, which
`docs/stack-matrix.md` explicitly lists as a sanctioned binding: "Email Workers (admin
invites)". But privacy-invariant **I11** says, verbatim:

> There is no signup form. There is no "request access" link. There is no email-based
> invite for Admins (only members are invitable by Admins; Admins themselves require the
> developer to provision).

So two always-loaded documents contradict each other: the stack matrix sanctions an
email admin-invite mechanism that I11 appears to forbid. A spec cannot resolve a
doc-vs-doc contradiction on its own — an architect planning the admin onboarding flow
would be guessing whether the emailed-link registration path is even permitted. The
`/clarify` gate on spec 001 surfaced this as a **critical** finding. Per
`docs/privacy-invariants.md` ("Removing or weakening an invariant requires an ADR"),
reconciling it requires an ADR.

The substantive question I11 was written to answer is: **can anyone obtain access
without the Developer's deliberate action?** The threat I11 defends against is
self-serve or socially-engineered escalation — a public signup form, a "request access"
link, or a member emailing themselves into an Admin role. A *Developer-initiated*,
single-use registration link is categorically different: it is the Developer's
deliberate action, merely transported over email.

## Decision

**Narrow I11** to distinguish *who initiates access* from *how the registration link is
transported*. I11 continues to forbid all self-serve and member-initiated paths, but
**permits the Developer to deliver a single-use Admin registration link out of band via
Email Workers**, subject to these constraints:

1. **Developer-minted only.** The link is produced exclusively by the developer-only,
   hardware-key-backed Admin-creation endpoint (unchanged from I11's existing
   enforcement). No member, Admin, or anonymous caller can mint one.
2. **Single-use.** The token is consumed on the first successful WebAuthn registration
   and cannot be replayed. A reused link is rejected and routes to `InviteExpired`.
3. **Short TTL, validated server-side.** The token expires after a short window
   (suggested 72h; final value an implementation detail), validated against server time,
   never the client clock.
4. **No PII in transit.** The email body carries only the opaque token/link — no name,
   phone, address, or other PII beyond what email delivery inherently exposes (the
   recipient address, which the Developer already holds out of band).
5. **No credential material in transit.** The link only *initiates* a WebAuthn
   registration ceremony in the browser; no passwords, secrets, or private keys ever
   travel by email. The credential is generated on the Admin's authenticator.
6. **Registration only.** The link grants the ability to *register a credential against
   an already-provisioned pending Admin record*. It does not grant a session, does not
   create the Admin record (the Developer already did that), and cannot elevate an
   existing member to Admin.

This narrowing is reflected as a clarifying note on I11 in
`docs/privacy-invariants.md`, so the two documents no longer contradict.

## Considered alternatives

### Option A (chosen) — Narrow I11; permit a Developer-minted single-use email registration link

**Pros:**
- Resolves the doc-vs-doc contradiction at the source (I11 ↔ stack-matrix).
- Matches the existing sanctioned binding in `stack-matrix.md` ("Email Workers (admin
  invites)") and the WebAuthn-provisioning design already in spec 001.
- Preserves the real security property I11 protects: access still requires deliberate
  Developer action; no self-serve or member-initiated escalation exists.
- Email is the natural out-of-band channel for a volunteer admin (Sarah) the Developer
  may never meet in person.

**Cons:**
- Weakens the literal "no email-based invite" wording — mitigated by the six constraints
  above and the single-use/TTL/no-PII/no-credential properties.
- Email is a lower-assurance transport than in-person hand-off; a compromised mailbox
  could intercept the link. Mitigated: the link only initiates WebAuthn registration, so
  an interceptor would have to *complete* registration with their own authenticator and
  the Developer would see an unexpected active Admin (and the legitimate Admin's later
  registration attempt would fail on the consumed token — a detectable signal).

### Option B — Keep I11 literal; deliver the token by a non-email out-of-band channel

The Developer conveys the one-time token by phone, in person, or via a signal/secure
channel; no email is sent. `stack-matrix.md` line 149 ("Email Workers (admin invites)")
would have to be amended/removed for consistency.

**Pros:**
- I11 stays literally intact; no invariant weakening.
- Slightly higher assurance (no email-interception surface).

**Cons:**
- Removes a sanctioned, already-listed capability and the most practical channel for
  reaching a remote volunteer admin.
- Pushes coordination cost onto the Developer for every admin onboarding.
- Still leaves the stack-matrix contradiction (must edit that doc anyway), so it does not
  actually avoid touching the always-loaded docs.

### Option C — Defer; leave the contradiction as an open question for a later auth ADR

**Pros:**
- No decision needed now.

**Cons:**
- Blocks `/speckit.plan` on spec 001: the architect cannot plan the admin onboarding flow
  without knowing whether the email channel is allowed.
- Leaves a known doc-vs-doc contradiction live in always-loaded context — exactly the
  kind of drift ADRs exist to prevent.

## Consequences

### Positive

- Spec 001's admin onboarding flow is unblocked and internally consistent with the
  always-loaded docs.
- The security intent of I11 is now stated more precisely (initiator-based, not
  transport-based), which is more robust to future questions.
- A new acceptance criterion (spec 001 AC16) makes the single-use + server-side-TTL +
  consumed-on-registration properties testable.

### Negative / costs

- A small attack surface is added (email interception of a registration link), bounded by
  the constraints above and detectable via the consumed-token signal.
- I11's wording is now qualified rather than absolute; future readers must read the
  clarifying note, not just the headline sentence.

### Neutral / follow-ups

- This ADR settles the **channel** only. WebAuthn parameter choices for Admins (resident
  keys, attestation, allowed authenticator types, lost-key recovery) remain open and are
  deferred to **ADR-0016** (the authentication & device-binding model).
- The Admin-creation endpoint authorization (developer-only, hardware-key-backed) is
  unchanged by this ADR.

## Compliance

- **Invariant change:** `docs/privacy-invariants.md` I11 gets a clarifying sub-bullet
  citing this ADR; numbering is unchanged (I11 stays I11).
- **Constitution:** no constitution principle changes (I11 lives in privacy-invariants,
  not the constitution). Closed-group/no-self-signup (P11-adjacent, glossary "Closed
  Group") is preserved. P9 (privacy invariants are testable) is satisfied: the new
  single-use/TTL enforcement is covered by spec 001 AC16 and the named core test cited in
  the I11 enforcement list.
- **Stack matrix:** `docs/stack-matrix.md` ("Email Workers (admin invites)") is now
  consistent with I11; no edit required.
- **Spec:** `specs/001-onboarding/spec.md` cites this ADR in "What changes for Sarah?",
  Detailed behavior A.1, Privacy notes (I11), and AC16.

## References

- `docs/privacy-invariants.md` — I11
- `docs/stack-matrix.md` — Cloudflare bindings ("Email Workers (admin invites)")
- `specs/001-onboarding/spec.md` — admin onboarding flow, AC16
- ADR-0014 (server-driven config) — the manifest the client also fetches at launch
- [Cloudflare Email Workers](https://developers.cloudflare.com/email-routing/email-workers/)
- [WebAuthn / Web Authentication](https://www.w3.org/TR/webauthn-2/)
- ADR-0016 (auth model) — WebAuthn parameters + device-binding mechanism deferred from this ADR
