# 008 — Admin member-management (issuance)

> Spec status: Draft (awaiting `/clarify`)
> Author: notch
> Date: 2026-06-10

## One-paragraph summary

Boundless is a **closed group**: there is no signup form. Every Rider and Driver
account is *issued* by an Admin, never self-created (I11, glossary "Closed Group").
Spec 001 built the auth model and the device-side first-launch flow that *consume* an
issued identity; it deliberately stopped short of the surface that *produces* one. This
spec builds that surface: the Admin's web member-management experience and the server
endpoints behind it. **Sarah** issues a member (Rider or Driver) by entering their name,
phone, and home address and choosing a role; the server stores that PII safely — phone
hashed for lookup and encrypted for display (I3), address encrypted at rest with a
per-Group key (I1) — and mints the single-use **Onboarding Code** a trusted helper will
type during that member's first launch (ADR-0016). Sarah can list, search, and view her
members, regenerate a lost Onboarding Code, edit a member's details, and read a
first-class audit log of every time member PII was viewed (I5). This is the spec where
the address-at-rest encryption (I1) and the admin-PII-read audit trail (I5) first become
real, and it is the precondition for everything downstream: until members exist, there
is nothing to onboard, match, or notify.

## User story

- As **Sarah (Admin)**, I want to add a new member by entering their name, phone,
  address, and role and get back an Onboarding Code I can hand to whoever sets up their
  phone, so that a Rider or Driver can join my group without ever filling in a form
  themselves.
- As **Sarah (Admin)**, I want to find an existing member, see their details, fix a
  wrong address or phone number, and re-issue a lost Onboarding Code, so that I can keep
  the membership roll correct over time.
- As **Sarah (Admin)**, I want to see exactly when member PII was read and by whom, so
  that the privacy promise I make to my group is something I can actually verify.

## What changes for Maria? (rider primary persona)

Nothing changes inside Maria's app, and she never sees this surface — it is web,
admin-only. What changes is that Maria's account *exists at all* because Sarah created
it here: Sarah typed Maria's name, the phone number that is "on file" (the one Maria's
sign-in checks against, I3), and Maria's home address, and chose **Rider**. The
Onboarding Code Maria's daughter types during setup (spec 001) is the one minted on this
screen. Maria's address — the most sensitive thing the product holds — is encrypted the
moment Sarah saves it and is never shown again except through an explicit,
audit-logged read (I5); it is dropped from memory entirely once matching computes her
pickup (P3/I2, a later spec). **What Maria must never experience as a result of this
spec:** a wrong phone number that locks her out (so the "that number doesn't match"
sign-in copy from spec 001 must have a path back to Sarah), or any sense that she had
to do paperwork.

## What changes for Daniel? (driver primary persona)

Daniel's identity is issued here too — Sarah chooses **Driver** when she adds him. He
still self-onboards on his own phone (spec 001), but the phone number he signs in with
and the Onboarding Code (or Recovery Code path) trace back to this screen. Daniel's
home address is captured for matching but is subject to the same I1 encryption and is
never shown to other members.

## What changes for Sarah? (admin primary persona)

This is **Sarah's** spec. It is the first substantial admin surface beyond onboarding
herself. Per her persona: desktop-first web, responsive to tablet; heavy use of a member
**table** with search and filters; keyboard-shortcut-friendly; and an **audit log that is
first-class, not a buried setting**. She can:

- **Add a member** — a form (name, phone, address, role) → server validates and stores →
  returns a printable/legible **Onboarding Code** with its expiry.
- **Browse members** — a searchable, filterable table of the group's members showing
  non-sensitive summary fields (name, role, onboarding status, version-straggler hints
  are a *different* spec, O5/012).
- **View a member** — opens full detail including decrypted address/phone; this read is
  **audit-logged** (I5).
- **Edit a member** — correct address, phone, or role; re-encrypts on save.
- **Regenerate an Onboarding Code** — when a code is lost or expired, mint a fresh one;
  the prior live code is invalidated atomically (supersede-then-insert).
- **Read the audit log** — who viewed which member's PII, when, which fields.

Sarah **cannot** create other Admins (only the Developer can, I11) — that affordance does
not appear anywhere on this surface.

## What changes for edge personas?

- **Margaret (sometimes-attends Rider):** issued like any Rider. Her "remote-only / join
  from home" mode (personas) — if it is configured anywhere, it is plausibly an
  Admin-set member attribute. **Open question:** is remote-only mode in this spec's scope
  or a later one?
