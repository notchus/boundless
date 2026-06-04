# Architecture Decision Records (ADRs)

This directory is the **append-only long-term memory** of the Boundless project. Every non-trivial architectural decision lands here.

## Why ADRs

The biggest long-term LLM failure mode is *context drift*: a model in month 18 doesn't know why the model in month 1 decided what it did. ADRs are the fix. The model reads them when needed instead of trying to remember them.

## Conventions

- **Numbering is permanent.** Once `0001-rust-core.md` exists, that number is taken forever — even if the ADR is superseded.
- **Filenames:** `NNNN-short-kebab-title.md` where NNNN is 4 digits, zero-padded.
- **Status field:** `Proposed`, `Accepted`, `Deprecated`, `Superseded by ADR-NNNN`.
- **Never delete an ADR.** Mark it Deprecated or Superseded and explain.
- **Specs reference ADRs**, not the other way around.
- **Every ADR considers at least 2 alternatives** explicitly.
- **Every ADR documents trade-offs**, not just upsides.

## Process

To create one, use the slash command:

```
/adr <short-kebab-title>
```

It scaffolds the file from the template in `.claude/commands/adr.md`.

## Read this before reading any individual ADR

The constitution (`.specify/memory/constitution.md`) holds higher authority than any ADR. If an ADR and the constitution conflict, the constitution wins, and the ADR needs to be amended (or the constitution updated via the amendment process documented at the bottom of the constitution).

## Currently-active ADRs

- [ADR-0001](./0001-rust-core.md) — Single shared Rust core for domain types and business logic
- [ADR-0013](./0013-license.md) — AGPL-3.0 for the entire repository (+ App Store §7 exception)
- [ADR-0014](./0014-server-driven-config.md) — Server-Driven Configuration via Cloudflare KV (supports P13 / O1–O8)

## Suggested early ADRs to author (stubs)

These were decided during the planning chats and should be formalized:

- ADR-0002 — Native UI on every platform (SwiftUI / Compose / SvelteKit), no cross-platform UI frameworks
- ADR-0003 — Cloudflare edge as the server tier (Workers + DOs + Hyperdrive)
- ADR-0004 — Neon Postgres with PostGIS via Hyperdrive for the primary store
- ADR-0005 — OpenAPI 3.1 + Protocol Buffers as API contract source of truth
- ADR-0006 — Role swaps allowed (a person may be Rider in one context, Driver in another)
- ADR-0007 — Silent reassignment (no "your driver changed" notification)
- ADR-0008 — ETA matrix computed batch on admin updates, not in request path
- ADR-0009 — Closed-group privacy model (no self-signup, admins issued by developer only)
- ADR-0010 — Optional Live Tracker is E2E encrypted (server cannot decrypt)
- ADR-0011 — Spec-Driven Development with GitHub Spec Kit as the development methodology
- ADR-0012 — Weblate as the translator workflow (self-host vs hosted: open)

Write these as you encounter the decisions during implementation, not all at once.

> ADR-0014 was authored ahead of order because the update-strategy is foundational — P13 needed it materialized before any UI work begins, since the manifest schema constrains how every client renders content. Subsequent ADRs (0015+) get the next available numbers.
