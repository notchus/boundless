//! The deployable Worker runtime (spec 001 **T07-shell-B slice 1**) — wasm-only.
//!
//! A minimal-but-real skeleton proving the toolchain + plumbing with **no Cloudflare account**
//! (miniflare via `@cloudflare/vitest-pool-workers`, see `server/test/`):
//! - `GET /healthz` — the version handshake (AC7/O4) + a real KV `MANIFEST` read (the seam the
//!   signed-manifest serving will use, ADR-0014).
//! - `POST /api/auth/signin` — the *real* core [`AuthService::sign_in`] (P4) over the
//!   [`ScaffoldStore`], serialized to the frozen `api/openapi.yaml` `SignInResponse` wire shape, and
//!   draining the below-min admin alert to the `ADMIN_ALERTS` Queue (§10-E).
//! - `POST /api/auth/bind-device` — forwarded to the [`group_hub::GroupHub`] Durable Object, which
//!   exercises the core §10-E Onboarding-Code rate-limit *window* (AC17) over a synthetic `{member}`
//!   DO envelope and persists a counter via `state.storage()`. This is **not** the full frozen
//!   `BindDeviceRequest` (no onboarding-code consume, no device-token bind) — that is the deferred PG slice.
//!
//! **Contract note (flagged, not resolved here):** the frozen `SignInRequest` schema carries a
//! client-computed `phone_lookup_hash`, but I3 keys that hash with a **per-instance server secret**
//! (`core/crypto`), which a client cannot hold — and `core::server::sign_in` itself takes the raw
//! (normalized) phone and hashes server-side. So this skeleton's request carries `phone`, matching
//! the system as actually built. Reconciling the OpenAPI `SignInRequest` with I3 needs an ADR
//! (recorded in DEFERRED.md → T07-shell-B); it is out of scope for this bring-up slice.
//!
//! [`AuthService::sign_in`]: boundless_server_core::AuthService::sign_in

// The `#[durable_object]` macro in `group_hub` emits the `GroupHub` JS-class export inline, so the
// module need only be compiled — no re-export is required for wrangler/worker-build to find it.
mod group_hub;
mod scaffold_store;

use boundless_auth::{SignInResult, VersionRequirement};
use boundless_domain::{AppVersion, ClientVersion};
use boundless_server_core::{
    normalize_phone, AuthConfig, AuthService, ManifestPointer, SignInRequest, SignInResponse,
};
use serde::Deserialize;
use serde_json::json;
use worker::{event, Context, Env, Request, Response, Result, Router};

use scaffold_store::{scaffold_key, BufferSink, ScaffoldSecrets, ScaffoldStore, WorkerClock};

/// The single Group per Boundless install (glossary: one install = one Group), so all auth
/// coordination routes to one named `GroupHub` instance.
const GROUP_HUB_NAME: &str = "default";

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get_async("/healthz", |_req, ctx| async move {
            // A real KV read (the manifest-serving seam, ADR-0014) — proves the binding is wired.
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
            let requirement = auth_config().requirement;
            Response::from_json(&json!({
                "status": "ok",
                "client_min_version": requirement.min,
                "client_recommended_version": requirement.recommended,
                "manifest_present": manifest_present,
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

            let mut service = build_service();
            let resp = match service.sign_in(SignInRequest { phone, reported }).await {
                Ok(r) => r,
                // ScaffoldStore never errors on the sign-in path (it only reads members).
                Err(_) => return Response::error("internal", 500),
            };

            // Deliver any PII-free admin alerts the core decided to emit to the ADMIN_ALERTS Queue
            // — the alert-fanout binding. NB (skeleton): the §10-E once-per-day dedup lives in
            // GroupHubState, which `build_service()` re-creates per request, so on THIS sign-in path
            // dedup does not persist across requests (every below-min sign-in re-enqueues). The
            // durable dedup is the DO's job; routing sign-in alert dedup through the GroupHub DO is
            // deferred with the PgAuthStore slice (DEFERRED.md → T07-shell-B).
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
            // consume + session mint over PgAuthStore — is the deferred PG slice.)
            let stub = ctx
                .durable_object("GROUP_HUB")?
                .id_from_name(GROUP_HUB_NAME)?
                .get_stub()?;
            stub.fetch_with_request(req).await
        })
        .run(req, env)
        .await
}

/// The skeleton sign-in request body. Carries the normalized `phone` (not the OpenAPI
/// `phone_lookup_hash` — see the module-level contract note) plus the reported client version.
#[derive(Deserialize)]
struct SignInWire {
    phone: String,
    reported: ClientVersion,
}

/// This instance's auth configuration (the scaffold dev key + the advertised version handshake +
/// the launch manifest pointer). The min/recommended match the frozen `fixtures/auth/*` (1.0.0 /
/// 1.2.0).
fn auth_config() -> AuthConfig {
    AuthConfig::new(
        scaffold_key(),
        VersionRequirement::new(AppVersion::new(1, 0, 0), AppVersion::new(1, 2, 0)),
        ManifestPointer::new("manifest:v1:index", "manifest:v1:"),
    )
}

/// Assemble the member-auth engine over the scaffold ports (to be replaced by `PgAuthStore` +
/// `RngSecretSource` in the deferred PG slice).
fn build_service() -> AuthService<ScaffoldStore, BufferSink, ScaffoldSecrets, WorkerClock> {
    AuthService::new(
        ScaffoldStore::new(),
        BufferSink::default(),
        ScaffoldSecrets,
        WorkerClock,
        auth_config(),
    )
}

/// Serialize the core [`SignInResponse`] to the **frozen** `api/openapi.yaml` wire shape: the flat
/// `VersionHandshake` (`client_min_version` / `client_recommended_version`, present on every
/// response — AC7) + the `oneOf` outcome (matched / not-on-file / below-min). Mirrors
/// `fixtures/auth/{signin_ok,phone_not_on_file,below_min_version}.json` exactly.
///
/// The no-existence-leak guarantee (carry-forward (b)) is that the core decides the outcome in
/// **constant time** on the hash compare and the failure responses carry **no member data** (I6) —
/// *not* that the response bodies are byte-uniform (the matched variant is shaped differently, per
/// the frozen `oneOf`). The body is TLS-only to the entitled client, which may know its own outcome.
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
