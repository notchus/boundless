# Boundless Forbidden Patterns

> These are anti-patterns that have been actively decided against. The `reviewer` subagent grep-checks for them. CI lint rules enforce the ones that can be automated.
>
> If you have a legitimate reason to violate one of these, write an ADR.

---

## Universal (all stacks)

| Pattern | Why forbidden | Fix |
|---|---|---|
| Hardcoded user-facing strings | Breaks i18n (P8) | Use the catalog |
| Logging PII types | Privacy (P2, I10) | Use `redacted_summary()` |
| `TODO` / `FIXME` left in shipped code | Incomplete work signal | Resolve or open a task |
| Dead code commented out | Git is the history | Delete |
| Generic exception catching that swallows | Hides bugs | Catch specific, re-raise others |
| "Optimistic UI" that lies to the user | Trust violation | Show real state with skeletons |
| A/B tests on rider primary flow | Persona violation (P10) | Decide, don't experiment, on Maria |
| Third-party analytics | Privacy (I8) | Self-hosted OTel |
| Date/time without timezone | Bugs in non-UTC regions | Always TZ-aware |

---

## Rust (core, server)

| Pattern | Why forbidden | Fix |
|---|---|---|
| `unwrap()` in non-test code | Panic in production | `expect("reason")` or `?` |
| `println!` / `dbg!` | Bypasses scrubbed logging (I10) | `tracing::info!` |
| Direct `SystemTime::now()` | Untestable | Inject a `Clock` trait |
| `std::sync::Mutex` in async code | Blocks executor | `tokio::sync::Mutex` |
| `Arc<Mutex<HashMap>>` for shared state | Coarse-grained | Per-key locking or actor pattern |
| `Box<dyn Error>` in library code | Loses type info | Concrete error enum with `thiserror` |
| `String` for IDs | Confusable | Newtype: `RiderId(Uuid)` |
| `i64` for currency | Floating point bugs | Domain currency type, integer cents |
| `let _ =` discarding `Result` | Silent failure | Match or handle |
| `#[allow(dead_code)]` outside tests | Hides scope creep | Remove the code |
| `pub` on internal items | Breaks encapsulation | `pub(crate)` |

### Rust core additional

| Pattern | Why forbidden | Fix |
|---|---|---|
| Network code in `core::domain` | Layer violation (P4) | Put it in `core::sync` |
| `std::fs` in `core::domain` | Layer violation | Inject a storage trait |
| `tokio::spawn` in `core::domain` | Couples to runtime | Return a future |

---

## Swift (Apple apps)

| Pattern | Why forbidden | Fix |
|---|---|---|
| Force unwrap (`!`) of optionals | Crash risk | `guard let` / `if let` / `?` |
| `print(_:)` of any tainted type | Logging PII (P2) | `Logger` with redaction |
| Hardcoded point sizes for body text | Breaks Dynamic Type | `.font(.body)` etc. |
| `UserDefaults` for PII | Unencrypted at rest | Keychain |
| `Combine` for app state | Replaced by Observation | `@Observable` |
| Third-party DI containers (Resolver, Swinject) | Adds magic | Pass deps through init |
| `UIKit` mixed into SwiftUI without `UIViewRepresentable` | Lifecycle bugs | Wrap explicitly |
| `DispatchQueue.main.async` in SwiftUI body | Bug-prone | `Task { @MainActor in }` |
| Singletons (`static let shared`) for stateful services | Untestable | Pass via environment |
| `try!` outside tests | Crash risk | `do/try/catch` |
| Storyboards or XIBs | Source of truth confusion | SwiftUI only |
| `@AppStorage` for PII | Same as UserDefaults | Keychain |

---

## Kotlin (Android apps)

