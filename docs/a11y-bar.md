# Boundless Accessibility Bar

> This is the *floor*, not the ceiling. The product fails if it doesn't clear this. Every PR that touches user-visible code is tested against this bar in CI.

---

## The big picture

Boundless's primary user is a 75-year-old with cataracts, possibly arthritis, possibly tremors, possibly hard of hearing, who uses a smartphone with the largest text size enabled. **This person must be able to use the rider app without help.**

This means:
- No screen can rely on color alone.
- No tap target can be smaller than the minimum-touch standard.
- No interaction can require fine motor control.
- No information can be conveyed only by motion or sound.
- Text scales without breaking layout.
- The app is usable with VoiceOver / TalkBack / a screen reader from end to end.
- Layout works in RTL.

---

## Required snapshot variants (every screen)

Every screen produces snapshots in **four variants**, asserted in CI:

| Variant | What it tests |
|---|---|
| `default` | Baseline appearance |
| `largest-text` | Dynamic Type xxxLarge (iOS) / 200% scaling (Android) / 200% zoom (Web) |
| `dark-mode` | Dark appearance (per OS preference) |
| `rtl` | Right-to-left layout (Arabic / Hebrew) |

A screen is incomplete until all four snapshots exist and pass.

---

## iOS (SwiftUI)

### Mandatory

- **Dynamic Type support** via `.font(.title)` etc. — never hardcoded point sizes for body text.
- **Layout works at `accessibility5` (xxxLarge)** with no truncation of primary actions, no horizontal overflow.
- **VoiceOver labels** — every interactive element has a `.accessibilityLabel` if the visual label is ambiguous.
- **VoiceOver hints** — for non-obvious interactions, `.accessibilityHint("Double-tap to mark you can't make it tonight")`.
- **Custom actions** — group related operations using `.accessibilityAction(.named:)`.
- **Switch Control reachable** — every interactive element is focusable.
- **Touch targets ≥ 44pt × 44pt** (Apple's HIG minimum).
- **Color contrast ≥ 4.5:1** for body text, ≥ 3:1 for large text and UI controls (WCAG 2.2 AA).
- **Reduce Motion respected** — use `.accessibilityReduceMotion` env value.
- **Reduce Transparency respected** — opaque backgrounds when set.
- **Bold Text respected** — use `.accessibilityBoldText`.
- **Differentiate Without Color** — icons or text accompany color signals.
- **No flashing content** above the photosensitive threshold.

### Tooling

- **Xcode Accessibility Inspector** run on every screen before merge.
- `swift-snapshot-testing` configured with the four variants.
- iOS UI tests use VoiceOver actions to navigate primary flows.

---

## Android (Jetpack Compose)

### Mandatory

- **TalkBack semantics complete** — every interactive Composable has `Modifier.semantics { contentDescription = ... }` or a clear text label.
- **Font scaling to 200%** — layouts adapt without truncation.
- **Touch targets ≥ 48dp × 48dp** (Material's minimum).
- **Color contrast ≥ 4.5:1** body, ≥ 3:1 large text and UI controls.
- **Switch Access reachable.**
- **Reduce Motion respected** via `LocalAccessibilityManager.current.isReduceMotionEnabled`.
- **Heading semantics** for screen titles (`Modifier.semantics { heading() }`).
- **Live regions** for state changes the user must hear (`liveRegion`).
- **Custom actions** for non-standard gestures.
- **No reliance on long-press** for primary actions.

### Tooling

- **Android Accessibility Scanner** run on every screen.
- **Paparazzi** snapshot tests with the four variants.
- Espresso tests with TalkBack enabled for primary flows.

---

## Wear OS

### Mandatory

- All content reachable within **two swipes / one tap** from the watch face.
- Text always at sizes designed for glance reading.
- No nested navigation more than 2 levels deep.
- Complications follow Wear's complication design guidance for legibility at a distance.
- Rotary input supported on round devices.

---

## watchOS

### Mandatory

- All content reachable within **the same constraints as Wear**, adapted to Apple's idioms.
- Smart Stack widget for the rider's "around 6:12 PM" surface.
- Complications: corner, inline, circular, rectangular — all tested.
- Crown rotation supported for scrolling.

---

## Web (Admin)

### Mandatory (WCAG 2.2 AA minimum, AAA where reasonable)

- **Keyboard-complete** — every action reachable and operable via keyboard.
- **Visible focus rings** — never `outline: none` without a replacement.
- **ARIA roles** correct (use semantic HTML first, ARIA second).
- **Form labels** programmatically associated with inputs (`<label for>` or `aria-labelledby`).
- **Error messages** programmatically associated with the field (`aria-describedby`).
- **Tab order** logical.
- **Zoom to 400% without horizontal scroll** (WCAG 1.4.10 Reflow).
- **Skip links** for primary navigation.
- **Live regions** (`aria-live`) for async status changes.
- **Color contrast** measured by axe-core in CI.
- **Reduced motion** via `prefers-reduced-motion`.
- **Screen reader tested** — NVDA on Windows, VoiceOver on macOS, JAWS optional.

### Tooling

- **axe-core** in Playwright tests; CI fails on any violation.
- **Lighthouse** accessibility score ≥ 95 (advisory, not blocking).
- **Manual screen reader test** before each release.

---

## What is forbidden everywhere

- Color as the only signal (red error without text or icon — forbidden).
- Hover-only affordances (touch users need tap, not hover).
- Time-based dismissals (toasts that disappear before a screen reader announces them).
- CAPTCHAs that require visual perception only (Turnstile is OK; image-puzzle CAPTCHAs are not).
- Auto-playing audio or video.
- Long-press as the only path to a function.
- Drag-and-drop as the only path to a function.
- Animations that flash > 3 times per second.
- Text inside images (use real text).
- "Mobile" sites at desktop sizes (responsive single source).

---

## Persona acceptance test

For the rider primary flow, a release passes when:

1. With **largest text + dark mode + VoiceOver**, Maria can verify "I'm coming tonight" in ≤ 3 swipes from app launch.
2. With **switch control alone**, she can tap "Can't make it tonight" in ≤ 5 switch presses.
3. With **screen reader on**, every state transition is announced.
4. In **RTL with Arabic locale**, every screen is correctly mirrored.

---

## What "passes a11y review" means

A subagent (`a11y-reviewer`, future addition) or a manual review must confirm:

- All four snapshot variants render.
- VoiceOver / TalkBack walkthrough recorded and inspected.
- Color contrast verified.
- Keyboard navigation (where applicable) verified.
- The persona acceptance test (above) passes.

A PR that touches a screen without showing this evidence is incomplete.
