---
description: Invoke the clarifier subagent on the current spec. Identifies ambiguities, contradictions, and unstated assumptions before /speckit.plan.
---

You will run the clarifier subagent on the current spec.

## Steps

1. Identify the current spec. If the user did not specify, find the most recently modified `specs/NNN-*/spec.md`.
2. Invoke the `clarifier` subagent with that path.
3. Wait for the findings.
4. Present findings to the user.
5. Offer to update the spec to resolve the criticals (the user confirms; you do the edits).

After the spec is updated, suggest: "Now run `/speckit.plan` to produce a technical plan."
