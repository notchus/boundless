# Boundless Personas

> Concrete people the product is built for. Every spec must include a "What changes for Maria?" section grounding the change in these personas. Reviewer subagent rejects specs that gesture at "users" generically.

These personas are derived from the design transcripts. They are not invented marketing personas — they are the lived shape of the people the original author had in mind.

---

## Maria — Primary Rider (75, retired, mild cataracts)

**Who she is:** Member of a small congregation that gathers Thursday evenings and Sunday mornings. Lives alone in a quiet street. Smartphone-capable but slow; has a basic iPhone with the largest text size enabled and her family's phone numbers as favorites. Does not have a watch.

**What she does in Boundless:**
- Opens the app once a week or so to check that it still says "You're coming tonight."
- Taps the small button "Can't make it tonight" only when she's unwell — maybe twice a year.
- Notices the calm "~ 6:12 PM" card the day of the gathering.
- Receives a doorbell notification when her driver arrives, lock-screen card visible.

**What she does NOT do:**
- Type addresses.
- Tap a map.
- "Search" for anything.
- Receive driver-changed notifications.

**Quotes (paraphrased from intent):**
- "I just want to know that someone's coming."
- "If the screen has a checkmark, I'm fine."

**Design implications:**
- One screen, one large affordance, no map, no clutter.
- The default state of the app is *informative*, not interactive.
- Text scales to xxxLarge. Buttons large. Contrast high.
- Voice and tone: warm, certain, never asking her to do something she doesn't have to.

---

## Daniel — Primary Driver (54, has a car, capable with phones)

**Who he is:** Member of the same congregation. Works regular hours, has a sedan, willing to pick up two riders on the way Thursday evenings. Owns an Apple Watch.

**What he does in Boundless:**
- Flips the "I have a seat tonight" toggle Thursday afternoons.
- Sets his **Effort Caps**: up to 8 km, 2 riders max.
- Sometimes opens the **Drive-Off Clock** to set his recurring departure time (6:05 PM Thursdays).
- Receives the assigned chain (with Riders' opaque names, neighborhood-level location until close).
- Taps "On my way" when leaving — this triggers Maria's "~ 6:12 PM."
- Occasionally has to drop out — taps "I can't make it anymore"; the system reassigns silently.

**What he does NOT do:**
- See Riders' full addresses until he is in their neighborhood.
- See other Drivers' identities.
- Get scored, ranked, or gamified.

**Design implications:**
- Driver app is moderately information-dense — Daniel can handle it.
- Watch complication: "2 riders · 6:05 PM departure" at a glance.
- The "I can't make it" button must be reachable in under 2 taps from any state.

---

## Sarah — Primary Admin (47, congregation organizer)

**Who she is:** Volunteer who manages the membership rolls. Tech-comfortable. Uses a laptop. May also be a Rider or Driver on the weekend.

**What she does in Boundless:**
- Receives an invite link from the developer to create her admin account.
- Issues accounts to new members (rider or driver), enters their address, phone, role.
- Performs role swaps (e.g., "Margaret can drive this Sunday, she has a guest car").
- Sees a weekly digest of "nobody was matched" situations and follows up by phone.
- Audits reads of address data (audit log visible to her).

**What she does NOT do:**
- Create other admin accounts (only the developer can).
- Manage multiple Groups (one Boundless install = one Group).
- See live ride state (that's the matching service's job).

**Design implications:**
- Web admin, desktop-first, responsive to tablet.
- Heavy use of tables, search, filters.
- Keyboard-shortcut-friendly.
- Audit log is first-class, not a buried setting.

---

## Margaret — Edge-Case Rider (82, doesn't go every week)

**Who she is:** Member of the congregation who attends only sometimes — her health varies. Lives with her daughter who drives her when she can.

**What she does in Boundless:**
- The app shows her "You're coming tonight" — but most weeks she taps "Can't make it tonight" early in the day.
- Occasionally her daughter is unavailable and Margaret leaves the default on — she's matched.

**Design implications:**
- Opt-out flow must be **frictionless**. No confirmation modal. No "are you sure." A single quiet tap.
- The system must not make her feel like she's saying "no" — it's just "tonight."
- A "Join from home" or "remote only" mode is configurable for members who attend remotely (per chat).

---

## Tobias — Edge-Case Driver (33, works variable shifts)

**Who he is:** Younger driver, often works late but wants to help when he can.

**What he does in Boundless:**
- Does not flip the seat toggle on most weeks.
- When he does, he sets tight Effort Caps (up to 5 km, 1 rider max) because of his schedule.
- Sometimes flips on mid-evening and gets matched for a partial chain.

**Design implications:**
- Effort Caps must be visible and editable from the same screen as the seat toggle.
- Late-arriving drivers must be matchable to remaining unmatched riders without requiring a full re-match.

---

## Anti-Personas (we do NOT build for these)

- **Power users who want every feature surfaced.** Boundless's value is what it omits.
- **Growth-hacker product managers.** No A/B tests on the rider surface, no nudges.
- **Surveillance-curious admins.** Admin sees what they need; everything is audit-logged.
- **Tech-skeptic boomers being convinced.** We don't try to "modernize" them; the app fits *their* shape.

---

## How to use these personas

When writing a spec:

```markdown
## What changes for Maria?
(filled out: how does this change her experience, in plain language)

## What changes for Daniel?
(if relevant)

## What changes for Sarah?
(if relevant)

## What changes for edge cases?
(Margaret / Tobias if relevant)
```

If a change cannot be grounded in at least one persona, it probably doesn't belong in the product.
