# Runbook: deploy the admin web dashboard (`boundless-admin-web`)

> The first deploy of the SvelteKit **admin dashboard** (`web/`) to Cloudflare Workers, wired
> end-to-end to the already-deployed Rust admin Worker (spec 009). After this, a real Admin (Sarah)
> opens a Developer-issued onboarding link at a real URL, registers a passkey, signs in, and manages
> members against live Neon Postgres — her session, passkey, and invite persisted across cold starts.
>
> **Under Option B1 (ADR-0027) the web tier holds ZERO Postgres.** WebAuthn invite/credential
> persistence and the member roster live behind the **Rust admin Worker's** endpoints, reached over the
> ADR-0026 BFF shared secret `ADMIN_API_SECRET`. The admin **session** lives in a Cloudflare **KV**
> namespace (`ADMIN_SESSIONS`); the WebAuthn **challenge** store in `CHALLENGES` KV. So this deploy is
> two KV namespaces + one shared secret + some config — no database of its own.
>
> Everything that can be prepared without an account already is — built and tested locally. What is left
> is **your** part: the `wrangler` commands (which mutate your Cloudflare account — the human gate; the
> agent never runs them) plus the operator first-admin seed.

---

## Before you start

- **The Rust admin Worker is already deployed** (`docs/runbooks/deploy-worker.md`) and you have, **saved**:
  - its **URL** (e.g. `https://boundless-worker.<account>.workers.dev`) — this is `ADMIN_WORKER_BASE`;
  - the **`ADMIN_API_SECRET`** you `wrangler secret put` on it (step 4c) — you cannot read it back, so use
    the value you saved;
  - the **`HMAC_KEY`** you set on it (step 4) — the operator seed below needs it, and you cannot read it
    back either.
- **The Group is bootstrapped** (`bootstrap-group.sh`, deploy-worker.md step 5b) — the seed below needs
  the Group's `groups` row to exist (admins carry no encrypted PII, so the per-Group *key* is not needed
  for the seed, but the `groups` row is the FK target).
- **A Cloudflare account**, and `wrangler` (pinned `4.98.0` in `web/package.json`). Run it via `npx
  wrangler` from `web/`. Authenticate once: `cd web && npx wrangler login` (or `CLOUDFLARE_API_TOKEN`).
- **`psql` + `openssl`** on your PATH (the operator seed shells out to both).

### The two footguns (read these first)

1. **`ADMIN_API_SECRET` must be byte-identical on both Workers.** This web Worker presents it to the Rust
   Worker on every `/api/admin/*` call (ADR-0026). A mismatch surfaces as the opaque **`ADMIN_UNAUTHORIZED`
   (401)** — not an obvious error. You **cannot read a `wrangler secret` back** from Cloudflare, so set this
   web Worker's secret (step 3) to the exact value you saved when deploying the Rust Worker (deploy-worker.md
   step 4c). It is a `wrangler secret`, **never** a `[vars]` entry — the credential-scan gate
   (`scripts/check-wrangler-credentials.sh`, AC6) rejects any secret value committed in `web/wrangler.toml`.
2. **`RP_ID` is permanent.** WebAuthn passkeys are bound to the Relying-Party id (`WEBAUTHN_RP_ID`).
   Changing the domain after passkeys exist **invalidates every one of them** (D7). Decide your final host
   *before* enrolling real admins. For dev/test the first deploy uses the `*.workers.dev` host; the
   production end state is a custom admin domain — moving from one to the other is a documented
   passkey-reset (one Developer re-invite per admin, survivable per ADR-0016 D4). See
   [The custom-domain cutover](#the-custom-domain-rp_id-cutover-d7).

### Account-free pre-flight (recommended)

From `web/`, with **no account**:

```bash
cd web
pnpm install
pnpm typecheck && pnpm test        # the full unit/contract suite (incl. the AC5/AC6/AC15 gates)
pnpm build                         # the production build — see the NODE_ENV note below
```

> **Always build via `pnpm build`, never a bare `vite build`.** `pnpm build` is `NODE_ENV=production vite
> build`. SvelteKit inlines `$app/environment`'s `dev` from **`NODE_ENV`** (not Vite's `--mode`): under
> `NODE_ENV=production` the four dev-only `/api/test/*` seams collapse to an unconditional `error(404)` and
> are tree-shaken; under **any non-production `NODE_ENV` they ship LIVE** — an I11 bypass. The
> `NODE_ENV=production` pin in `package.json`'s build script is therefore load-bearing; the AC5 gate
> (`web/tests/build-gates/no-dev-seams.test.ts`) builds under a hostile `NODE_ENV=test` to prove it. Any
> deploy/CI path MUST keep the pin.

---

## The deploy

The order is deliberate: each `wrangler … create` prints an `id` you paste into `web/wrangler.toml`; the
final `wrangler deploy` reads that file.

### 1 — Create the two KV namespaces

