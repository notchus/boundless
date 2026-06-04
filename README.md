# Boundless — Bootstrap

This directory is a starter kit for Boundless, the privacy-first geofence carpooling platform for closed groups. It does not yet contain application code. It contains the **operating context** — the constitution, glossary, personas, voice, stack matrix, privacy invariants, accessibility floor, forbidden patterns, architecture, subagents, slash commands, hooks, and MCP configuration — that turns Claude Code into a disciplined development partner for the project.

**Start with [SETUP.md](./SETUP.md).** It walks you through everything you need to do, in order.

## What's in here

```
.
├── SETUP.md                          ← read this first
├── .claude/                          ← Claude Code project config
│   ├── CLAUDE.md                       auto-loaded into every session
│   ├── agents/                         8 read-only subagents + a README
│   ├── commands/                       /new-spec, /clarify, /adr, /ship-feature
│   ├── hooks/                          post-edit, pre-commit, pre-push
│   └── settings.json.template          → copy to settings.json
├── .specify/
│   └── memory/
│       └── constitution.md             12 principles (P1–P12)
├── .mcp.json.template                ← copy to .mcp.json with your keys
├── .gitignore
├── docs/                             ← always-loaded operating context
│   ├── architecture.md
│   ├── domain-glossary.md
│   ├── personas.md
│   ├── voice-and-tone.md
│   ├── stack-matrix.md
│   ├── privacy-invariants.md           I1–I12
│   ├── a11y-bar.md
│   ├── forbidden-patterns.md
│   └── adr/                            architecture decision records
│       ├── README.md
│       └── 0001-rust-core.md           worked example
└── specs/                            ← Spec Kit specs (you create as you go)
```

## What this bootstrap is NOT

- **Not application code.** Code lives under `core/`, `apple/`, `android/`, `web/`, `server/` — none of which exist yet. The first specs you write will scaffold those.
- **Not a Spec Kit replacement.** It coexists with Spec Kit. After `specify init`, the `.specify/` directory will gain templates and scripts alongside the constitution you already have.
- **Not opinionated about your editor.** Designed for Claude Code CLI, but the docs files are readable in any editor.

## License & contribution

Boundless is intended to be free and open source. Add a `LICENSE` file (suggested: AGPL-3.0 to keep derivatives open, or MIT for maximum permissiveness — your call, file an ADR). Add `CONTRIBUTING.md` when you're ready for outside contributors.

## Status

You're at step zero. After completing SETUP.md, write your first spec — onboarding is a good first one (it touches all three roles and forces you to materialize the auth model).

— Generated as a Claude Code starter kit for the Boundless project.
