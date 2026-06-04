# Boundless Operational Invariants

> These are the operational counterparts of `privacy-invariants.md`. Each is enforced by code, not comments. Numbering is permanent (separate from I1–I12 to avoid conflation).
>
> Together with P13, these define what "the rider never sees an update prompt" means concretely.

---

## O1 — Server is N-2 backward compatible

The server supports the current client minor version and the two previous minor versions, minimum.

**Enforcement:**
- The OpenAPI spec carries a `min_supported_client_version` field in the auth response.
- Integration tests in `server/tests/compat/` replay every supported minor's request fixtures against the current server build; new build fails CI if any older supported version's tests fail.
- Removing support for a version requires an ADR.

---

## O2 — Soft content is server-driven via signed KV manifest

Translations, user-facing copy, design tokens (the subset that affects layout, not branding), feature flags, and per-locale error messages live in Cloudflare KV. The client fetches a signed manifest on launch, verifies the signature with a public key bundled in the app, caches the result, and applies without restart.

**Enforcement:**
- The manifest is signed with libsodium detached signatures using a key in Cloudflare Secrets Store.
- The client verifies the signature client-side before applying.
- A property test asserts: server-rotates-content → client-launches → client-reflects-content within one launch cycle.
- See ADR-0014 for what is and isn't in KV.

---

## O3 — Auto-update is enabled at onboarding by an admin or family member

The first-launch flow includes a "set up this phone" step performed in person by someone other than the rider (admin, family member, or anyone the rider trusts). One step of that flow: enabling App Store / Play Store automatic app updates.

**Enforcement:**
- The onboarding spec (`specs/001-onboarding/`) includes this step as an acceptance criterion.
- A snapshot test of the onboarding flow includes a screen labeled "auto-update enabled."
- The rider's settings UI does not surface an "automatic updates" toggle — that is in the OS settings, not in Boundless.

---

## O4 — `client_min_version` returns from every auth response

Every `/api/auth/*` response and every WebSocket open handshake includes a `client_min_version` field. If the requesting client's reported version is below that threshold, the response degrades gracefully.

**Enforcement:**
- OpenAPI schema includes `client_min_version` as a required field in all auth responses.
- Client behavior when below threshold: show one calm screen with the locale's equivalent of "This device needs help — Sarah has been told." No mid-flow interrupts. No "Update Now" button (because the rider can't action it).
- Server emits an admin alert via Queues when a below-threshold request arrives (rate-limited per member per day).

---

## O5 — Admin dashboard shows version distribution

The admin surface includes a "Devices" panel showing the version count distribution across the group, with stragglers (members below the recommended version) surfaced as actionable items.

**Enforcement:**
- The data source is Analytics Engine (non-PII — only version strings, not who has which device).
- A spec for this view (suggested `specs/012-admin-device-versions/`) materializes the screen.
- Audit log records when an admin views the panel.

---

## O6 — Matching never depends on the client version

The matching engine in the Durable Object computes a chain regardless of which client versions are connected. If a client is too old to display the result, the result is still computed, the driver is still notified, and the gathering still happens.

**Enforcement:**
- The matching function signature does not accept client version as input.
- A property test asserts: matching output is deterministic across client-version permutations of identical (Rider, Driver, Address) inputs.

---

## O7 — Manual fallback is always exportable

The admin can export a current phone list of the group (PDF + plain text) with one click. This is the disaster-recovery fallback if the entire app fleet is unreachable.

**Enforcement:**
- The endpoint `/api/admin/export/phone-list` exists from v1.0.
- A scheduled test exports the list weekly to a known location to verify it remains functional.
- The export is audit-logged (per I5).

---

## O8 — No update prompt reaches the rider's primary surface

The Rider app's primary screen (the "You're coming tonight" surface) never displays an update prompt, update banner, or update modal — under any condition.

**Enforcement:**
- A snapshot test of the primary screen at all four a11y variants asserts no update-related text or controls present.
- The reviewer subagent treats any new UI element on the rider primary surface that references "update," "version," "new," or equivalents as a critical finding.
- If a forced update is genuinely necessary (e.g. a critical security fix), the rider sees only the O4 graceful-degradation screen, not an update CTA.

---

## Cross-references

- **Constitution principle:** P13.
- **Strategy doc:** `docs/update-strategy.md` (the ladder: KV → auto-update → admin-assisted → manual).
- **Architecture:** see `docs/architecture.md` — KV usage section.
- **ADRs:** ADR-0014 (server-driven config), future ADRs for any policy change.

---

## Updating this file

- Adding an invariant requires the implementing test in the same PR.
- Removing or weakening an invariant requires an ADR.
- Numbering is permanent; deprecated invariants are marked `**DEPRECATED**` but never deleted.