```bash
cd web
npx wrangler kv namespace create CHALLENGES        # one-time-use WebAuthn challenges (~5-min TTL)
npx wrangler kv namespace create ADMIN_SESSIONS     # durable admin sessions (12h TTL, sign-out revoke)
```

Paste each printed `id` into `web/wrangler.toml`, replacing the matching placeholder:
`challenges_placeholder_replaced_at_deploy` and `admin_sessions_placeholder_replaced_at_deploy`. The
binding **names** (`CHALLENGES`, `ADMIN_SESSIONS`) must not change — `App.Platform`, `KvChallengeStore`,
and `KvSessionStore` depend on them, and the AC15 drift test
(`web/tests/build-gates/wrangler-types-match.test.ts`) fails the build if a binding is renamed without
updating `src/app.d.ts`.

### 2 — Fill the `[vars]` host values

In `web/wrangler.toml`, replace the `REPLACE_AT_DEPLOY` placeholders in `[vars]`:

- `ADMIN_WORKER_BASE` → the deployed **Rust** Worker origin (from deploy-worker.md), e.g.
  `https://boundless-worker.<account>.workers.dev`.
- `WEBAUTHN_RP_ID` / `WEBAUTHN_ORIGIN` → **this admin-web** host. For the dev/test deploy that is its
  `*.workers.dev` host (`boundless-admin-web.<account>.workers.dev` / `https://…`). `WEBAUTHN_RP_NAME` is
  the human label (`Boundless`).

These reach the deployed Worker via `$env/dynamic/private`; they are **not** secrets (no secret may live
in `[vars]` — AC6). `RP_ID`/`ORIGIN` are fully env-driven so the custom-domain cutover is a config change,
not a code change (D7). **Remember RP_ID is permanent** (footgun 2).

### 3 — Set the BFF shared secret

```bash
npx wrangler secret put ADMIN_API_SECRET    # paste the EXACT value from deploy-worker.md step 4c
```

It must be **byte-identical** to the Rust Worker's `ADMIN_API_SECRET` (footgun 1). Never commit it.

### 4 — Seed the first admin (the operator seed)

A passkey onboarding needs a Developer-authorized **pending-admin** row (role `admin`, **no PII**) and a
single-use, TTL-bounded **invitation**. Minting these via Developer WebAuthn is out of scope (spec 001
T08-shell); until then this operator seed stands in (it preserves I11/ADR-0015 — only the Developer
initiates admin access; the seed carries no PII and no credential material). Run it once, as the database
**owner** (its `BYPASSRLS` lands the insert under FORCE RLS), with the Worker's `HMAC_KEY` so the token it
mints resolves on the deployed Worker:

```bash
# From the repo root. The owner URL + HMAC key ride in the ENV (never an argv → not in `ps`).
SEED_OWNER_URL="postgresql://neondb_owner:PW@HOST/neondb?sslmode=require" \
SEED_HMAC_KEY_HEX="<the HMAC_KEY you saved in deploy-worker.md step 4>" \
SEED_ADMIN_WEB_BASE="https://boundless-admin-web.<account>.workers.dev" \
  bash scripts/seed-admin-invite.sh 00000000-0000-0000-0000-000000000001
```

`<group-id>` is this install's `GROUP_ID` (deploy-worker.md step 5). It prints — **only to stdout** — the
new `admin_id`, the single-use `token`, and a ready-to-send `onboard_url`
(`https://…/admin/onboard/<token>`). The token rides in that URL; it is single-use (consumed on Sarah's
first successful passkey registration) and TTL-bounded (default **72h**; override with
`SEED_INVITE_TTL_SECS`). **Hand the `onboard_url` to the admin out of band.** The token never appears in a
log line (R20); the seed's progress goes to stderr, redacted.

> **Re-issue a lost/expired link** (no second admin): pass the printed `admin_id` as a second argument —
> `bash scripts/seed-admin-invite.sh <group-id> <admin-id>` — and the seed atomically supersedes the prior
> invitation and mints a fresh one (the `one_live_per_admin` invariant is upheld).
>
> **`created_by` is left NULL.** It is meant to name the Developer who authorized the admin (I11 audit
> actor), but there is no Developer WebAuthn identity to attribute it to until the minting UI lands — a
> documented audit-actor gap (DEFERRED spec 009 T10 → R18/F5), not a silent omission.

### 5 — Build and deploy

The committed `web/wrangler.toml` has no deploy-target keys yet (it is built in the adapter's default
mode locally, with no account). To deploy to **Workers**, `@sveltejs/adapter-cloudflare` emits
`.svelte-kit/cloudflare/_worker.js` + assets, so add (at deploy) the entry + static-assets keys:

```toml
# web/wrangler.toml — add for the Workers deploy:
main = ".svelte-kit/cloudflare/_worker.js"

[assets]
directory = ".svelte-kit/cloudflare"
binding = "ASSETS"
```

Then, from `web/`:

```bash
pnpm build            # NODE_ENV=production (the load-bearing seam-stripping pin — see the pre-flight note)
npx wrangler deploy   # reads web/wrangler.toml; uploads the Worker + assets
```

