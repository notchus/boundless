# 008 — Admin member-management (issuance)

> Spec status: Clarified (ready for `/speckit.plan`) — `/clarify` pass 2026-06-10
> Author: notch
> Date: 2026-06-10

## One-paragraph summary

Boundless is a **closed group**: there is no signup form. Every Rider and Driver
account is *issued* by an Admin, never self-created (I11, glossary "Closed Group").
Spec 001 built the auth model and the device-side first-launch flow that *consume* an
issued identity; it deliberately stopped short of the surface that *produces* one. This
spec builds that surface: the Admin's web member-management experience and the server
endpoints behind it. **Sarah** issues a member (Rider or Driver) by entering their name,
phone, and home address and choosing one or more roles; the server stores that PII safely
— phone hashed for lookup and encrypted for display (I3), name and address encrypted at
rest with a per-Group **secretbox** key (I1, ADR-0025) — and mints the single-use
**Onboarding Code** a trusted helper will type during that member's first launch
(ADR-0016). Sarah can list, search, and view her members, regenerate a lost Onboarding
Code, edit a member's details, and read a first-class audit log of every time member PII
was viewed (I5). This is the spec where the **per-Group encryption key** is first
generated (at Group bootstrap), where **address/name-at-rest encryption** (I1) and the
**admin-PII-read audit trail** (I5) first become real, and it is the precondition for
everything downstream: until members exist, there is nothing to onboard, match, or notify.

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
screen. Maria's name and address — the most sensitive things the product holds — are
encrypted the moment Sarah saves them and are never shown again except through an
explicit, audit-logged detail read (I5); the address is dropped from memory entirely once
matching computes her pickup (P3/I2, a later spec). **What Maria must never experience as
a result of this spec:** a wrong phone number that locks her out (so spec 001's "that
number doesn't match" sign-in copy keeps its path back to Sarah), or any sense that she
had to do paperwork.

## What changes for Daniel? (driver primary persona)

Daniel's identity is issued here too — Sarah chooses **Driver** when she adds him. He
still self-onboards on his own phone (spec 001), but the phone number he signs in with
and the Onboarding Code (or Recovery Code path) trace back to this screen. Daniel's
home address is captured for matching but is subject to the same I1 encryption and is
never shown to other members. (A member may hold more than one role — Daniel can also be
a Rider in another context; the role *set* is established at issuance, ADR-0006.)

## What changes for Sarah? (admin primary persona)

This is **Sarah's** spec. It is the first substantial admin surface beyond onboarding
herself. Per her persona: desktop-first web, responsive to tablet; heavy use of a member
list with search and filters; keyboard-shortcut-friendly; and an **audit log that is
first-class, not a buried setting**. She can:

- **Add a member** — a form (name, phone, address, role(s)) → server validates and stores
  → returns a legible/printable **Onboarding Code** with its expiry.
- **Browse members** — a searchable, filterable member list showing non-sensitive summary
  fields (name, role, onboarding status). The summary carries no tainted PII type and is
  not an audited read.
- **View a member** — opens full detail including decrypted address/phone; this read is
  **audit-logged** (I5).
- **Edit a member** — correct address, phone, name, or role set; re-encrypts on save.
- **Regenerate an Onboarding Code** — when a code is lost or expired, mint a fresh one;
  the prior live code is invalidated atomically (supersede-then-insert).
- **Read the audit log** — who viewed which member's PII, when, which fields.

Sarah **cannot** create other Admins (only the Developer can, I11) — that affordance does
not appear anywhere on this surface. She also cannot *delete/forget* a member here
(account deletion is the separate `core::deletion` path, I12 — out of scope; see below).

## What changes for edge personas?

- **Margaret (sometimes-attends Rider):** issued like any Rider. Her "remote-only / join
  from home" mode is **deferred to the matching/gathering spec** that actually consumes it
  (issuance needs no such attribute until matching does) — see Out of scope.
- **Tobias (variable-shift Driver):** issued like any Driver; nothing special at
  issuance.
- **Role swaps** (Margaret drives this Sunday): a member's role *set* (`roles[]`) is
  established at issuance and editable here, but the *role-swap workflow* itself (an Admin
  re-granting roles per Gathering) is **deferred to a sibling spec** (ADR-0006 records that
  role swaps are allowed; the data model already supports a multi-role member).

## Detailed behavior

> The precise field set and screen flow are pinned at `/speckit.plan`; this captures the
> intended shape.

### Group bootstrap (once per install)

Boundless is single-tenant: one install = one Group (glossary). Before any member can be
issued, the Group must exist **and have a per-Group encryption key**. This spec owns that
bootstrap: it creates the single `groups` row and generates the Group's per-Group
**secretbox** key, wrapped (encrypted) by the **KEK** in Cloudflare Secrets Store and
stored in `delegated_keys` (ADR-0025). The plaintext key never touches durable storage or
logs (P2). Without this step, address/name encryption (I1) has no key.

### Add a member

1. Sarah opens "Members" and chooses **Add a member**.
2. A form collects: **name**, **phone** (entered in a human form; the server normalizes
   to E.164 before hashing — single-source `normalize_phone`, spec 001), **address**
   (street address), and **role(s)** (Rider and/or Driver).
3. On save, the server (in the Rust core, P4):
   - normalizes + validates the phone; computes `phone_lookup_hash` (HMAC, I3) and
     `phone_encrypted`;
   - encrypts the **address** and the **name** with the per-Group secretbox key (I1,
     ADR-0025);
   - writes the member row (RLS-scoped to this install's Group), `roles[]` set, and
     `created_by` = the acting Admin (audit, I5);
   - mints an **Onboarding Code**: generated server-side, single-use, TTL-bounded,
     rate-limited, carrying no PII (ADR-0016) — stored as a hash at rest (the T03
     `onboarding_code_hash` primitive), shown to Sarah exactly once in plaintext.
4. Sarah is shown the new member and the Onboarding Code with its expiry, in a form she
   can read aloud or print for the helper.

### Browse / search / view / edit

- The member list returns a **`MemberSummary`** projection that contains **no tainted PII
  type** (no `Address`, no `PhoneNumber`): name (decrypted to a plain display `String`),
  role(s), and onboarding status. Listing is therefore *not* an audited read (the
  P2-sensitive unit is the name+address *pair*, not name alone) and never logs PII.
- Viewing a member's full **`MemberDetail`** decrypts address/phone for display and emits
  an audit event (timestamp, admin id, member id, fields returned, request id — I5). Any
  endpoint whose return type contains a tainted PII type carries `#[require_audit]`
  (compile-enforced).
- Editing re-validates and re-encrypts changed PII; a phone change recomputes the lookup
  hash so the member's next sign-in matches. Concurrent edits use optimistic concurrency
  on the existing `updated_at` (a stale write is rejected with calm copy).

### Regenerate an Onboarding Code

- From a member's detail, **Regenerate code** mints a fresh code and invalidates any
  prior live code for that member in the same transaction (the T06 partial-unique-index
  "one live code per member" contract; supersede-then-insert atomically).

### Duplicate phone

- The schema already enforces one phone per group (`members_group_phone_lookup_key`
  unique on `(group_id, phone_lookup_hash)`). When Sarah enters a phone already enrolled,
  the server **surfaces and links the existing member** rather than silently failing —
  which is itself an I5-audited PII read. Existence disclosure here is *intended* (Sarah
  manages the roll) and is an admin-surface-only behavior; it must never be reused on a
  member-facing endpoint (spec 001's no-existence-leak discipline holds there).

## States and transitions

Member lifecycle (deletion/forgetting is out of scope — I12 lives in `core::deletion`):

- **Issued, not yet onboarded** — account exists, a live Onboarding Code is outstanding,
  no device bound.
- **Onboarded** — a device is bound (spec 001 `bind-device` succeeded); the Onboarding
  Code is consumed.
- **Code expired / lost** — no live code; Sarah must regenerate.
- **Needs re-onboarding** — device replaced/revoked (spec 001 I4 path).

## Acceptance criteria (testable)

- [ ] AC1: An Admin can create a Rider and a Driver by submitting name + phone + address
      + role(s); the member is persisted RLS-scoped to the install's Group with `roles[]`
      set and `created_by` = the acting Admin.
- [ ] AC2: A created member's **address is encrypted at rest** with the per-Group
      secretbox key — the stored column is `bytea`/ciphertext (ciphertext ≠ plaintext),
      and the round-trip requires the **unwrapped per-Group key** (`Address::from_db(bytes,
      &GroupKey)` shape, I1). Enforced by the named test `i1_addresses_encrypted`.
- [ ] AC3: A created member's **name is encrypted at rest** (`name_encrypted bytea`,
      secretbox), and is exposed only as a plain `String` inside the non-tainted
      `MemberSummary`/`MemberDetail` projections — never persisted in clear.
- [ ] AC4: A created member's **phone is stored two-fold** — `phone_lookup_hash` (HMAC,
      constant-time compare) and `phone_encrypted`; the plaintext phone is never logged
      and never persisted in clear (I3, P2).
- [ ] AC5: Creating a member **mints exactly one live Onboarding Code**, stored hashed,
      single-use, TTL-bounded, rate-limited, carrying no PII; the plaintext is returned to
      the Admin exactly once (ADR-0016).
- [ ] AC6: **Regenerating** an Onboarding Code invalidates the prior live code atomically
      (supersede-then-insert); at most one live code per member exists at any time.
- [ ] AC7: Every server response that returns member **PII to an Admin emits an audit
      event** with **timestamp, admin id, member id, fields returned, request id**; the
      `#[require_audit]` obligation is **compile-enforced** for any handler whose return
      type contains a tainted PII type, and an **integration test asserts every PII handler
      in the OpenAPI spec has a matching audit-log entry** (I5).
- [ ] AC8: The member-list **`MemberSummary` projection contains no tainted PII type** (a
      compile/type assertion), so listing exposes no address/full-phone and is not an
      audited read; no summary or list path writes PII to logs (P2).
- [ ] AC9: The Admin can **read the audit log** for the group; the log stores **field
      names, not PII values**, so reading it is not itself a recursive PII read.
- [ ] AC10: **No admin-creation affordance exists** on this surface — an Admin cannot
      create or invite another Admin (I11); the only Admin-issuance path remains the
      Developer's.
- [ ] AC11: Editing a member re-validates and **re-encrypts** changed PII; a phone change
      recomputes `phone_lookup_hash` such that the member's next sign-in matches; a stale
      concurrent edit (older `updated_at`) is rejected.
- [ ] AC12: **Group bootstrap** creates the single `groups` row and generates the
      per-Group secretbox key wrapped by the KEK and stored in `delegated_keys`
      (ADR-0025); the plaintext key never appears in durable storage or logs, and member
      issuance fails closed if no Group key exists.
- [ ] AC13: A member's **role set (`roles[]`)** is established at issuance (Rider, Driver,
      or both) and editable; the role-swap *workflow* is out of scope (ADR-0006).
