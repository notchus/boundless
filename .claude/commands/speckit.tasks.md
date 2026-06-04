---
description: Decompose the approved plan into specs/NNN-name/tasks.md — an ordered, dependency-aware, testable task list (the implement gate's contract).
---

You will decompose the current spec's plan into a task list.

## Preconditions

- `specs/NNN-name/plan.md` exists. If not, stop and tell the user to run `/speckit.plan` first.

## Steps

1. Identify the current spec and read its `spec.md` + `plan.md` (and the constitution).
2. Use the plan's **sequencing** section as the spine. For larger specs you may invoke the `test-strategist` to firm up per-task test obligations.
3. Write `specs/NNN-name/tasks.md`: a numbered, dependency-ordered list. Each task is one PR-sized slice with: what it does, the files/areas it touches, the acceptance criteria (AC IDs) it closes, the tests it must add, and its blockers. Mark which tasks are parallelizable; respect the plan's hard serialization points.
4. Every task maps to at least one spec AC; no task introduces behavior absent from the spec (P6 — anything not in `tasks.md` is scope creep).

After writing, suggest: "Pick task 1, start a fresh session, and run `/speckit.implement`; `/compact` and end the session when it's done."

> Local shim. See `DEFERRED.md` → Spec-Driven tooling.
