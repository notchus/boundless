# Boundless — Claude Code Project Brief

> **You are working on Boundless: a free, open-source, accessibility-first geofence carpooling platform for closed groups (family / congregation / community) bringing elderly and disabled members to a recurring gathering. Native UI on every platform. Single shared Rust core. Cloudflare edge. Privacy-by-design.**

This file is auto-loaded into every session. Read it. Do not re-derive what's stated here.

---

## The Five Layers of Discipline

This project uses Spec-Driven Development (Spec Kit) with the following non-negotiable layered controls. Skipping a layer is a process bug.

1. **Constitution** (`.specify/memory/constitution.md`) — immutable principles. Every plan must explicitly reference and obey.
2. **Spec Kit gates** — `specify` → `clarify` → `plan` → `tasks` → `implement`. No implementation without an approved task list.
3. **Subagents** (`.claude/agents/`) — focused, isolated workers; the parent session never carries research/review context.
4. **Hooks** (`.claude/hooks/`) — post-edit format/test, pre-commit lint, pre-push full suite.
5. **CI gates** — final arbiter; nothing merges if red.

---

## Always-Loaded Context (read these before acting)

| File | What it gives you |
|---|---|
| `.specify/memory/constitution.md` | The principles. Highest authority. |
| `docs/domain-glossary.md` | Every Boundless term defined. **No new nouns without adding here first.** |
| `docs/personas.md` | Who we build for, with verbatim quotes. |
| `docs/voice-and-tone.md` | "Warmth" with do/don't examples. |
| `docs/stack-matrix.md` | Canonical library names + versions. **Never invent a version.** |
| `docs/privacy-invariants.md` | What must never break (privacy). Each invariant is enforced by code. |
| `docs/operational-invariants.md` | What must never break (operations: updates, fallbacks). Same enforcement model. |
| `docs/a11y-bar.md` | Accessibility floor. Required snapshot variants. |
| `docs/forbidden-patterns.md` | Per-stack anti-patterns. Reviewer grep-checks. |
| `docs/architecture.md` | The diagram + the why. |
| `docs/update-strategy.md` | The ladder for getting new behavior to riders (read on-demand for update-touching work). |
| `DEFERRED.md` (repo root) | Decided-but-not-yet-done checklist with WHEN triggers. |

Read **all** of these on session start. They are short by design.

## Imports (these actually load the files above into context at launch)
@../.specify/memory/constitution.md
@../docs/domain-glossary.md
@../docs/personas.md
@../docs/voice-and-tone.md
@../docs/privacy-invariants.md
@../docs/operational-invariants.md
@../docs/stack-matrix.md
@../DEFERRED.md

**Check `DEFERRED.md` at the repo root at the start of every session** — it tracks decided-but-deferred work, each item tagged with a WHEN trigger. If an item's trigger has arrived, act on it or surface it. When you defer something new, **record it in `DEFERRED.md` rather than leaving a `// TODO` in code** (the hooks reject those anyway).

---

## The Anti-Hallucination Protocol (mandatory)

These rules apply to every session, every model.

1. **Never invent a library, API, version, or file path.** If unsure, use a tool to verify:
   - Library docs → `Context7` MCP (preferred) or `Brave Search` MCP (fallback)
   - Cloudflare / Apple / Google docs → `WebFetch` against the official URL
   - Project files → `Read` / `Glob` — never recall a path from memory
2. **Lock files are ground truth** for versions: `Cargo.lock`, `Package.resolved`, `pnpm-lock.yaml`. Read them, don't guess.
3. **No "this should work" code.** If a build/test command exists, run it. Evidence > intuition.
4. **Cite the source** for any factual claim about libraries, Apple/Cloudflare/Google features, or external behavior. Include the URL.
5. **If you don't know, say so** and propose how to find out. Guessing is a process violation.
6. **Code without a passing test is incomplete.** No `// TODO` left in shipped code (post-commit hook fails on this).

---

## The Anti-"Lost in the Middle" Protocol

These rules keep your context small and focused.

