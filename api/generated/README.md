# Generated bindings — DO NOT EDIT BY HAND

Everything under `api/generated/**` is **generated** from the contract sources
(`api/openapi.yaml`, `api/boundless.proto`) and the Rust core (`core/`) by
`scripts/generate-bindings.sh`. These files **are committed** (so consumers don't need
the full toolchain) and are kept in sync by CI:

- `scripts/check-binding-drift.sh` fails if `core/`/`api/` changed without regenerated
  bindings, **and** fails if any file here was hand-edited.
- Regenerate after any contract/core change: `bash scripts/generate-bindings.sh`,
  then commit the result.

Per-language outputs (populated in **T10-shell**, per target, alongside the consuming UI tasks
**T11–T15**, once those toolchains exist — see `DEFERRED.md`):

| Dir | Generator | Consumed by |
|---|---|---|
| `swift/` | swift-openapi-generator + protoc-gen-swift | `apple/BoundlessKit/` |
| `kotlin/` | openapi-generator (kotlin) + protoc-gen-kotlin | `android/core-bridge/` |
| `typescript/` | openapi-typescript + ts-proto | `web/src/lib/api/generated/` |

Scaffolded by spec 001 task **T01**; the contracts (`api/openapi.yaml`, `api/boundless.proto`)
were **FROZEN** in **T10**; real codegen is wired in **T10-shell** (with T11–T15).
