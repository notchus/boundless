# Boundless Voice and Tone

> The product's voice is what makes it work. Get this wrong and the rest of the architecture is decorative. Every user-visible string ships through this filter.

---

## The voice in one paragraph

Warm, certain, quietly competent. Speaks like a thoughtful friend who has organized things, not like an app. Never gives the user homework. Never asks them to confirm what they already know. Never apologizes when nothing went wrong. Never explains itself when silence would do.

---

## The five voice qualities

### 1. Calm
The app is the opposite of urgent. Even when conveying timing, it does so softly: "~ 6:12 PM," not "Pickup in 7 MINUTES!!"

### 2. Certain
Avoid hedges. The default state is declarative: "You're coming tonight." Not "It looks like you might be coming tonight."

### 3. Personal but not over-familiar
"Hello, Maria." — yes. "Hey Maria 👋" — no. The product calls people by name; it does not perform friendliness.

### 4. Anti-administrative
Avoid form-language. Not "Submit your availability for this evening's transportation event." → "I have a seat tonight."

### 5. Honest about what's small
When something fails or is unknown, say so simply. Not "Oops! Something went wrong, please try again." → "No driver tonight. Sarah has been told."

---

## Do / Don't (grounded in the prototype)

| Don't | Do | Why |
|---|---|---|
| "Are you attending tonight?" (yes/no) | "You're coming tonight." (with a quiet "Can't make it tonight" link) | Riders are in by default. Asking inverts the model. |
| "Driver assigned: Daniel" | "Daniel will be at your door in about 7 minutes." | Lead with the human outcome, not the system event. |
| "Estimated arrival: 18:12" | "Around · 6:12 PM" | "Around" replaces "estimated"; 12-hour with locale default. |
| "Your driver has changed due to a scheduling conflict." | (silence — the new driver introduces themselves in person) | Per ADR-007 silent reassignment. |
| "No drivers available tonight, sorry." | "No driver tonight. Sarah has been told." | Names the admin; removes apologetic register. |
| "Submit availability" | "I have a seat tonight" | First-person voice, action-oriented. |
| "Pickup time" | "Around · 6:12 PM" | The label is the value. |
| "Tap here to opt out of tonight's event" | "Can't make it tonight" | Plain language, no app jargon. |
| "Welcome back!" | "Hello, Maria." | Named, not generic. |
| "Notifications enabled successfully ✓" | (silence — just show the next screen) | No celebration of plumbing. |

---

## Microcopy rules

1. **Sentence case, not Title Case.** "Can't make it tonight" not "Can't Make It Tonight".
2. **No exclamation marks.** Anywhere.
3. **No emoji in primary text.** A checkmark icon is OK; "✨ You're all set ✨" is not.
4. **Periods at end of full sentences.** "Daniel will be at your door in about 7 minutes."
5. **No periods at end of UI labels that aren't sentences.** "Can't make it tonight"
6. **Numbers spelled out under ten, except in time/distance.** "about 7 minutes" → kept as numeral because it's a quantity.
7. **Times: locale-aware.** "6:12 PM" in en-US, "18:12" in de-DE.
8. **Approximate times use the word "around" or its locale equivalent.** Never "approximately" — too clinical.
9. **The Driver's name appears; the Rider's address never does** (on the Driver's screen until in-neighborhood).

---

## Voice in error states

Errors are the highest-stakes voice moment. They reach a person who is already confused.

### Bad
> "Error: Network request failed (HTTP 502)"

### Good
> "Couldn't reach Boundless just now. Trying again."

### Bad
> "Authentication failure. Please log in again."

### Good
> "Let's sign in again. Your phone number works."

### Bad
> "Invalid input. Please check and retry."

### Good
> "That number doesn't match what's on file. Try again or call Sarah."

---

## Voice in critical states

When something the user actually needs to know fails, say it with full clarity and a path forward.

### Doorbell notification
> Daniel is at your door.

### No driver tonight
> No driver tonight. Sarah has been told.

### Driver canceled, no reassignment yet
> Working on a new driver. Sarah's looking too.

### App down (maintenance)
> Boundless is being updated. Back in a few minutes.

---

## Translation principles

This product ships in many languages (English, Spanish, German, Swiss German, Russian, Polish, Arabic, Hebrew, more). Translation is not literal — it preserves the voice.

1. **Translate the feeling, not the words.** "You're coming tonight" → in German, the same warmth, not the same syntax.
2. **Swiss German is a separate locale (`gsw`).** It is not "German with funny words" — it has its own grammar and pronouns.
3. **RTL is not just a layout flip.** Numbers, punctuation, and time formatting all change in Arabic and Hebrew.
4. **Pseudo-locale (`zz-ZZ`) builds catch hardcoded strings.** Every screen must render in pseudo-locale without breaking.
5. **Glossary terms are catalog keys, not phrases.** Translators see "the gathering" with context, not a literal sentence to translate.

See `docs/i18n.md` for the technical pipeline.

---

## When in doubt

Ask: **"Would Maria's grandson, who built this, want her to hear this exact sentence?"**

If no, rewrite.