1. **One session = one task** from `specs/CURRENT/tasks.md`. End the session when the task is done.
2. **Delegate research to `docs-researcher`** — never read docs in the main context.
3. **Delegate review to `reviewer`** — never re-read your own diff in the main context.
4. **Append decisions to ADRs** (`docs/adr/`) — they are the long-term memory. Read them when needed; never try to "remember" them.
5. **`/compact` at every gate boundary** (spec → plan → tasks → per task), with a structured summary of what was decided.
6. **If a session exceeds ~80% context, stop.** Write a continuation note in the spec, end the session, start fresh.

---

## How to Work (the standard loop)

For any user request that introduces or changes behavior:

1. **Identify the spec.** If `specs/NNN-name/spec.md` doesn't exist for this work, run `/new-spec` first. Never start coding without one.
2. **Plan mode by default.** Claude Code's plan mode is the default — produce a plan, get the user's sign-off, then implement.
3. **Use subagents** — see `.claude/agents/README.md`. The big ones: `architect`, `clarifier`, `docs-researcher`, `reviewer`, `security-auditor`, `i18n-validator`, `platform-parity`, `test-strategist`.
4. **Touch one slice per PR.** "Rider iOS opt-out screen" is a slice. "Refactor while I'm here" is scope creep — open a separate spec.
5. **After every edit:** the post-edit hook auto-formats and runs scoped tests. If it fails, fix before continuing.
6. **Before commit:** the pre-commit hook lints, type-checks, and rejects any `// TODO`, `dbg!`, `print(...)` of PII types, etc.
7. **Before push:** the pre-push hook runs the full test suite + snapshot diffs.

---

## Subagents — How They Work in Claude Code

You delegate to a subagent by invoking the Task tool with the subagent's name. The subagent:

- Starts with an empty context window.
- Sees **only**: the prompt you give it, its own system prompt (its `.md` file body), and any skills listed in its frontmatter.
- Does its work in isolation.
- Returns a single summary message to you. Intermediate tool calls and verbose output stay inside the subagent.

**Key implication: front-load everything the subagent needs in the prompt.** It cannot see your conversation, your prior reasoning, or other subagents' outputs unless you pass them in.

**Read-only by default.** Most of our subagents have no Edit/Write/Bash. The parent session does all writes (only the parent can approve permission prompts). This is deliberate.

**Up to 10 subagents can run in parallel.** Use this for embarrassingly-parallel work like "review the same PR for security + i18n + a11y + platform-parity at the same time."

---

## Stack One-Liner (for context — see `docs/stack-matrix.md` for canonical versions)

- **Core:** Rust workspace; UniFFI → Swift/Kotlin; wasm-bindgen → TS (limited use)
- **Apple:** SwiftUI for iOS / iPadOS / watchOS / macOS / visionOS targets
- **Android:** Kotlin + Jetpack Compose (phone) + Compose for Wear OS (watch) + Glance (widgets)
- **Admin web:** SvelteKit 2 + TypeScript strict + Tailwind 4 + Radix Primitives
- **Edge / server:** Cloudflare Workers (workers-rs) + Durable Objects + Hyperdrive → Neon Postgres + PostGIS
- **i18n:** ICU MessageFormat, String Catalogs (.xcstrings) on Apple, strings.xml + ICU on Android, FormatJS on web, Weblate for translation workflow
- **API contracts:** OpenAPI 3.1 (HTTP), Protocol Buffers + Buf (WebSocket)
- **Testing:** proptest (Rust), swift-snapshot-testing, Paparazzi (Compose), Playwright (web), axe-core (a11y)

---

## What This Project Is Not

- Not a startup looking for product-market fit. The product is specified.
- Not vendor-locked. Every choice has a free-tier and OSS-compatible path.
- Not a place for clever code. Clarity > cleverness; this code will be read by volunteers.
- Not a place for "while I was here" refactors. Open a spec.

---

## When You're Confused

If anything in the user's request seems to conflict with the constitution, the glossary, an ADR, or a spec: **stop and surface the conflict**. Don't reconcile silently.