Note the printed URL (e.g. `https://boundless-admin-web.<account>.workers.dev`). It must match the
`WEBAUTHN_ORIGIN`/`RP_ID` host you set in step 2 (footgun 2).

> **Adding the `ASSETS` binding** introduces a non-KV binding. Extend `src/app.d.ts` + the AC15 drift-test
> classifier (`wrangler-types-match.test.ts`) to cover it, or the build gate may flag the drift (the
> classifier currently scopes to KV bindings — DEFERRED spec 009 T09). Do not commit real namespace ids /
> account hosts into the repo before open-sourcing (genericize back to `REPLACE_AT_DEPLOY_*`).

### 6 — Smoke the deployed dashboard

Always-on checks (no seed needed) — reachable + fail-closed gate + `Referrer-Policy: no-referrer` on the
invite route + `RP_ID` not `localhost` (HTTPS) + every `/api/test/*` seam returning 404:

```bash
bash scripts/smoke-deployed-admin-web.sh https://boundless-admin-web.<account>.workers.dev
```

The full AC10 passkey flow (a seeded invite → passkey registration → sign-in → the live roster → issue one
member → sign-out → the revoked cookie returns the `/admin/signin` redirect). It can't be curl'd, so the
script shells out to a Chromium virtual-authenticator Playwright leg — opt in with a **fresh**
`seed-admin-invite.sh` token (the registration consumes it; re-seed for a re-run):

```bash
# mint a fresh invite (step 4) → export its `token=` → run the ceremony:
SMOKE_INVITE_TOKEN="<fresh token from step 4>" DEPLOYED_CEREMONY=1 \
  bash scripts/smoke-deployed-admin-web.sh https://boundless-admin-web.<account>.workers.dev
```

> The ceremony **issues one test member** (a `Smoke <timestamp>` rider) and there is no member-delete yet
> — ignore/rename it (cleanup arrives with the I12 deletion spec).

Cross-tenant isolation (AC14 edge leg, opt-in) — prove a token seeded in a **second** Group is invisible
(resolves 410, like a never-issued token): set `CROSS_TENANT_INVITE_TOKEN=<a token seeded in Group B>`.

---

## The custom-domain RP_ID cutover (D7)

The production end state is a **custom admin domain**. Moving from the `*.workers.dev` host to it is a
config change (`WEBAUTHN_RP_ID`/`WEBAUTHN_ORIGIN` in `[vars]`, plus the domain routing in Cloudflare), not
a code change — that is the whole point of keeping RP config env-driven. But because **RP_ID is
permanent**, every passkey enrolled on the old host stops working after the cutover. The remedy is the
ordinary recovery path: re-run the operator seed (step 4) once per admin to issue a fresh onboarding link,
and each admin re-registers a passkey on the new host (ADR-0016 D4). Do the cutover **before** enrolling
real admins if you can; if not, schedule it and re-invite.

---

## Troubleshooting

- **`ADMIN_UNAUTHORIZED` (401) on every member action** — the web Worker's `ADMIN_API_SECRET` does not
  match the Rust Worker's. Re-set it (step 3) to the exact saved value (footgun 1).
- **`ADMIN_GROUP_KEY_MISSING` (503) / a calm "couldn't reach the admin service"** — the Group is not
  bootstrapped on the Rust Worker. Run `bootstrap-group.sh` (deploy-worker.md step 5b). The dashboard
  fails **closed** here — it never serves a fake roster (AC1).
- **The onboarding link 404s or "expired"** — the invite is single-use and TTL-bounded; if it was used or
  has expired, re-issue with `seed-admin-invite.sh <group-id> <admin-id>` (step 4).
- **Passkeys suddenly all fail after a domain change** — RP_ID changed (footgun 2 / the cutover). Re-invite.

---

## References

- ADR-0026 — admin Worker shared-secret BFF trust (`ADMIN_API_SECRET`).
- ADR-0027 — Option B1: WebAuthn invite/credential persistence behind the Rust Worker (web tier = zero
  Postgres).
- ADR-0016 — auth model (admin sessions separate + shorter-lived; passkey-reset on RP change is survivable).
- ADR-0017 — admin auth via edge WebAuthn (ceremony in edge-TS; invite-token hash match in the core).
- `docs/runbooks/deploy-worker.md` — the Rust Worker deploy this one depends on (`ADMIN_API_SECRET`,
  `HMAC_KEY`, the Group bootstrap).
- Spec + plan: `specs/009-admin-web-deploy/{spec,plan,tasks}.md` (decisions D1–D8).
- Cloudflare / SvelteKit docs: [adapter-cloudflare](https://svelte.dev/docs/kit/adapter-cloudflare) ·
  [Workers static assets](https://developers.cloudflare.com/workers/static-assets/) ·
  [wrangler config](https://developers.cloudflare.com/workers/wrangler/configuration/) ·
  [KV commands](https://developers.cloudflare.com/kv/reference/kv-commands/) ·
  [secrets](https://developers.cloudflare.com/workers/configuration/secrets/).
