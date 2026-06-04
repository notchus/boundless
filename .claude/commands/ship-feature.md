---
description: Run the full pre-merge checklist for a feature — review, security audit, i18n validation, platform parity. Invokes the appropriate subagents in parallel.
---

You will orchestrate the full pre-merge checklist for the current branch.

## Steps

1. **Determine the diff.** Use `git diff origin/main...HEAD`.
2. **Determine the spec.** The PR body or commit messages should reference `specs/NNN-*/spec.md`. If unclear, ask the user.
3. **Invoke subagents in parallel** (up to 10 can run concurrently):
   - `reviewer` — code quality and constitution
   - `security-auditor` — if the diff touches PII / crypto / auth
   - `i18n-validator` — if the diff touches user-visible strings
   - `platform-parity` — if the diff touches shared types or API contracts
4. **Collect findings.** Present a single consolidated report.
5. **Block merge** if any finding is Critical.
6. **Suggest follow-ups** for Warnings; surface but don't block on Suggestions.

## Output

```markdown
# Ship checklist: <branch / PR title>

## Spec
specs/NNN-<name>/spec.md

## Subagent results

### reviewer: N critical, M warning, K suggestion
(highlights)

### security-auditor: …

### i18n-validator: …

### platform-parity: …

## Verdict
- [ ] Ready to merge
- [ ] Critical findings — fix before merge
- [ ] Warnings — fix or document why deferred

## Suggested next steps
…
```