- [ ] AC14 (a11y): Every admin screen passes the a11y bar — WCAG 2.2 AA, keyboard-complete
      (incl. the member list, the add/edit dialogs, and any menus), visible focus, zero axe
      violations, 400% zoom without horizontal scroll, dark + RTL.
- [ ] AC15 (i18n): Every user-visible string ships from the catalog; pseudo-locale renders
      without truncation; no hardcoded strings (P8).
- [ ] AC16 (cross-tenant, deployed-edge): With **two issued Groups** present, an Admin
      scoped to Group A cannot **list, read, or edit** any member of Group B — proven on
      the **deployed edge as the real, locked-down `boundless_app` role** (non-superuser /
      non-`BYPASSRLS` / non-`REPLICATION`; `ensure_least_privilege`), against the real RLS
      policy. *(This is the live cross-tenant isolation proof sec-audit F5 has been waiting
      on — it needs the ≥2 seeded Groups this spec produces.)*

## Edge cases

- **Duplicate phone** — surface-and-link the existing member (an I5-audited read); never a
  silent failure, never reused on a member-facing endpoint.
- **Invalid / unparseable phone or address** — calm, non-administrative validation copy
  with a path forward (voice-and-tone error register).
- **Regenerating a code that was already consumed** vs. one merely expired vs. one still
  live — all three converge to "a fresh single live code," prior invalidated.
