# Runbook: deploy the edge Worker (`boundless-worker`)

> The first deploy of the member-auth Cloudflare Worker — the `#[event(fetch)]` router + the `GroupHub`
> Durable Object, talking to Neon Postgres over Hyperdrive (spec 001 **T07-shell-B**). Everything that
> can be prepared without a Cloudflare account already is — scripted and tested against a local Postgres.
> What is left is **your** part: the `wrangler` commands (which mutate your Cloudflare account and so are
> the human gate — the agent never runs them) plus one database-provisioning script.
>
> The order below is deliberate. Each `wrangler … create` prints an `id` you paste into
> `server/wrangler.toml`; the final `wrangler deploy` reads that file.

---

## Before you start

- **A Cloudflare account.** Cloudflare **Queues** (the `ADMIN_ALERTS` fan-out, §10-E) is available on
  the **Workers Free** plan (10,000 operations/day, 24h retention) — sufficient for a first deploy —
  with higher limits on Paid ([pricing](https://developers.cloudflare.com/workers/platform/pricing/);
  re-check at deploy time, plan gating changes). If you'd rather not provision a queue at all, you can
  temporarily comment out the `[[queues.producers]]` block in `server/wrangler.toml` (the below-min
  admin alert is non-critical for a first deploy).
- **A Neon Postgres database** and its **owner** connection string (Neon's `neondb_owner`). You will
  *not* deploy with that role — it is a `neon_superuser` member with `BYPASSRLS`, which the Worker's
  boot guard correctly **rejects** (see [Why the app role](#why-a-dedicated-app-role)). Step 0 mints a
  dedicated locked-down role.
- **A local Postgres client (`psql`)** and **`openssl`** on your PATH — step 0 shells out to `psql`
  (against the Neon owner URL) and `openssl rand`, and step 4 uses `openssl rand`. (No `psql`? You can
  run step 0's SQL via the Neon console instead, or set `PSQL=` to a containerized client.)
- `wrangler` is pinned in `server/package.json` (4.98.0); run it via `npx wrangler` from `server/`.
- Authenticate once: `cd server && npx wrangler login` (or set `CLOUDFLARE_API_TOKEN`).

### Account-free pre-flight (optional, recommended)

Validate the deploy bundle + bindings with **no account**, from `server/`:

```bash
cd server && npx wrangler deploy --dry-run --outdir dist
```

It runs the `worker-build` step and resolves every binding (you'll see `GROUP_HUB`, `MANIFEST`,
`ADMIN_ALERTS`, `HYPERDRIVE`, `GROUP_ID`), then exits without uploading.

You can also run the **whole** auth path locally against a real Postgres, no account, via `wrangler dev`
(it honors the `[[hyperdrive]] localConnectionString`):

```bash
# 1. a local PG with the schema (the test container is fine):
PSQL="docker exec -i boundless-postgres psql" \
  WORKER_TEST_SUPERUSER_URL=postgres://postgres:postgres@localhost:5432/boundless_test \
  bash scripts/setup-worker-test-db.sh
# 2. a LOCAL-ONLY secret (git-ignored — never committed; deploy uses `wrangler secret`):
printf 'HMAC_KEY=%s\n' "$(openssl rand -hex 32)" > server/.dev.vars
# 3. run it + smoke it (the smoke retries /healthz, so it tolerates wrangler dev still booting):
cd server && npx wrangler dev --port 8787 &      # wait for "Ready on http://127.0.0.1:8787"
bash ../scripts/smoke-deployed-edge.sh http://localhost:8787
```

> `server/.dev.vars` holds a local secret. It is git-ignored; confirm with
> `git check-ignore server/.dev.vars` before you ever `git add`.

---

## The deploy

### 0 — Provision the Neon database (the one non-`wrangler` step)

Run once, as the database **owner**, against an existing (empty) Neon database. The script is
**non-destructive and idempotent** — it creates the locked-down `boundless_app` role, applies the
schema migrations only if the database is empty, grants least privilege, and prints the **app-role
connection string** you need for step 1.

```bash
bash scripts/provision-neon.sh "postgresql://neondb_owner:PW@HOST/neondb?sslmode=require"
```

The last line of **stdout** is the app-role connection string (a credential), e.g.:

```
postgresql://boundless_app:<generated-password>@HOST/neondb
```

Copy it. (To capture just that line: `bash scripts/provision-neon.sh "$NEON_OWNER_URL" | tail -1`. To
pin the password instead of generating one, set a URL/SQL-safe `BOUNDLESS_APP_DB_PASSWORD` first —
`[A-Za-z0-9._~-]` only.) The string is intentionally **bare** (no `?sslmode=…`) — TLS is set by the
`--sslmode require` flag in the next step.

### 1 — Create the Hyperdrive config

```bash
cd server
npx wrangler hyperdrive create boundless-pg \
  --connection-string "postgresql://boundless_app:<password>@HOST/neondb" \
  --sslmode require
```

`--sslmode require` is how Hyperdrive does TLS to Neon (the documented `--connection-string` carries no
query — don't also put `?sslmode=` in it). It validates connectivity from Cloudflare's network at
create time (Neon allows this by default) and prints an `id`. Paste it into `server/wrangler.toml` →
`[[hyperdrive]] id` (replacing `REPLACE_AT_DEPLOY_hyperdrive_id`).

### 2 — Create the KV namespace

```bash
npx wrangler kv namespace create MANIFEST
```

Paste the printed `id` into `server/wrangler.toml` → `[[kv_namespaces]] id` (replacing
`REPLACE_AT_DEPLOY_kv_namespace_id`).

### 3 — Create the Queue

```bash
npx wrangler queues create boundless-admin-alerts
```

(On the Workers Free plan's Queues tier — see [Before you start](#before-you-start). To skip Queues
entirely on a first deploy, comment out the `[[queues.producers]]` block in `server/wrangler.toml`.)

### 4 — Set the HMAC secret

The per-instance I3 key. Generate a fresh 32-byte hex value and set it as a secret (never a `[vars]`
entry, never committed):

```bash
openssl rand -hex 32            # copy the output
npx wrangler secret put HMAC_KEY   # paste it at the interactive prompt
```

### 5 — `GROUP_ID`

Leave the `[vars]` default (`00000000-…-0001`) for now. It is an opaque tenant UUID, not a secret, and
there are no members until issuance (spec 008) assigns the real Group id — so the value does not affect
this deploy. Set the real id when issuance lands.

### 6 — Deploy

```bash
npx wrangler deploy
```

It runs the `[build]` (`worker-build --release`) and uploads. Note the printed URL, e.g.
`https://boundless-worker.<account>.workers.dev`.

### 7 — Smoke the deployed edge

```bash
bash ../scripts/smoke-deployed-edge.sh https://boundless-worker.<account>.workers.dev
```

It asserts `/healthz` ok, `/readyz` reports `db:"ok"` (the live proof that the Worker connects over
Hyperdrive and passes the least-privilege guard), a sign-in returns `AUTH_PHONE_NOT_ON_FILE`, and that
no response leaks a connection string.

> **What's still empty:** there are no Groups or members until issuance (spec 008). So a correct first
> deploy answers every sign-in with `AUTH_PHONE_NOT_ON_FILE` — exactly what the smoke checks. That is
> the Worker + transport + RLS working, not a bug.

---

## Why a dedicated app role

The Worker connects as `boundless_app` — **LOGIN, `NOSUPERUSER`, `NOBYPASSRLS`, non-table-owner**.
Row-level security (the per-tenant isolation behind every PII table) is *bypassed* by a superuser or a
`BYPASSRLS` role regardless of policy, so the Worker's boot guard
(`boundless_server_store::ensure_least_privilege`) refuses to serve if `current_user` is either —
fail-closed. Neon's default `neondb_owner` **is** such a role, which is why step 0 mints a separate one.
This is the highest-impact privacy control in the deploy (DEFERRED.md → T07-shell-B, sec-audit W2/R3).
`scripts/test-provision-neon.sh` proves, account-free, that the provisioned role is locked down and that
cross-tenant reads return zero rows.

After the first real deploy, run the cross-tenant check **as the live app role** against the deployed
edge — the production analog of that meta-test (still open in DEFERRED.md → T07-shell-B).

## References

- ADR-0019 — Worker → Postgres via `tokio-postgres` over a Hyperdrive Socket (not `sqlx`).
- ADR-0024 — the unnamed-statement `query_typed*` family on the Hyperdrive-pooled path (no driver fork).
- ADR-0021 — access-token wire format (the mint path is the next slice; sign-in mints nothing).
- Error codes: `docs/error-codes.md` (`AUTH_PHONE_NOT_ON_FILE`).
- Cloudflare docs: [wrangler commands](https://developers.cloudflare.com/workers/wrangler/commands/) ·
  [Hyperdrive wrangler commands](https://developers.cloudflare.com/hyperdrive/reference/wrangler-commands/) ·
  [KV commands](https://developers.cloudflare.com/kv/reference/kv-commands/) ·
  [Queues](https://developers.cloudflare.com/queues/get-started/) ·
  [secrets](https://developers.cloudflare.com/workers/configuration/secrets/) ·
  [local development](https://developers.cloudflare.com/workers/testing/local-development/).