| Pattern | Why forbidden | Fix |
|---|---|---|
| `!!` (non-null assertion) | Crash risk | Safe call or proper handling |
| `Log.d` / `Log.i` of tainted types | Privacy (P2) | Custom logger with redaction |
| `LiveData` | Replaced by Flow | `StateFlow` |
| `RxJava` | Replaced by coroutines | Coroutines + Flow |
| `GlobalScope.launch` | Lifecycle leaks | Scoped to component lifecycle |
| Hardcoded sp values for text | Breaks font scaling | MaterialTheme typography |
| Hardcoded strings in Composables | i18n (P8) | `stringResource(R.string.x)` |
| Singletons outside DI graph | Untestable | Hilt-managed |
| `Activity.findViewById` | View system, not Compose | Compose all the way |
| `runBlocking` outside tests | Blocks UI thread | `suspend` + coroutines |

---

## TypeScript (Admin Web)

| Pattern | Why forbidden | Fix |
|---|---|---|
| `any` | Defeats type safety | `unknown` + narrow |
| `as` type casts | Hides errors | Validate with Zod |
| `// @ts-ignore` | Hides errors | Fix the type |
| `console.log` in committed code | Pollutes prod logs | `logger.info` |
| `localStorage` for PII | Unencrypted, JS-readable | Server-side session |
| Inline styles (`style={...}`) | Defeats design tokens | Tailwind classes |
| `dangerouslySetInnerHTML` / `{@html}` | XSS risk | Sanitize via DOMPurify (with audit) |
| `Date.now()` for ordering | Clock skew | Server timestamps |
| Optimistic updates without rollback | Lies to user | Wait or show pending |
| Fetch without abort signal | Memory leak on unmount | `AbortController` |

---

## Cloudflare Workers / Durable Objects

| Pattern | Why forbidden | Fix |
|---|---|---|
| WebSocket without Hibernation API | Massive billing | Use `state.acceptWebSocket()` |
| Synchronous loops > 30s | CPU limit | Use Workflows |
| Plaintext PII in KV | Unencrypted at rest | Encrypt before put |
| Plaintext PII in R2 | Same | Encrypt before put |
| Plaintext PII in log lines | Privacy (I10) | Use scrubbed emitter |
| `console.log` of request bodies | May contain PII | Log structured fields only |
| Hardcoded secrets | Use Secrets Store | `env.SECRETS.get(...)` |
| Untyped `env` access | Runtime errors | Generate types from wrangler.toml |
| Subrequests to user-controlled URLs | SSRF risk | Allow-list domains |

---

## SwiftUI design

| Pattern | Why forbidden | Fix |
|---|---|---|
| Hardcoded colors | Breaks dark mode | Asset catalog or `ShapeStyle` semantics |
| `Color.red` etc. for status | Color-only signal | Add icon + text |
| Tap targets < 44pt | Accessibility | Increase frame or padding |
| `NavigationView` (deprecated) | Outdated | `NavigationStack` |
| `.opacity(0)` for visibility toggle | Still tappable | Conditional rendering |
| Async work in `body` | Performance | `.task { }` |

---

## Compose design

| Pattern | Why forbidden | Fix |
|---|---|---|
| Hardcoded `dp` values for padding > 32 | Inconsistency | Theme spacing tokens |
| Hardcoded colors | Breaks dark mode | `MaterialTheme.colorScheme` |
| `remember { mutableStateOf(viewModelState) }` | State desync | Collect from VM Flow |
| Side effects in composition | Leaks | `LaunchedEffect` / `SideEffect` |
| Tap targets < 48dp | Accessibility | `Modifier.minimumInteractiveComponentSize()` |

---

## Process forbidden patterns

| Pattern | Why forbidden | Fix |
|---|---|---|
| PR without a linked spec | Constitution P5 | Open the spec first |
| "While I was here" refactor | Scope creep | Separate PR + spec |
| Disabled test | Bug hidden | Fix the test or open a task |
| Skipping the `clarify` pass | Hidden ambiguity | Run `/clarify` |
| Implementing without `tasks.md` | No contract | Run `/speckit.tasks` |
| Self-merging | No review | Wait for review |
| Force-pushing to main | History destruction | Always merge via PR |

---

## How the reviewer subagent uses this file

The `reviewer` subagent reads this file before reviewing a diff. For each forbidden pattern with a regex-detectable form, it greps the diff. For pattern violations that require judgment, it reasons about the diff against the table.

Findings are returned as: `{path, line, pattern, suggested_fix, severity}`.