- **Two Admins editing the same member concurrently** — optimistic concurrency on
  `updated_at`; the stale write is rejected with calm copy (an install may hold ≥1
  Developer-provisioned Admin).
- **Network down mid-issuance** — the member must not be half-created (atomic write +
  code mint); calm retry copy.
- **Group key missing at issuance** — issuance fails closed (AC12); never store an
  unencrypted address as a fallback.
- **Admin's clock is wrong** — code TTL and audit timestamps are **server**-time, never
  device time (spec 001 carry-forward).
- **Dynamic Type / large text on tablet; keyboard-only; screen reader** — the admin
  surface is desktop-first but must still meet the a11y bar.

## Privacy notes

This spec is privacy-load-bearing — several invariants first acquire enforcing code here:

- **I1 — Addresses (and names) encrypted at rest.** First real implementation: a per-Group
  **secretbox** symmetric key (dryoc), the Group key wrapped by the KEK in Cloudflare
  Secrets Store, generated at Group bootstrap and stored in `delegated_keys` (**ADR-0025**,
  authored with this spec). Adds the `i1_addresses_encrypted` test (AC2). `name_encrypted`
  rides the same key (AC3).
- **I3 — Phone hashed for lookup, encrypted for display.** Issuance is the write side of
  the columns spec 001 reads (AC4).
