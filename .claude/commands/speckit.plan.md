---
description: Produce specs/NNN-name/plan.md from the clarified spec, via the architect (+ platform-parity, security-auditor, test-strategist) subagents and the constitution.
---

You will produce the technical plan for the current spec.

## Preconditions

- The spec exists and has passed `/clarify` (status Clarified). If not, stop and tell the user to run `/clarify` first (constitution P5/P6).

## Steps

1. Identify the current spec (most recently modified `specs/NNN-*/spec.md` unless the user names one). Read it, the constitution, the relevant ADRs, and `docs/{architecture,stack-matrix,privacy-invariants,operational-invariants,a11y-bar,forbidden-patterns}.md`.
2. Invoke the `architect` subagent to produce the backbone plan: constitution gate check, where each piece lives (core / server / clients), data model & migrations, API surface (OpenAPI + proto), per-platform work breakdown, dependency-ordered sequencing, and open decisions.
3. For non-trivial specs, also invoke **in parallel**: `platform-parity` (shared contract surface), `security-auditor` (risk register for any PII/auth surface), and `test-strategist` (test levels + AC→test mapping). Skip any that don't apply.
4. Synthesize their outputs into `specs/NNN-name/plan.md`. The parent session writes the file; the subagents are read-only.
5. **Surface any conflict** with the constitution, an ADR, the glossary, or `architecture.md` rather than silently reconciling. List open decisions that need the user's call.

After writing, suggest: "Now run `/speckit.tasks` to decompose the plan into an ordered task list."

> Local shim. GitHub Spec Kit was not installed upstream; this command drives the same constitution-aware, subagent-based flow as the project's custom commands. See `DEFERRED.md` → Spec-Driven tooling.
