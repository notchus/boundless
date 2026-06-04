# Boundless Update Strategy — The Ladder

> How Boundless gets new behavior in front of riders without ever asking them to do anything.
>
> The principle (P13): the rider must never see an "update required" interstitial. Operational changes climb a ladder. The lower rungs reach the rider invisibly. The higher rungs route through someone else.

---

## The ladder

```
   Rung 5 ─── Manual fallback (printable phone list)              ← worst case; gathering still happens
   Rung 4 ─── Admin-assisted (Sarah / family helps in person)     ← high-friction; non-urgent only
   Rung 3 ─── Native auto-update (overnight, no interaction)      ← enabled once at onboarding
   Rung 2 ─── Native update available but not forced              ← Sarah sees stragglers in dashboard
   Rung 1 ─── Server-driven config (KV, signed manifest)          ← invisible; covers most changes
   Rung 0 ─── Pure server behavior (Workers / DOs)                ← invisible; client doesn't care
```

The strategy: **resolve every change at the lowest rung it can be resolved at.**

---

## Rung 0 — Pure server behavior

Anything that doesn't change the client UI shape happens here.

- Matching algorithm tweaks.
- ETA calibration.
- Auth flow changes (within existing OpenAPI shape).
- Database schema migrations (with backward-compatible API).
- New audit log fields.
- Bug fixes in the Worker / Durable Object.
- Performance improvements.

**Deployment:** push via Wrangler. Rider sees zero change. Effective immediately.

---

## Rung 1 — Server-driven config (Cloudflare KV)

Anything that changes user-visible content but not client structure happens here.