- **Tobias (variable-shift Driver):** issued like any Driver; nothing special at
  issuance.
- **Role swaps** (Margaret drives this Sunday; glossary references `ADR-006-role-swaps`):
  Sarah's persona explicitly lists "performs role swaps." **Open question:** is the
  role-swap workflow part of this spec or a sibling spec? (See Open Questions.)

## Detailed behavior

> Draft — the precise screen flow and field set are subject to `/clarify`. This section
> captures the intended shape, not the final contract.

### Add a member

1. Sarah opens "Members" and chooses **Add a member**.
2. A form collects: **name**, **phone** (entered in a human form; the server normalizes
   to E.164 before hashing, single-source `normalize_phone` from spec 001), **address**
   (street address), and **role** (Rider or Driver).
3. On save, the server (in the Rust core, P4):
   - normalizes + validates the phone; computes `phone_lookup_hash` (HMAC, I3) and
     `phone_encrypted`;
   - encrypts the address with the per-Group key (I1, dryoc sealed-box/secretbox);
   - writes the member row (RLS-scoped to this install's Group), `created_by` = Sarah
     (audit, I5);
   - mints an **Onboarding Code**: generated server-side, single-use, TTL-bounded,
     rate-limited, regenerable, carrying no PII (ADR-0016) — stored as a hash at rest
     (the T03 `onboarding_code_hash` primitive), shown to Sarah exactly once in plaintext.
4. Sarah is shown the new member and the Onboarding Code with its expiry, in a form she
   can read aloud or print for the helper.

### Browse / search / view / edit

- The member table shows summary fields only (no address; no full phone) and never logs
  PII (P2).
- Viewing a member's full detail decrypts address/phone for display and emits an audit
  event (timestamp, admin id, member id, fields returned, request id — I5).
- Editing re-validates and re-encrypts; a phone change recomputes the lookup hash.

### Regenerate an Onboarding Code

- From a member's detail, **Regenerate code** mints a fresh code and invalidates any
  prior live code for that member in the same transaction (the T06 partial-unique-index
  "one live code per member" contract; supersede-then-insert atomically).

## States and transitions

> Draft. To be enumerated precisely in `/clarify`/`/plan`. Member lifecycle (candidate):

- **Issued, not yet onboarded** — account exists, a live Onboarding Code is outstanding,
  no device bound.
- **Onboarded** — a device is bound (spec 001 `bind-device` succeeded); the Onboarding
  Code is consumed.
- **Code expired / lost** — no live code; Sarah must regenerate.
- **Needs re-onboarding** — device replaced/revoked (spec 001 I4 path).
- **(Open) Deactivated / forgotten** — see I12 / Open Questions on deletion scope.

## Acceptance criteria (testable)

> Draft set — will be refined and made fully testable during `/clarify`/`/plan`.

- [ ] AC1: An Admin can create a Rider and a Driver by submitting name + phone + address
      + role; the member is persisted RLS-scoped to the install's Group with `created_by`
      set to the acting Admin.
- [ ] AC2: A created member's **address is encrypted at rest** — the stored column is
      `bytea`/ciphertext, decryptable only with the per-Group key; ciphertext ≠ plaintext.
      *(This is where I1's `i1_addresses_encrypted` test lands.)*
- [ ] AC3: A created member's **phone is stored two-fold** — `phone_lookup_hash` (HMAC,
      constant-time compare) and `phone_encrypted`; the plaintext phone is never logged
      and never persisted in clear (I3, P2).
- [ ] AC4: Creating a member **mints exactly one live Onboarding Code**, stored hashed,
      single-use, TTL-bounded, rate-limited, carrying no PII; the plaintext is returned to
      the Admin exactly once (ADR-0016).
- [ ] AC5: **Regenerating** an Onboarding Code invalidates the prior live code atomically
      (supersede-then-insert); at most one live code per member exists at any time.
- [ ] AC6: Every server response that returns member **PII to an Admin emits an audit
      event** with timestamp, admin id, member id, fields returned, request id; the audit
      handler obligation is compile-enforced for PII-returning endpoints (I5,
      `#[require_audit]`).
- [ ] AC7: The Admin can **read the audit log** for the group; the audit-log view itself
      records that it was viewed (I5).
- [ ] AC8: The member **table and summary views never expose** another member's address
      or full phone, and never write PII to logs (P2/I6 spirit on the admin side).
- [ ] AC9: **No admin-creation affordance exists** on this surface — an Admin cannot
      create or invite another Admin (I11); the only Admin-issuance path remains the
      Developer's.
- [ ] AC10: Editing a member re-validates and **re-encrypts** changed PII; a phone change
      recomputes `phone_lookup_hash` such that the member's next sign-in matches.
- [ ] AC11 (a11y): Every admin screen passes the a11y bar — WCAG 2.2 AA, keyboard-complete
      (incl. the table, the add/edit dialogs, and any menus), visible focus, zero axe
      violations, 400% zoom without horizontal scroll, dark + RTL.
- [ ] AC12 (i18n): Every user-visible string ships from the catalog; pseudo-locale renders
      without truncation; no hardcoded strings (P8).
- [ ] AC13 (cross-tenant): With **two Groups** present, an Admin scoped to Group A cannot
      read, list, or edit any member of Group B — proven against the real RLS policy on
      the deployed path. *(This finally enables the sec-audit F5 live cross-tenant
      isolation proof that needs ≥2 seeded Groups.)*

## Edge cases

- **Duplicate phone** — Sarah adds a member whose phone already exists in the group.
  (Reject? Surface the existing member? — clarify.)
- **Invalid / unparseable phone or address** — calm, non-administrative validation copy
  with a path forward (voice-and-tone error register).
- **Regenerating a code that was already consumed** vs. one merely expired vs. one still
  live — all three converge to "a fresh single live code," prior invalidated.
- **Two Admins editing the same member concurrently** — last-write-wins? optimistic
  concurrency? (Only relevant if an install has >1 Admin — clarify.)
- **Removing/deactivating a member who is in tonight's chain** — interaction with
  matching (later spec) — clarify deletion scope first.
- **Network down mid-issuance** — the member must not be half-created (atomic write);
  calm retry copy.
- **Admin's clock is wrong** — code TTL and audit timestamps are **server**-time, never
  device time (spec 001 carry-forward).
- **Dynamic Type / large text on tablet; keyboard-only; screen reader** — the admin
  surface is desktop-first but must still meet the a11y bar.

## Privacy notes

This spec is privacy-load-bearing — it is where several invariants first acquire enforcing
code:

- **I1 — Addresses encrypted at rest.** First real implementation: per-Group key
  (dryoc sealed-box/secretbox), the Group key wrapped by the KEK in Cloudflare Secrets
  Store. Adds the `i1_addresses_encrypted` test. **Open question:** when/where is the
  per-Group key generated, and does that warrant a new ADR (key generation at group
  bootstrap, KEK access, rotation)?
- **I3 — Phone hashed for lookup, encrypted for display.** Issuance is the write side of
  the columns spec 001 reads.
- **I5 — Admin reads of PII are audit-logged.** First real `audit_log` table +
  `#[require_audit]` enforcement on PII-returning handlers.
- **I11 — Admins issued only by the Developer.** This surface must not violate it (no
  admin-creation path here).
- **I12 — Forgetting is a feature.** Member deletion / `forget_member`. **Open question:**
  is deletion in scope here, or a sibling `core::deletion` spec? (Spec 001 deferred it.)
- **P2 — No PII in logs.** Address/phone/name handling on the hottest PII path in the
  product; the scrubbed `emit()` log sink and tainted-type discipline apply throughout.

## A11y notes

- Beyond the default four snapshot variants, the admin web a11y story is axe-core +
  keyboard + reflow (per spec 001 T15's pattern), now extended to **tables, dialogs, and
  menus** — which is exactly why the stack matrix parked `melt-ui`/Radix Svelte,
  TanStack Table, and TanStack Query for "spec 008."
- Audit-log and validation states need `aria-live` regions.
- Switch Access / keyboard-only operation of the add/edit dialogs is required.

## i18n notes

New catalog keys (draft — placeholder English, voice-and-tone-checked; admin register is
functional but still warm and non-administrative):

| Key | English | Notes |
|---|---|---|
| `admin.members.title` | Members | Nav/heading |
| `admin.members.add` | Add a member | Sentence case, no period (label) |
| `admin.members.search` | Search members | Placeholder |
| `admin.member.name` | Name | Field label |
| `admin.member.phone` | Phone | Field label |
| `admin.member.address` | Address | Field label |
| `admin.member.role` | Role | Field label |
| `admin.member.role_rider` | Rider | Glossary term — catalog key, not phrase |
| `admin.member.role_driver` | Driver | Glossary term |
| `admin.member.save` | Save | Action |
| `admin.member.onboarding_code` | Onboarding code | The minted secret's label |
| `admin.member.code_expires` | Code expires {when} | ICU; "around"-style time per voice |
| `admin.member.regenerate_code` | Regenerate code | Action |
| `admin.members.audit_log` | Audit log | First-class nav, not buried |
| `admin.member.phone_invalid` | That number doesn't look right. Check and try again. | Error, voice register |
| `admin.member.duplicate_phone` | That number is already in your group. | Error (pending edge decision) |

## Voice and tone check

The admin surface is more utilitarian than Maria's, but the same rules apply: sentence
case, no exclamation marks, anti-administrative, errors honest with a path forward. "Add
a member," not "Create new member record." "That number doesn't look right. Check and try
again," not "Invalid input (422)." Every new string above is drafted to pass
`docs/voice-and-tone.md`; the `i18n-validator` and `reviewer` subagents will confirm.

## Constitution principles touched

- [ ] P1 — Accessibility: admin web meets WCAG 2.2 AA, keyboard-complete, axe-clean
      (incl. tables/dialogs/menus).
- [ ] P2 — No PII in logs: the product's hottest PII write path; address/phone/name never
      logged; tainted types + scrubbed `emit()`.
- [ ] P4 — Rust core is source of truth: issuance validation, phone normalization/hash,
      address encryption, and code minting live in the core, not in SvelteKit.
- [ ] P5 — Spec before code: this spec + `/clarify` precede any implementation.
- [ ] P7 — Native UI: admin = SvelteKit; a11y parity required.
- [ ] P8 — i18n: all strings from the catalog; pseudo-locale clean.
- [ ] P9 — Privacy invariants testable: adds `i1_addresses_encrypted` and the I5
      audit-trail tests.
- [ ] P11 — Free/open: no new paid dependency; melt-ui/Radix/TanStack are OSS.
- [ ] P12 — Operability: stable error codes for issuance failures; audit log; runbook
      touch-points for key management.

## ADRs referenced

- **ADR-0016** — Onboarding/Recovery codes (the code this spec mints).
- **ADR-0014** — Server-driven config / signed manifest (admin name in copy; design
  tokens).
- **ADR-0018 / ADR-0019 / ADR-0021 / ADR-0024** — HMAC, tokio-postgres-over-Hyperdrive,
  access-token model, the pooler-safe query family (the server substrate this builds on).
- **`ADR-006-role-swaps`** — referenced by the glossary; relevant iff role swaps are
  in-scope (Open Question).
- **(likely new) ADR — per-Group encryption key management** — generation at group
  bootstrap, KEK in Secrets Store, rotation policy. To be decided during `/clarify`/`/plan`.

## Out of scope (explicitly)

- The **admin device-versions panel** (O5) — its own spec (suggested 012).
- **Live ride state / matching** — Sarah does not see live rides; matching is a later
  spec (004+). Issuance stores the address; *using* it for a chain is out of scope.
- **The rider/driver/admin device UI** — built in spec 001; not re-touched here.
- **Geocoding the address into coordinates for matching** — *(Open Question: does
  issuance geocode now, or does matching geocode later?)* — provisionally out of scope.
- **Bulk import / CSV** of members — provisionally out of scope.
- **The signed-manifest serving/signing pipeline** — server/infra, separate.

## Open questions

1. **Role swaps** — is Sarah's "performs role swaps" workflow part of this spec, or a
   sibling spec (with `ADR-006-role-swaps`)?
2. **Member deletion (I12 / `forget_member`)** — in scope here, or the separate
   `core::deletion` spec that 001 deferred?
3. **Per-Group encryption key lifecycle** — where/when is the key generated (group
   bootstrap?), how is the KEK accessed from Secrets Store, what is the rotation policy —
   and does this need its own ADR?
4. **Group bootstrap** — does this spec create the Group row + its key (single-install
   bootstrap), or assume the Group already exists and only issue members into it?
5. **Geocoding** — does issuance compute/store an (encrypted) coordinate for matching, or
   is that matching's job? (Affects PostGIS scope here.)
6. **Multi-Admin concurrency** — can an install have more than one Admin, and if so what
   is the concurrent-edit policy?
7. **Remote-only / "join from home" mode** (Margaret) — an issuance-time member attribute
   here, or later?
8. **Duplicate-phone policy** — reject, or surface-and-link the existing member?
9. **Device-token at-rest encryption (`PgDeviceStore`)** — several spec-001 deferrals are
   tagged "spec 007 (push) **or** spec 008 (issuance), whichever brings device-token
   encryption." Does the encryption primitive land here or in the push spec?
