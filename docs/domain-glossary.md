# Boundless Domain Glossary

> **Single source of truth for all domain terms.** If a noun is not here, it does not exist. Adding a new term requires a PR that updates this file. The glossary-lint subagent rejects PRs that introduce undefined nouns in code or specs.

---

## People

### Rider
A group member who needs transportation to the **Gathering**. Riders are **in by default**; they only act to opt out.
- Synonyms: ❌ "passenger" (don't use), ❌ "user" (too generic), ❌ "elderly"/"disabled" (use only when discussing accessibility specifics).
- Code type: `Rider` (Rust), `Rider` (Swift), `Rider` (Kotlin).

### Driver
A group member with a car and capacity, who has flipped their **Seat Toggle** on for tonight.
- Code type: `Driver`.
- A Driver may **also** be a Rider in another context (role swaps supported, see ADR-006-role-swaps).

### Admin
A trusted member who manages **Group** membership, addresses, and role swaps. Admin accounts are issued **only** by the developer; admins cannot self-onboard, and they cannot create other admins.
- Code type: `Admin`.

### Developer
The person operating the Boundless instance. Only role with the ability to issue Admin accounts.

---

## Things

### Group
A closed organization (typically family / congregation / community) inside a single Boundless install. **One person belongs to exactly one Group.** Groups have an `id`, a human-readable `name`, and a set of **Recurring Gatherings**.
- Synonyms: ❌ "tenant", ❌ "organization" (too business-y), ❌ "team".

### Gathering
The destination event — the place riders are being brought to. Two kinds:
- **Recurring Gathering** — known time + place, repeats weekly (e.g. "Thursday gathering at 7pm at 123 Main St").
- **Special Event** — one-off, with its own date/time/place.

The term "meeting" is **banned** in user-facing copy (per chat history) — it carries the wrong connotation.

### Seat Toggle
The Driver's "I have a seat tonight" affordance. A boolean per (Driver, Gathering instance), with optional **Effort Caps**.

### Effort Caps
A Driver's voluntary limit on (max km/mi, max riders) for tonight. Per-Gathering, editable.

### Chain
The ordered list of (Driver → Rider₁ → Rider₂ → … → Gathering) computed by the **Matching Service**. Chains are constructed to minimize total km while respecting Effort Caps.

### Opt-Out
A Rider's "can't make it tonight" action. May happen at any time (no cutoff). Triggers a reassignment if the chain is affected.

### Reassignment
A silent re-computation of a Chain when a Driver drops out post-match. The new Driver introduces themselves to the Rider in person; the app does not display a "your driver changed" screen.

### Drive-Off Time
The Driver's planned departure time from home, for a Recurring Gathering. Used to compute the Rider's **Approximate Pickup Time**.

### Approximate Pickup Time
The calm "~ 6:12 PM" shown to the Rider after matching. Computed as `drive_off + drive_duration`. Locations are dropped after this is computed (per P3).

### Doorbell Notification
The full-bleed terra "Daniel is at your door" notification + Live Activity on the Rider's phone and watch when the Driver arrives.

### Optional Live Tracker
An opt-in (Driver's choice) E2E-encrypted live position feed shown to the Rider after the Driver has left. Default: off. Never shows Rider's address.

### Onboarding Code
The one-time, Admin-issued secret a trusted helper enters during a member's first-launch to bind that device to the member's identity. Short-lived, single-use, rate-limited, server-validated; regenerable by an Admin. Carries no PII. Not SMS/email-based (per ADR-0016, P11, I8).
- Code type: `OnboardingCode`.
- See: ADR-0016, spec 001.

### Recovery Code
A single-use, **Driver-held** secret captured once at the Driver's onboarding, used to self-serve re-bind a new device without Admin involvement (phone number + Recovery Code). A fresh one is issued on use; if lost, the Driver falls back to the Admin re-issuing an **Onboarding Code**. Riders do **not** use Recovery Codes — Riders always recover via an Admin.
- Code type: `RecoveryCode`.
- See: ADR-0016.

---

## Concepts

### Geofence Matching
Nearest-neighbor matching where the closest available Driver is paired with the closest unmatched Rider, then extended into a Chain. Implemented in `core::matching`.

### Chained Pickups
The default model: one Driver picks up multiple Riders en route to the Gathering, in optimal order.

### Closed Group
The privacy model: Boundless instances are not public marketplaces. Only Admin-issued accounts exist. There is no signup form anywhere.

### Unbreachable by Design
The product's privacy promise. Concretely: PII encrypted at rest at the field level; matching happens in ephemeral memory; addresses dropped after match; admin reads audit-logged; no third-party trackers.

### Warmth
The product's voice. See `docs/voice-and-tone.md`. Examples: "You're coming tonight," "Daniel will be at your door in about 7 minutes," "Can't make it tonight."

---

## Roles in Code

| Code term | Means |
|---|---|
| `Group::id` | Stable UUID, never displayed |
| `MemberId` | Generic ID type; refers to a (Group, Person) pair |
| `Role` | Enum: `Rider`, `Driver`, `Admin` (a person can hold multiple in different contexts) |
| `Address` | Tainted-type wrapper, encrypted at rest, never logged |
| `Chain` | Ordered list of `MemberId` ending at a Gathering |
| `MatchingContext` | Ephemeral compute context that holds plaintext addresses; dropped after `compute()` returns |

---

## Banned Words (in code, docs, and UI)

| Word | Why banned | Use instead |
|---|---|---|
| "meeting" | Wrong vibe — Boundless events are warm gatherings, not corporate meetings | "gathering" |
| "passenger" | Too transactional | "rider" |
| "elderly/disabled" (in UI) | Othering | (don't label users; build for everyone) |
| "user" (in user-facing copy) | Cold | The person's role: rider, driver, admin |
| "request a ride" | Rideshare connotation | "tonight" / "the gathering" |
| "table" | From the original spec, replaced | "gathering" |
| "destination" | Generic | "the gathering" / "the special event" |
| "trip" | Generic | "ride" or "tonight's ride" |