| In KV | Examples |
|---|---|
| **Translations** | All catalog strings for all locales |
| **User-facing copy** | "Can't make it tonight" → "Stay home tonight" if we ever wanted (we wouldn't) |
| **Error messages** | Per-locale, per-error-code |
| **Design tokens (subset)** | Colors that don't require asset changes (background tints, accent shades) |
| **Feature flags** | Enable/disable per-group, per-rollout-cohort |
| **Microcopy** | Empty-state text, hint text, button labels |
| **Per-Gathering content** | "Tonight's special event" overrides |

The client fetches the **signed manifest** at launch (and periodically on long-running sessions), verifies the signature against a public key embedded in the app, and applies. No restart. Cached locally for offline launches.

**Format:** see ADR-0014 for the manifest schema.

**Deployment:** `wrangler kv:key put`. Rider sees the new content next time they open the app. Effective in seconds.

**What's NOT in KV (i.e., requires a binary update):**
- New screens or screen structures.
- New native capabilities (HealthKit, etc. — and Boundless avoids these anyway).
- Changes to data the app stores locally (Keychain schema, Core Data model, etc.).
- Updated cryptography primitives.
- Updates to the embedded Rust core.

---

## Rung 2 — Native update available but not forced

For changes that require a new binary but aren't urgent.

- New translation locale added (the strings are in KV, but the locale picker UI may need a build).
- New screen added.
- Native API changes.
- Embedded Rust core changes.
- Updated permissions (rare in Boundless — minimal permissions by design).

**Deployment:** ship to App Store / Play Store. Auto-update (Rung 3) handles it overnight on devices that have it enabled.

**Admin signal:** Sarah's "Devices" panel shows the version distribution. She may see "3 riders on v2.0.x — current is v2.2.x." She decides if/when to nudge.

---

## Rung 3 — Native auto-update (the actual delivery mechanism)

Enabled once at onboarding by an admin or family member (O3). Runs overnight. Rider does nothing.

The onboarding flow includes an explicit "let's enable automatic updates" step — the helper opens iOS Settings → App Store → Automatic Updates (toggle on) or Android Settings → Play Store → Network Preferences → Auto-update apps. This is a one-time setup.

**Critical:** the rider is not expected to remember this. The admin dashboard tracks devices and flags those still on old versions.

---

## Rung 4 — Admin-assisted (the human in the loop)

When a device is stuck below `client_min_version` (O4) and auto-update isn't catching up — e.g., the rider has the phone in airplane mode at night, or has manually disabled auto-update without realizing, or is on a Wi-Fi-only iPad that's rarely on Wi-Fi.

**What the rider sees:** the O4 graceful-degradation screen. Calm. One sentence. No update CTA.

**What Sarah sees:** an admin alert. "Margaret's phone hasn't checked in on the current version in 14 days. Her daughter Anna is the listed helper."

**What Sarah does:** calls Anna. Anna walks in, opens the App Store, updates Boundless. Done.

This is the high-friction rung — it costs a phone call. But it costs nothing of the rider, and it surfaces *actionable* signals to the admin (not just "12 devices outdated" but "here are the 3 you need to call about").

---

## Rung 5 — Manual fallback (the disaster recovery)

If the entire app fleet is down for any reason — Cloudflare incident, Apple/Google App Store outage, our DNS gone, anything — the gathering must still happen.

**The mechanism:** Sarah can export a printable phone list (O7) at any time. The list shows: Rider name, phone number, address, mobility notes, default driver assignments (manually maintained). She prints it. Drivers coordinate by phone.

This is not a fancy mechanism. It's a PDF export. But it's a guarantee: **the gathering does not depend on Boundless being up.** Boundless is a coordination layer over a community that already exists. If the layer fails, the community functions.

---

## The `client_min_version` mechanism in detail

This is the technical primitive that makes Rung 4 possible without ever showing the rider an update prompt.

### Schema

In `api/openapi.yaml`, every auth-touching response includes:

```yaml
client_min_version:
  type: string
  description: |
    The minimum client version the server still fully supports.
    Clients below this threshold should show the graceful-degradation
    screen and stop attempting interactive flows. Format: semver.
  example: "2.1.0"

client_recommended_version:
  type: string
  description: |
    The current recommended client version. Below this and above
    `client_min_version`, the client functions normally but may show
    a non-blocking admin-side signal that an update exists.
  example: "2.4.0"
```

### Client behavior

```
when (response.client_min_version > installed_version):
    set app state = "needs_help"
    show single screen: <%= localized("needs_help") %>
    do NOT proceed with auth flow
    do NOT show update CTA (rider can't action it)
    emit telemetry event "below_min_version" (non-PII: version string + group ID)

when (response.client_min_version <= installed_version < response.client_recommended_version):
    normal operation
    no UI signal to the rider
    admin dashboard signal exists but rider doesn't see it

when (installed_version >= response.client_recommended_version):
    normal operation, fully current
```

### Server behavior

- Both fields are configurable per-Group (so a beta group can require a newer version).
- Default values live in KV; changed via a single `wrangler kv:key put` command.
- Bumping `client_min_version` is a deliberate operational action — it's the only thing that pushes a rider into the "needs help" state. Document it as a runbook step, with rollback.
- Emit a Queue event when a member's most recent auth was below threshold. The admin alert is debounced to once per (member, day).

---

## The onboarding "set up this phone" flow

This is the single-most-important moment for P13 to hold. Get this right and 95% of riders never see Rung 4.

The onboarding spec (`specs/001-onboarding/`) must include:

1. **Welcome screen** — large, named, warm.
2. **Locale confirmation** — auto-detected, helper confirms.
3. **Notification permission** — explained in plain language ("Boundless will tell you when your driver is at your door, and nothing else").
4. **Critical Alerts permission** (iOS only) — same.
5. **Set up automatic updates** — instructions specific to iOS or Android, with screenshots if possible, walked through by the helper. After: a "✓ Automatic updates enabled" confirmation that the helper taps.
6. **Set up emergency helper** — the rider can name a family member who Sarah can call if the device gets stuck. Optional.
7. **All set** — "Hello, Maria. You'll see this app once a week, around your gathering day."

The helper steps must be skippable but loudly flagged in the admin dashboard if skipped ("Maria's auto-update not confirmed").

---

## What happens during a critical security update

Even in the worst case — a security flaw requires shipping a new binary urgently — the rider does not face an interruption.

The sequence:

1. Server-side mitigation deployed first (Rung 0). Buy hours or days.
2. Native build released to App Store / Play Store.
3. `client_recommended_version` bumped in KV.
4. **Wait at least 72 hours** for auto-update to catch most devices (Rung 3).
5. Admin dashboard surfaces stragglers (Rung 4); Sarah and helpers chase them.
6. Only after attempted human contact does the server raise `client_min_version`. Even then, the rider sees the calm degradation screen — never an "Update Now" CTA.
7. If 5% of riders still can't be updated and the issue is severe enough, the admin uses Rung 5 (manual phone list) until those devices catch up.

The rider's experience throughout: nothing visibly changes until the night the phone updates automatically, or the night someone they trust knocks on their door.

---

## Cross-references

- **Constitution principle:** P13.
- **Operational invariants:** O1–O8 in `docs/operational-invariants.md`.
- **ADR:** 0014 (server-driven config via KV).
- **Suggested specs:**
  - `001-onboarding` — includes the "set up this phone" flow.
  - `011-admin-update-visibility` — the admin Devices panel.
  - `012-admin-export-phone-list` — the Rung 5 export.
- **Architecture:** `docs/architecture.md` KV section.
