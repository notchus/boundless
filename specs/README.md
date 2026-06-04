# specs/

This directory holds your specifications, one folder per spec, numbered:

```
specs/
├── 001-onboarding/
│   ├── spec.md           ← /new-spec creates this from a template
│   ├── plan.md           ← /speckit.plan produces this
│   ├── tasks.md          ← /speckit.tasks produces this
│   └── acceptance.md     ← (optional) testable criteria, derived from spec
├── 002-rider-opt-out/
└── ...
```

## Suggested first 10 specs

Order is roughly bottom-up — auth and shared types first, surface features next.

| # | Name | What it materializes |
|---|---|---|
| 001 | `onboarding` | Auth model + first-launch flow for Rider, Driver, Admin |
| 002 | `rider-opt-out` | The canonical Rider interaction; tests warmth + a11y |
| 003 | `driver-seat-toggle` | Driver evening flow; sets up Effort Caps |
| 004 | `matching-engine` | The Rust core matching algorithm with property tests |
| 005 | `chain-display-driver` | How Driver sees their chain (neighborhood until <1km) |
| 006 | `approximate-pickup-time` | The "around 6:12 PM" surface on Rider |
| 007 | `doorbell-notification` | Critical Alert + Live Activity when Driver arrives |
| 008 | `admin-member-management` | Sarah's primary surface |
| 009 | `silent-reassignment` | Driver drops out → silent re-match |
| 010 | `optional-live-tracker` | E2E-encrypted opt-in tracker |

Don't write all of these up front. Write spec 001, drive it through plan/tasks/implement/ship, *then* write spec 002. The point of the methodology is to ship the small loop fast and learn.

## To create a spec

In Claude Code:

```
/new-spec <kebab-name>
```

This invokes the slash command at `.claude/commands/new-spec.md`, which scaffolds the directory with a constitution-aware template.

After drafting your spec, the next steps are:

1. `/clarify` — invokes the `clarifier` subagent
2. `/speckit.plan` — Spec Kit's planner (uses the `architect` subagent and the constitution)
3. `/speckit.tasks` — decompose into ordered tasks
4. Per task: fresh Claude Code session, implement, `/compact` and end

See `SETUP.md` Section 8 for the first-session walkthrough.
