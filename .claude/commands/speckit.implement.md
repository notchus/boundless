---
description: Implement one task from specs/NNN-name/tasks.md in an isolated session, against the constitution, hooks, and the spec's acceptance criteria.
---

You will implement exactly **one** task from the current spec's `tasks.md`.

## Preconditions

- `specs/NNN-name/tasks.md` exists. The user names the task, or take the lowest-numbered unblocked one.

## Rules (from CLAUDE.md / the constitution)

- One session = one task. Touch one slice. No "while I'm here" refactors — open a new spec/task.
- Plan mode by default; get sign-off before editing (P6).
- Delegate research to `docs-researcher`; never recall a library API/version from memory (anti-hallucination protocol). Lock files are ground truth for versions.
- Code without a passing test is incomplete. No `// TODO` in shipped code (the hooks reject it) — defer in `DEFERRED.md` instead.
- The post-edit hook formats + runs scoped tests; pre-commit lints/type-checks; pre-push runs the full suite. Fix red before continuing.

## Steps

1. Read the named task, the spec's relevant ACs, the plan, and the constitution.
2. Produce an implementation plan (plan mode); get the user's approval.
3. Implement the slice, writing the tests the task names (the plan/tasks AC→test mapping).
4. Before commit: run the `reviewer` subagent (and `security-auditor` / `i18n-validator` / `platform-parity` as relevant); fix findings.
5. Mark the task done in `tasks.md`; `/compact` and end the session.

> Local shim. See `DEFERRED.md` → Spec-Driven tooling.
