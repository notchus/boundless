# Boundless Subagents

> Eight focused, read-only subagents that absorb noisy context (research, review, audit) so the parent session stays small and decisive.

## Why subagents?

A subagent is an isolated Claude session. Its context starts empty except for: the prompt you give it, its system prompt (the markdown file body), env details, and any skills listed in its frontmatter. It returns one summary message. Intermediate tool calls stay inside it.

**This is the central anti-"lost-in-the-middle" tool.** Anything verbose — docs research, code review, log analysis — should be delegated.

## Why read-only?

Subagents cannot show interactive permission prompts. If a subagent tries `Edit`/`Write`/`Bash` and it matches an ask rule, the call is **denied silently**. Best practice: subagents are read-only; the parent does all writes. Our agents follow this.

## When to invoke

You can call them explicitly:
> Use the `reviewer` subagent on the current diff.
> Use the `docs-researcher` subagent to fetch the latest UniFFI docs for binding async functions.

Or Claude Code auto-delegates based on the `description` field. Write clear `description`s; that's the routing signal.

## The eight

| Agent | When to use | Returns |
|---|---|---|
| **architect** | After a spec is clarified, before planning | A `plan.md` proposal: which platforms, types, endpoints, migrations |
| **clarifier** | After a fresh spec is drafted | A list of ambiguities + suggested resolutions |
| **docs-researcher** | When introducing or upgrading any library/API | A condensed "API surface card" with citations |
| **reviewer** | After implementation, before commit | A prioritized findings list (Critical / Warning / Suggestion) |
| **security-auditor** | On any PII- or crypto-touching diff | Risks vs the privacy invariants |
| **i18n-validator** | On any user-visible string change | Catalog gaps across locales |
| **platform-parity** | On any change to shared types or API contracts | Drift report iOS/Android/Web |
| **test-strategist** | When designing a new test suite | A test plan: unit, property, snapshot, integration |

## Models

- `architect`, `clarifier`, `reviewer`, `security-auditor`, `platform-parity`, `test-strategist`: **inherit** (use the parent's model, typically Sonnet or Opus)
- `docs-researcher`, `i18n-validator`: **haiku** (cheap, focused, repetitive)

## How to extend

Drop a new `.md` file under `.claude/agents/` with frontmatter:

```
---
name: my-agent-name
description: When Claude should delegate to this. Be specific.
tools: Read, Glob, Grep
model: inherit
---
System prompt body — what the agent does, in detail.
```

## Anti-patterns for our agents

- ❌ Giving an agent `Edit`/`Write`/`Bash` — they can't approve their own writes.
- ❌ Writing a vague `description` like "code helper" — the auto-router won't find it.
- ❌ Passing a tiny prompt expecting the agent to remember conversation context — it can't.
- ❌ Chaining agents without writing intermediate results to a file the next agent can read.
