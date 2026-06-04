---
description: Create a new Architecture Decision Record under docs/adr/ with the next available number.
---

You will create a new ADR (Architecture Decision Record).

## What I need from the user

1. **Short kebab-case title** (e.g., `rust-core`, `cloudflare-edge`).
2. **One-sentence summary** of the decision.

## Steps

1. Find the next free ADR number by listing `docs/adr/`. Use 4 digits, zero-padded.
2. Create `docs/adr/NNNN-<title>.md` from the template below.
3. Output the path; remind user to commit alongside the work it documents.

## ADR template

```markdown
# ADR-NNNN: <Title>

- **Status:** Proposed | Accepted | Deprecated | Superseded by ADR-MMMM
- **Date:** <ISO>
- **Author:** <user>
- **Deciders:** <user, others if any>

## Context

<What problem does this decision address? What's the current situation? What's the pressure?>

## Decision

<The choice made, in one or two paragraphs.>

## Considered alternatives

### Option A — <name>
**Pros:** …
**Cons:** …

### Option B — <name>
**Pros:** …
**Cons:** …

(at least 2 alternatives explicitly considered)

## Consequences

### Positive
- …

### Negative / costs
- …

### Neutral / follow-ups
- …

## Compliance

- Does this decision change the constitution? (If yes, version-bump constitution.md in the same PR.)
- Does this decision change the stack matrix? (If yes, update docs/stack-matrix.md in the same PR.)
- Does this decision require a migration of existing code?

## References

- Specs: <list>
- ADRs: <list>
- External: <URLs, papers, blog posts>
```
