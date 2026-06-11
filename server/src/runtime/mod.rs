//! The deployable Worker runtime (spec 001 **T07-shell-B**) — wasm-only.
//!
//! The PgAuthStore slice: the Worker now talks to a **real Postgres over a Hyperdrive `Socket`**
//! (see [`pg`]) and runs the *real* core [`AuthService`] over the real [`PgAuthStore`] (P4). The
//! `T07-shell-B slice 1` in-memory scaffold is gone.
//!
//! - `GET /healthz` — liveness (dependency-free): the version handshake (AC7/O4) + a real KV
//!   `MANIFEST` read (ADR-0014 seam). No DB connect, so uptime monitors don't churn the pooler.
//! - `GET /readyz` — readiness: connect over Hyperdrive + run the W2 least-privilege guard, reporting
//!   `"db": "ok" | "role_too_privileged" | "unavailable"`. The proof that `connect_raw` over the wasm
//!   `worker::Socket` + the `spawn_local` driver + the guard work end-to-end inside workerd. (Opens a
//!   per-call connection — rate-limit/Access-gate it at deploy-hardening, DEFERRED.md → T07-shell-B.)
//! - `POST /api/auth/signin` — the real [`AuthService::sign_in`] over [`PgAuthStore`], serialized to
//!   the frozen `api/openapi.yaml` `SignInResponse` wire shape, draining any below-min admin alert
//!   to the `ADMIN_ALERTS` Queue (§10-E).
//! - `POST /api/auth/bind-device` — still forwarded to the [`group_hub::GroupHub`] Durable Object,
//!   which exercises the §10-E Onboarding-Code rate-limit *window* (AC17). The **full** bind (atomic
//!   onboarding-code consume + session mint + device-token persist over `PgDeviceStore`) needs the
//!   spec-008 token-encryption primitive and is the next slice (DEFERRED.md → T07-shell-B).
//!
//! **Fail-closed at runtime** (this replaces the retired compile-time scaffold guard): the W2
//! [`ensure_least_privilege`] guard refuses a superuser / `BYPASSRLS` DB role, and a missing
//! `HMAC_KEY` / `GROUP_ID` / `HYPERDRIVE` binding errors at request time — so a misconfigured deploy
//! cannot silently serve auth without tenant isolation.
//!
//! **Contract note (ADR-0023):** the auth request schemas carry the E.164 `phone` (TLS-protected),
//! not a client-computed `phone_lookup_hash` — I3 keys that hash with a per-instance server secret a
//! client cannot hold, and `core::server::sign_in` takes the raw (normalized) phone and hashes
//! server-side. `SignInWire { phone }` matches the frozen `api/openapi.yaml`.
//!
//! [`AuthService::sign_in`]: boundless_server_core::AuthService::sign_in

// The `#[durable_object]` macro in `group_hub` emits the `GroupHub` JS-class export inline, so the
// module need only be compiled — no re-export is required for wrangler/worker-build to find it.
mod group_hub;
mod members;
mod pg;

use boundless_auth::{SignInResult, VersionRequirement};
use boundless_crypto::HmacKey;
use boundless_domain::{AppVersion, ClientVersion};
use boundless_server_core::{
    normalize_phone, AuthConfig, AuthService, ManifestPointer, SignInRequest, SignInResponse,
};
use boundless_server_store::{ensure_least_privilege, PgAuthStore};
use serde::Deserialize;
use serde_json::json;
use worker::{event, Context, Env, Request, Response, Result, Router};

