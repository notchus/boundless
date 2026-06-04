# Manifest golden fixtures (ADR-0014, spec 001)

The signed server-driven-config manifest, in the four scenarios `core::crypto` must handle
at launch (AC10 / O2). Each file is a signature **envelope**: a `manifest` object (the
canonical signable content — frozen here in **T02**), plus `signature` and `public_key`.

## Signature vectors are produced in T03 (the crypto hand-off)

T02 freezes the manifest **content bytes**. The Ed25519 detached-signature vectors are a
crypto concern, so they are generated and committed by **T03** (`core::crypto`, `dryoc`),
whose `ac10_manifest_*` tests are the actual verifiers — keeping crypto single-sourced
(P4) rather than hand-faked here. Until then:

- `signature: null` / `public_key: null` ⇒ T03 fills these (a **valid** vector for
  `verify_ok`; not needed for the version-ignore case).
- A present `signature`/`public_key` (all-zero base64) is a deliberately **invalid**
  vector — it decodes to the right length (64-byte sig / 32-byte key) but does not verify.

## Scenarios → expected client behavior

| File | `manifest_version` | Signature | Expected outcome (ADR-0014 tiers) |
|---|---|---|---|
| `verify_ok.json` | 7 | `null` → **valid** (T03) | Signature verifies → **apply** this manifest. |
| `verify_fail_with_cache.json` | 8 | invalid (zeros) | Verify fails **and a cached manifest exists** → tier 2: keep using the **cached** manifest. Never blocks the primary surface (`ManifestFailReturning`). `MANIFEST_VERIFY_FAILED`. |
| `verify_fail_no_cache.json` | 1 | invalid (zeros) | Verify fails **and no cache exists** (true first launch) → tier 3: use the **bundled-in-binary** catalog. `MANIFEST_VERIFY_FAILED`. |
| `lower_version_ignored.json` | 2 | `null` (irrelevant) | Fetched `manifest_version` (2) **<** the cached version (test against a cached **7**) → **ignored** before any signature check. `MANIFEST_VERSION_STALE`. |

The `offline_first_launch` case in T03 (`ac10_manifest_offline_first_launch`) needs no
fixture file — it exercises the no-network path that falls back to the bundled catalog.

All manifest content is **non-PII** (ADR-0014): translation copy, design tokens, feature
flags, per-code error messages, the version fields, and the per-Group `admin_name` (a
first name, the source of `{adminName}` on the degradation screen — OQ7).