- **I5 — Admin reads of PII are audit-logged.** First real `audit_log` table (new
  migration) + `#[require_audit]` compile enforcement + the OpenAPI-PII-handler-coverage
  test (AC7). The audit log stores field *names*, not values (AC9).
- **I11 — Admins issued only by the Developer.** This surface has no admin-creation path
  (AC10).
- **I12 — Forgetting is a feature.** Member deletion / `forget_member` is **out of scope**
  here — it is the separate `core::deletion` spec that 001 deferred. (A soft "deactivate"
  state is *not* introduced in v1.)
- **P2 — No PII in logs.** The product's hottest PII write path; name/address/phone and the
  per-Group key all routed through tainted-type discipline and the scrubbed `emit()` sink.
- **P3/I2 — Plaintext addresses only during matching.** Issuance persists the ciphertext;
  *decrypting into a `MatchingContext`* and dropping it is matching's job (later spec).

## A11y notes

- Beyond the default four snapshot variants, the admin web a11y story is axe-core +
  keyboard + reflow (spec 001 T15's pattern), now extended to **lists, dialogs, and
  menus** — which is why the stack matrix parked accessible primitives (`melt-ui` /
  Radix Svelte) + TanStack Table/Query for "spec 008." The exact primitive library and
  versions are a `/speckit.plan` decision pinned from `pnpm-lock.yaml` (never invented).
- Audit-log and validation states need `aria-live` regions.
- Switch Access / keyboard-only operation of the add/edit dialogs is required.

## i18n notes

New catalog keys (placeholder English; sentence case per voice-and-tone — note the
glossary term "Onboarding Code" is title-cased only in docs/code, never in UI copy):

| Key | English | Notes |
|---|---|---|
| `admin.members.title` | Members | Nav/heading |
| `admin.members.add` | Add a member | Label, no period |
| `admin.members.search` | Search members | Placeholder |
| `admin.member.name` | Name | Field label |
| `admin.member.phone` | Phone | Field label |
| `admin.member.address` | Address | Field label |
| `admin.member.role` | Role | Field label |
| `admin.member.role_rider` | Rider | Glossary term — catalog key, not phrase |
| `admin.member.role_driver` | Driver | Glossary term |
| `admin.member.save` | Save | Action |
| `admin.member.onboarding_code` | Onboarding code | Sentence case in UI |
| `admin.member.code_expires` | Code expires {when} | ICU; admin surface may show a precise time (Sarah is tech-comfortable, unlike Maria) |
| `admin.member.regenerate_code` | Regenerate code | Action |
| `admin.members.audit_log` | Audit log | First-class nav, not buried |
| `admin.member.phone_invalid` | That number doesn't look right. Check and try again. | Error, voice register |
| `admin.member.duplicate_phone` | That number is already in your group. | Admin-surface only; surfaces the existing member |
| `admin.member.edit_stale` | Someone else just changed this member. Refresh and try again. | Optimistic-concurrency reject |

> Note: "member list" is a UI data-grid; it is never *labeled* "table" in copy (the
> glossary's "table" ban is the domain term, satisfied — no catalog string contains it).

## Voice and tone check

The admin surface is more utilitarian than Maria's, but the same rules apply: sentence
case, no exclamation marks, anti-administrative, errors honest with a path forward. "Add
a member," not "Create new member record." "That number doesn't look right. Check and try
again," not "Invalid input (422)." Every new string above is drafted to pass
`docs/voice-and-tone.md`; the `i18n-validator` and `reviewer` subagents will confirm.

## Constitution principles touched

- [ ] P1 — Accessibility: admin web meets WCAG 2.2 AA, keyboard-complete, axe-clean
      (incl. lists/dialogs/menus).
- [ ] P2 — No PII in logs: the product's hottest PII write path; name/address/phone and
      the per-Group key never logged; tainted types + scrubbed `emit()`.
- [ ] P4 — Rust core is source of truth: issuance validation, phone normalization/hash,
      name/address encryption, key generation, and code minting live in the core.
- [ ] P5 — Spec before code: this spec + `/clarify` precede implementation.
- [ ] P7 — Native UI: admin = SvelteKit; a11y parity required.
- [ ] P8 — i18n: all strings from the catalog; pseudo-locale clean.
- [ ] P9 — Privacy invariants testable: adds `i1_addresses_encrypted` and the I5
      audit-trail tests.
- [ ] P11 — Free/open: no new paid dependency (melt-ui/Radix/TanStack are OSS).
- [ ] P12 — Operability: stable error codes for issuance failures (`docs/error-codes.md`);
      a **key-management runbook** (`docs/runbooks/`) for per-Group key generation / KEK
      access / rotation (tied to ADR-0025).
- [ ] P13/O-model — N/A to issuance directly, but the manifest-provided admin name (O2)
      is the copy substrate.

## ADRs referenced

- **ADR-0006** — Role swaps allowed (a member's `roles` is a set) — **authored with this
  spec**.
- **ADR-0025** — Per-Group encryption key lifecycle (secretbox key, KEK-wrapped in Secrets
  Store, generated at Group bootstrap, rotation policy) — **authored with this spec**.
- **ADR-0016** — Onboarding/Recovery codes (the code this spec mints).
- **ADR-0014** — Server-driven config / signed manifest (admin name in copy; design
  tokens).
- **ADR-0017** — Admin auth (the session this surface runs behind).
- **ADR-0018 / ADR-0019 / ADR-0021 / ADR-0024** — HMAC, tokio-postgres-over-Hyperdrive,
  access-token model, the pooler-safe query family (the server substrate this builds on).
- **ADR-0008** (stub) — ETA matrix / geocoding on admin update: **not** implemented here;
  geocoding is deferred to the matching spec (see Out of scope).

## Out of scope (explicitly)

- **Geocoding the address into coordinates / the ETA-matrix Workflow** — deferred to the
  matching spec that consumes it (architecture flow D's geocode trigger is owned by that
  spec, not issuance; the doc is amended to say so). Issuance persists the encrypted
  address only.
- **Member deletion / forgetting (I12 / `forget_member`)** — the separate `core::deletion`
  spec (deferred by 001). No soft-deactivate in v1.
- **Device-token at-rest encryption (`PgDeviceStore`)** — deferred to the push spec (007).
  (It depends on this spec's per-Group key primitive, which now exists — 007 can build on
  it.)
- **The role-swap workflow** (per-Gathering role re-grant) — a sibling spec (ADR-0006
  records the policy; this spec only sets the initial role set).
- **Remote-only / "join from home" mode** (Margaret) — the matching/gathering spec.
- **The admin device-versions panel** (O5) — its own spec (suggested 012).
- **The phone-list export** (O7) — a separate spec; it will reuse this spec's audit (I5)
  and decryption path.
- **Live ride state / matching / chain computation** — later spec (004+).
- **Bulk import / CSV** of members.

## Resolved at `/clarify` (2026-06-10)

The draft's nine open questions were resolved (clarifier pass + author decisions):

1. **Role swaps** → initial `roles[]` at issuance only; swap workflow deferred; **ADR-0006
   authored**; glossary dangling `ADR-006-role-swaps` ref fixed.
2. **Member deletion (I12)** → out of scope (the `core::deletion` spec); no soft-deactivate.
3. **Per-Group key lifecycle** → **ADR-0025 authored** (secretbox key, KEK-wrapped, Secrets
   Store, generated at bootstrap, rotation = runbook-driven).
4. **Group bootstrap** → in scope: this spec creates the Group row + key (AC12).
5. **Geocoding** → deferred to the matching spec; architecture flow D amended.
6. **Multi-Admin concurrency** → ≥1 Admin allowed; optimistic concurrency on `updated_at`.
7. **Remote-only mode** → deferred to the matching spec.
8. **Duplicate-phone policy** → surface-and-link the existing member (I5-audited,
   admin-only).
9. **Device-token encryption (`PgDeviceStore`)** → deferred to the push spec (007).

No blocking ambiguities remain for `/speckit.plan`. Minor `/plan`-time items: pin the
accessible-primitive library + TanStack versions from the lock; enumerate issuance error
codes in `docs/error-codes.md`; author the key-management runbook (tied to ADR-0025).