use pg::{
    connect_pg, load_group_id, load_hmac_key, BufferSink, PgService, PlaceholderSecrets,
    WorkerClock,
};

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get_async("/healthz", |_req, ctx| async move {
            // Liveness — dependency-free (no DB connect): the version handshake (AC7/O4) + a real KV
            // `MANIFEST` read (the manifest-serving seam, ADR-0014). Cheap for uptime monitors.
            let manifest_present = match ctx.kv("MANIFEST") {
                Ok(kv) => kv
                    .get("manifest:v1:index")
                    .text()
                    .await
                    .ok()
                    .flatten()
                    .is_some(),
                Err(_) => false,
            };
            let requirement = requirement();
            Response::from_json(&json!({
                "status": "ok",
                "client_min_version": requirement.min,
                "client_recommended_version": requirement.recommended,
                "manifest_present": manifest_present,
            }))
        })
        .get_async("/readyz", |_req, ctx| async move {
            // Readiness — the DB probe: connect over Hyperdrive + the W2 least-privilege guard (a real
            // query). `db:"ok"` proves the whole transport (connect_raw over the wasm Socket +
            // spawn_local driver + guard) works in workerd. Value-free states (P2): ok /
            // role_too_privileged / unavailable. Never `?` — /readyz must answer even when the DB is
            // down. NB: opens a per-call connection — an unauth caller can drive DB connects, so
            // rate-limit / Access-gate this at deploy-hardening (DEFERRED.md → T07-shell-B).
            let db = match connect_pg(&ctx.env).await {
                Ok(client) => match ensure_least_privilege(&client).await {
                    Ok(()) => "ok",
                    Err(_) => "role_too_privileged",
                },
                Err(_) => "unavailable",
            };
            let requirement = requirement();
            Response::from_json(&json!({
                "status": "ok",
                "client_min_version": requirement.min,
                "client_recommended_version": requirement.recommended,
                "db": db,
            }))
        })
        .post_async("/api/auth/signin", |mut req, ctx| async move {
            let body: SignInWire = match req.json().await {
                Ok(b) => b,
                Err(_) => return Response::error("bad request", 400),
            };
            let phone = match normalize_phone(&body.phone) {
                Ok(p) => p,
                Err(_) => return Response::error("bad phone", 400),
            };
            let reported = body.reported;

            // Connect + W2 guard + compose the engine over the real PgAuthStore. Any failure
            // (binding missing, too-privileged role, bad config) → a uniform, value-free 500.
            let mut service = match build_service(&ctx.env).await {
                Ok(s) => s,
                Err(_) => return Response::error("internal", 500),
            };
            let resp = match service.sign_in(SignInRequest { phone, reported }).await {
                Ok(r) => r,
                Err(_) => return Response::error("internal", 500),
            };

            // Deliver any PII-free admin alerts the core decided to emit to the ADMIN_ALERTS Queue.
            // NB (deferred): the §10-E once-per-day dedup lives in GroupHubState, which
            // `build_service` re-creates per request, so on THIS sign-in path dedup does not persist
            // across requests; routing sign-in alert dedup through the GroupHub DO is the next slice
            // (DEFERRED.md → T07-shell-B).
            let alerts = service.alerts.drain();
            if !alerts.is_empty() {
                let queue = ctx.env.queue("ADMIN_ALERTS")?;
                for alert in alerts {
                    queue.send(alert).await?;
                }
            }

            Response::from_json(&signin_wire(&resp, &reported))
        })
        .post_async("/api/auth/bind-device", |req, ctx| async move {
            // Forward to the per-Group GroupHub DO: it applies the §10-E rate-limit window and
            // persists its counter via state.storage(). (The full bind — atomic onboarding-code
            // consume + session mint + device-token persist over PgDeviceStore — is the next slice.)
            let stub = ctx
                .durable_object("GROUP_HUB")?
                .id_from_name(GROUP_HUB_NAME)?
                .get_stub()?;
            stub.fetch_with_request(req).await
        })
        // ── Admin member-management (spec 008 T09) — the ADR-0026 shared-secret BFF surface. Each
        // handler runs `admin_guard` first (fail-closed) and composes the real `MemberService` over
        // `PgMemberStore`. The `:id` segment is read via `ctx.param("id")`.
        .get_async("/api/admin/members", members::list)
        .post_async("/api/admin/members", members::issue)
        .get_async("/api/admin/members/:id", members::detail)
        .patch_async("/api/admin/members/:id", members::edit)
        .post_async(
            "/api/admin/members/:id/regenerate-code",
            members::regenerate,
        )
        .get_async("/api/admin/audit-log", members::audit_log)
        .run(req, env)
        .await
}

