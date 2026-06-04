---
description: Create a new spec under specs/NNN-name/ from the constitution-aware template, with the next available number.
---

You will create a new spec for the Boundless project.

## What I need from the user

If not already provided, ask for:
1. **Short kebab-case name** (e.g., `rider-opt-out`, `driver-clock`).
2. **One-sentence description.**

## Steps

1. Find the next free spec number by listing `specs/`. Use 3 digits, zero-padded (`001`, `002`, ...).
2. Create the directory `specs/NNN-<name>/`.
3. Create `spec.md` from the template below.
4. Output the path and remind the user to run `/clarify` next.

## spec.md template

```markdown
# NNN — <Title>

> Spec status: Draft (awaiting `/clarify`)
> Author: <user>
> Date: <ISO>

## One-paragraph summary

<What this feature does, in plain language a translator could read.>

## User story

As <persona>, I want <outcome>, so that <reason>.

## What changes for Maria? (rider primary persona)

<Required. If nothing changes, say so explicitly.>

## What changes for Daniel? (driver primary persona)

<Required if Driver-affecting; otherwise say N/A.>

## What changes for Sarah? (admin primary persona)

<Required if Admin-affecting; otherwise say N/A.>

## What changes for edge personas?

<Margaret, Tobias, etc., if relevant.>

## Detailed behavior

<Step-by-step, including what the system shows, what the user sees, what the user does, what the system does next. Include states.>

## States and transitions

<List or diagram. Every state visible to a user must be enumerated.>

## Acceptance criteria (testable)

- [ ] AC1: <precise, observable, testable>
- [ ] AC2: <…>
- [ ] AC3: <…>

## Edge cases

- What if no driver is available?
- What if the rider opts out at the last second?
- What if the driver drops out post-match?
- What if the user's network is down?
- What if the user's clock is wrong?
- What if the user has Dynamic Type at xxxLarge?
- What if the user is using VoiceOver?

## Privacy notes

Which privacy invariants (I1–I12) does this touch? How are they preserved?

## A11y notes

- What snapshot variants are needed beyond the default four?
- Any VoiceOver labels or hints required?
- Any Switch Control / Switch Access concerns?

## i18n notes

New catalog keys this introduces (with placeholder English):

| Key | English | Notes |
|---|---|---|
| `rider.cant_make_it` | Can't make it tonight | Short, sentence case, no exclamation |
| ... |

## Voice and tone check

For each new user-visible string, does it pass `docs/voice-and-tone.md`?

## Constitution principles touched

- [ ] P_: <how>

## ADRs referenced

- ADR-NNNN: <title>

## Out of scope (explicitly)

- <…>
- <…>

## Open questions

<List. Each will be resolved during /clarify.>
```

After writing the file, output:

> Spec created at `specs/NNN-<name>/spec.md`. Next step: run `/clarify` to find ambiguities.
