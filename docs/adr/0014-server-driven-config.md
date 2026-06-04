# ADR-0014: Server-Driven Configuration via Cloudflare KV

- **Status:** Accepted
- **Date:** 2026-05-27
- **Author:** Boundless founder
- **Deciders:** Boundless founder

## Context

The rider population is primarily elderly. Many cannot complete an "Update Required" flow without help — and worse, an update prompt on the primary screen breaks the only promise the app makes ("you're coming tonight"). Every change we'd otherwise ship as a native binary update is a friction event that reaches a 78-year-old who didn't ask for it.

But we still need to change things. Translations get refined. Copy gets warmer. Layout subtleties get fixed. New feature flags get tested per-group. Error messages get clearer. These are constant — daily during early development, weekly during steady-state. If every one of them required an App Store update, we'd either ship rarely (frustrating contributors) or ship often (frustrating riders).

There needs to be a path that lets us iterate freely on *content and behavior* without forcing a binary update. Constitution P13 ("updates are nearly invisible") makes this an architectural requirement.

## Decision

Adopt **server-driven configuration via a signed Cloudflare KV manifest**. The client reads a manifest at launch, verifies its signature, caches it, and applies it without restart.

### What's in the manifest

- **Translation catalogs** for all locales (the canonical source — the client's bundled catalog is only a fallback for offline first-launch).
- **User-facing copy** as ICU MessageFormat keys (every key resolvable, including new keys added between binary releases).
- **Design tokens, subset:** colors, spacing scales, font scales that don't require asset changes. Branding-level changes (logo, primary palette) stay in the binary.
- **Feature flags:** per-Group enable/disable for experimental behaviors.
- **Error messages** per locale, per error code.
- **`client_min_version` and `client_recommended_version`** (see operational-invariant O4).
- **`manifest_version`** (monotonic integer; clients ignore manifests with lower version).

### What's NOT in the manifest (requires a binary update)

- New screens or screen structure.
- Native API usage changes.
- Embedded Rust core changes.
- Cryptographic primitive changes.
- Permission scope changes.
- Local storage schema changes.

### Signing

The manifest is signed with a libsodium detached signature (Ed25519). The signing key lives in Cloudflare Secrets Store, scoped to a single Worker that mints manifests. The verification key (public) is embedded in the app binary.

The client verifies signature on every fetch, refuses to apply unsigned or tampered manifests, falls back to the previously-cached manifest (or the bundled one) if verification fails.

### Distribution

- Manifest is stored as a single KV key per locale: `manifest:v1:<locale>`.
- A "manifest index" KV key (`manifest:v1:index`) lists current manifest versions per locale, fetched once by the client to know what to download.
- Workers cache headers set so Cloudflare's edge cache holds manifests at the colo for ~5 minutes; stale-while-revalidate for 24 hours.

### Client behavior

- On launch: read cached manifest from local storage; show UI immediately.
- In parallel: fetch the manifest index; if newer than cached, fetch the per-locale manifest.
- On successful verify: apply; UI re-renders affected strings/tokens.
- On failure: keep using cached manifest; log to OTel; do not surface anything to the rider.

## Considered alternatives

### Option A — All config baked into the binary

Ship copy, translations, tokens, error messages with the app. Each change requires a new build.

**Pros:**
- Simplest model.
- No network dependency for content.
- No signature/verification machinery.

**Cons:**
- Every translation tweak requires an App Store + Play Store release.
- A Weblate translator update can't reach users for days or weeks.
- Per-group feature flags become impossible without a binary release.
- Violates P13 (rider faces updates frequently).

### Option B — Remote Config from a third party (Firebase Remote Config, LaunchDarkly, etc.)

Use a managed remote-config service.

**Pros:**
- Mature tooling.
- Polished dashboards.

**Cons:**
- Third-party dependency (violates I8 unless self-hosted alternative; LaunchDarkly is paid; Firebase Remote Config implies the Firebase SDK which has tracking).
- Vendor lock-in.
- Privacy story is murky on most of these.
- Cloudflare KV already covers our needs at near-zero cost.

### Option C — Dynamic native code modules (JS bundles, Kotlin scripts)

Ship updated *logic*, not just *config*, via a downloaded module.

**Pros:**
- Maximum flexibility.

**Cons:**
- App Store policy forbids it on iOS (with narrow exceptions like JS for web views).
- Massive security surface — a compromised manifest could now execute arbitrary code.
- Complicates audit; harder to reason about behavior.
- Not needed — config-level changes cover ~90% of update needs.

### Option D (chosen) — Signed KV manifest

**Pros:**
- Covers the change classes that actually drive most updates (translation, copy, tokens, flags, error messages).
- First-party Cloudflare; no third party.
- Signed → cannot be tampered with by a network attacker or a compromised non-signing Cloudflare scope.
- Cheap (KV is high-RPS-read, low-write — perfect fit).
- Cacheable at the edge.
- Compatible with `client_min_version` mechanism — `client_min_version` itself lives in the manifest.

**Cons:**
- Verification machinery to build and audit (mitigated: libsodium is well-trodden).
- Manifest schema becomes a first-class API surface — breaking changes need their own versioning (`manifest:v2:*`).
- Bricks-if-misconfigured risk: a bad manifest could break all clients. Mitigation: staged rollout (per-Group), automatic rollback on client-error spike via Worker logic.

## Consequences

### Positive

- **Translation updates ship in seconds.** A Weblate sync → KV write → users see the new strings next launch.
- **Per-Group rollouts** become trivial. New behavior for `group_id = abc` only, tested with a single group before broad rollout.
- **`client_min_version` lives in KV.** Bumping it is a single `wrangler` command, fully auditable in Cloudflare logs.
- **Rider sees almost no native updates.** Most changes go through KV and are invisible.
- **Translator workflow stays decoupled** from release engineering.

### Negative / costs

- **Signing infrastructure to build.** ~1 week of focused work. Includes the Worker that mints manifests, the Secrets Store integration, and the client-side verification on every platform.
- **Manifest schema versioning.** Breaking schema changes need a coordinated rollout: ship a client that understands both `v1` and `v2`; only then start writing `v2`; eventually deprecate `v1`. Plan ADR-level migrations for these.
- **Edge cases for offline launches.** If a rider's first launch is offline, the bundled-in-binary catalog is the only source. This catalog can drift from KV over time. Mitigation: client checks bundled catalog version on every online launch; if older than configured threshold, prompts the admin (not the rider) to help refresh.
- **Manifest poisoning blast radius.** A compromised signing key would push to every client. Mitigation: signing key in Secrets Store, accessible only to the manifest-mint Worker; quarterly key rotation; emergency revocation = ship a binary that rejects the old key (yes, this is one case where a native update *is* the rescue path — and it's worth the risk).

### Neutral / follow-ups

- The manifest endpoint is non-PII. It is served with `Cache-Control: public, max-age=300, stale-while-revalidate=86400`.
- The manifest endpoint should be subject to Cloudflare's standard DDoS protections.
- Telemetry on manifest fetches (non-PII: version applied, error counts) feeds the admin Devices panel (O5).

## Compliance

- **Constitution change:** Added P13 in the same ratification.
- **Stack matrix:** Add `dryoc` / `sodiumoxide` for signing on the server side; the same crate (compiled to platform binaries via UniFFI) does verification on every client — single crypto implementation across the system, matching P4.
- **Migration plan:** N/A — this is greenfield. The first client release will be `v1.0` with manifest `v1` support.

## References

- [Cloudflare KV documentation](https://developers.cloudflare.com/kv/)
- [libsodium signatures](https://doc.libsodium.org/public-key_cryptography/public-key_signatures)
- Constitution P13
- Operational invariants O1, O2, O4 (`docs/operational-invariants.md`)
- Update ladder (`docs/update-strategy.md`)