/// The single Group per Boundless install (glossary: one install = one Group), so all auth
/// coordination routes to one named `GroupHub` instance.
const GROUP_HUB_NAME: &str = "default";

/// The skeleton sign-in request body. Carries the normalized `phone` (ADR-0023 — not a
/// `phone_lookup_hash`) plus the reported client version.
#[derive(Deserialize)]
struct SignInWire {
    phone: String,
    reported: ClientVersion,
}

/// The advertised version handshake (AC7/O4). Min/recommended match the frozen `fixtures/auth/*`
/// (1.0.0 / 1.2.0). Standalone (not folded into [`auth_config`]) so `/healthz` can report it
/// without needing the HMAC key.
fn requirement() -> VersionRequirement {
    VersionRequirement::new(AppVersion::new(1, 0, 0), AppVersion::new(1, 2, 0))
}

/// This instance's auth configuration: the per-instance HMAC key (I3, loaded from the `HMAC_KEY`
/// binding), the version handshake, and the launch manifest pointer (ADR-0014).
fn auth_config(key: HmacKey) -> AuthConfig {
    AuthConfig::new(
        key,
        requirement(),
        ManifestPointer::new("manifest:v1:index", "manifest:v1:"),
    )
}

/// Connect over Hyperdrive, run the W2 least-privilege guard (fail closed), and assemble the
/// member-auth engine over the real `PgAuthStore` (+ the in-memory device half) with the loaded
/// HMAC key + Group id. Per-request — a fresh connection each call (the Hyperdrive pooler pattern).
async fn build_service(
    env: &Env,
) -> Result<AuthService<PgService, BufferSink, PlaceholderSecrets, WorkerClock>> {
    let client = connect_pg(env).await?;
    // W2 (sec-audit, highest-impact): refuse a superuser / BYPASSRLS role — RLS would silently not
    // apply → cross-tenant PII. The detail is value-free; the client sees only a generic 500.
    ensure_least_privilege(&client)
        .await
        .map_err(|_| worker::Error::RustError("db role misconfigured".into()))?;
    let group_id = load_group_id(env)?;
    let key = load_hmac_key(env)?;
    Ok(AuthService::new(
        PgService::new(PgAuthStore::new(client, group_id)),
        BufferSink::default(),
        PlaceholderSecrets,
        WorkerClock,
        auth_config(key),
    ))
}

/// Serialize the core [`SignInResponse`] to the **frozen** `api/openapi.yaml` wire shape: the flat
/// `VersionHandshake` (`client_min_version` / `client_recommended_version`, present on every
/// response — AC7) + the `oneOf` outcome (matched / not-on-file / below-min). Mirrors
/// `fixtures/auth/{signin_ok,phone_not_on_file,below_min_version}.json` exactly.
///
/// The no-existence-leak guarantee is that the core decides the outcome in **constant time** on the
/// hash compare and the failure responses carry **no member data** (I6) — *not* that the bodies are
/// byte-uniform (the matched variant is shaped differently, per the frozen `oneOf`). The body is
/// TLS-only to the entitled client, which may know its own outcome.
fn signin_wire(resp: &SignInResponse, reported: &ClientVersion) -> serde_json::Value {
    match resp.result {
        SignInResult::MemberMatched => json!({
            "outcome": "member_matched",
            "next_step": "device_binding",
            "manifest_pointer": resp.manifest_pointer,
            "client_min_version": resp.version.min,
            "client_recommended_version": resp.version.recommended,
        }),
        SignInResult::PhoneNotOnFile => json!({
            "error_code": "AUTH_PHONE_NOT_ON_FILE",
            "client_min_version": resp.version.min,
            "client_recommended_version": resp.version.recommended,
        }),
        SignInResult::BelowMinVersion => json!({
            "error_code": "AUTH_BELOW_MIN_VERSION",
            "reported_client_version": reported,
            "client_min_version": resp.version.min,
            "client_recommended_version": resp.version.recommended,
        }),
    }
}
