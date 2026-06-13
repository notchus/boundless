//! The deployable Option B1 admin-WebAuthn persistence routes (spec 009 **T04**, ADR-0027) —
//! wasm-only.
//!
//! `/api/admin/webauthn/*`: the durable half of the admin passkey onboarding/sign-in the SvelteKit
//! edge drives. These move the WebAuthn **invite + credential persistence** behind the Rust Worker
//! (web tier = zero Postgres, B1) — the store + the invite-token HMAC compare stay in Rust (P4); the
//! ceremony verification + the TTL/consumed verdict stay edge-TS (ADR-0017 carve-out, scope
//! unchanged). Each handler composes the **real** [`AdminWebAuthnStore`] over the real [`PgAuthStore`]
//! (the T02 impl), RLS-scoped to the single-install `GROUP_ID` (D3).
//!
//! ## Pre-session trust model (ADR-0027 — differs from `/api/admin/members/*`)
//!
//! Every B1 op here runs **before** a verified admin session exists — the admin is being registered
//! (`invite/resolve`, `register-complete`) or authenticated (`credentials/lookup`, `sign-count`
//! during assertion verification). So they require the ADR-0026 shared secret ([`admin_secret_guard`])
//! but carry **no** `X-Admin-Id` — there is no acting admin to assert (contrast the session-bearing
//! member ops, which carry both). The `register-complete` admin id is *derived from the consumed
//! invitation row*, never web-supplied.
//!
//! ## Privacy spine (P2/R13/I10)
//!
//! - The presented invite token arrives in the **POST body**, never the URL/query. It is wrapped in
//!   the tainted [`AdminInvitationToken`] (no `Debug`/`Display`/`Serialize`) on parse, so it cannot be
//!   logged or echoed; the resolve/register-complete error paths emit only stable value-free codes
//!   (never the token, never an existence oracle — a no-match returns the same shape as a wrong-group
//!   token).
//! - These rows are **PII-free** (opaque WebAuthn bytes + counters + server-time instants), so the
//!   endpoints are not `x-requires-audit`. Every 200 body still goes through the sealed
//!   [`admin_response_body`](boundless_server_core::admin_response_body) seam (the B1 DTOs are blessed
//!   `AuditedResponse` in `core/server`), so the Worker hand-rolls no admin JSON.
//!
//! [`AdminWebAuthnStore`]: boundless_server_core::AdminWebAuthnStore
//! [`PgAuthStore`]: boundless_server_store::PgAuthStore
//! [`admin_response_body`]: boundless_server_core::admin_response_body

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use boundless_auth::Clock;
use boundless_domain::AdminInvitationToken;
use boundless_server_core::{
    AdminRegisterCompleteResult, AdminWebAuthnStore, NewAdminCredential, RegisterCompleteOutcome,
};
use boundless_server_store::{ensure_least_privilege, PgAuthStore};
use serde::Deserialize;
use worker::{Env, Request, Response, Result, RouteContext};

use super::members::{admin_secret_guard, audited_body, err_code};
use super::pg::{connect_pg, load_group_id, load_hmac_key, WorkerClock};

/// Run the pre-session [`admin_secret_guard`] and short-circuit the handler with its reject `Response`
/// (a missing binding → 500, a missing/wrong shared secret → 401). On success, control continues with
/// no acting admin id bound (the pre-session carve-out — ADR-0027).
macro_rules! guard_secret {
    ($req:expr, $env:expr) => {
        if let Err(resp) = admin_secret_guard(&$req, &$env)? {
            return Ok(resp);
        }
    };
}

// ===== the routes =================================================================================

/// `POST /api/admin/webauthn/invite/resolve` — resolve a presented token → its pending-admin invite
/// metadata (AC4b). The token is in the BODY (R13), tainted on parse. The core computes its keyed
/// hash and matches by the unique `token_hash` index inside the `GROUP_ID`-scoped RLS txn (D3); an
/// unknown OR cross-tenant token returns the same value-free `ADMIN_INVITE_NOT_FOUND` 404 — no
/// existence oracle (R16). Read-only (the TTL/consumed verdict is the edge-TS `evaluateInvite`).
pub(crate) async fn invite_resolve(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    guard_secret!(req, ctx.env);
    let Ok(body) = req.json::<ResolveInviteWire>().await else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let token = AdminInvitationToken::new(body.token);
    let key = match load_hmac_key(&ctx.env) {
        Ok(k) => k,
        Err(_) => return Response::error("internal", 500),
    };
    let mut store = match build_admin_store(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match store.resolve_invitation_by_token(&key, &token).await {
        Ok(Some(record)) => audited_body(&record, 200),
        Ok(None) => err_code("ADMIN_INVITE_NOT_FOUND", 404),
        Err(_) => Response::error("internal", 500),
    }
}

/// `POST /api/admin/webauthn/register-complete` — consume the invite + revoke prior credentials +
/// insert the new one in ONE server-side transaction (R11; AC4a single-use). The admin id is DERIVED
/// from the consumed invitation row, never web-supplied. A token matching no live invitation in this
/// tenant rolls back (nothing written) → value-free `ADMIN_INVITE_CONSUMED` (the TOCTOU backstop after
/// the edge `evaluateInvite`).
pub(crate) async fn register_complete(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    guard_secret!(req, ctx.env);
    let Ok(body) = req.json::<RegisterCompleteWire>().await else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let token = AdminInvitationToken::new(body.token);
    let Some(credential) = body.credential.into_new_credential() else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let key = match load_hmac_key(&ctx.env) {
        Ok(k) => k,
        Err(_) => return Response::error("internal", 500),
    };
    let mut store = match build_admin_store(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    let now = WorkerClock.now();
    match store.register_complete(&key, &token, credential, now).await {
        Ok(RegisterCompleteOutcome::Completed { admin_id }) => {
            audited_body(&AdminRegisterCompleteResult { admin_id }, 200)
        }
        Ok(RegisterCompleteOutcome::InviteNotConsumable) => err_code("ADMIN_INVITE_CONSUMED", 400),
        Err(_) => Response::error("internal", 500),
    }
}

/// `POST /api/admin/webauthn/credentials/lookup` — resolve a presented `credential_id` → the active
/// credential for usernameless assertion sign-in. The admin id is read OFF the resolved credential
/// (hence pre-session). A revoked / unknown / cross-tenant id → value-free `ADMIN_CREDENTIAL_NOT_FOUND`
/// 404 (RLS-scoped, D3).
pub(crate) async fn credential_lookup(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    guard_secret!(req, ctx.env);
    let Ok(body) = req.json::<CredentialLookupWire>().await else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let Some(credential_id) = decode_b64url(&body.credential_id) else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let mut store = match build_admin_store(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match store.find_active_credential(&credential_id).await {
        Ok(Some(credential)) => audited_body(&credential, 200),
        Ok(None) => err_code("ADMIN_CREDENTIAL_NOT_FOUND", 404),
        Err(_) => Response::error("internal", 500),
    }
}

/// `POST /api/admin/webauthn/credentials/{id}/sign-count` — bump the stored signature counter
/// only-if-strictly-greater (the WebAuthn clone-detection backstop, R10) and only while active. `{id}`
/// is the base64url credential_id. Best-effort + idempotent — the result is not surfaced (204),
/// mirroring the store's unit return.
pub(crate) async fn bump_sign_count(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    guard_secret!(req, ctx.env);
    let Some(credential_id) = ctx.param("id").and_then(|s| decode_b64url(s)) else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let Ok(body) = req.json::<SignCountBumpWire>().await else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let mut store = match build_admin_store(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match store.bump_sign_count(&credential_id, body.sign_count).await {
        Ok(()) => Ok(Response::empty()?.with_status(204)),
        Err(_) => Response::error("internal", 500),
    }
}

// ===== helpers ====================================================================================

/// Decode a base64url-no-pad wire field to raw bytes (the `credential_id`/`public_key`/`aaguid`
/// WebAuthn-byte convention shared with `@simplewebauthn` and the core DTOs). `None` on bad input —
/// the caller returns a value-free 400 and never echoes the input (P2/R10).
fn decode_b64url(s: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(s).ok()
}

/// Connect over Hyperdrive, run the W2 least-privilege guard (fail closed — refuse a superuser /
/// `BYPASSRLS` role, R17), and build the [`PgAuthStore`] scoped to the single-install `GROUP_ID` (the
/// RLS tenant, D3/R16). Per-request — a fresh connection (the Hyperdrive pooler pattern); each
/// [`AdminWebAuthnStore`] method runs in its own `GROUP_ID`-scoped transaction. Any failure maps to a
/// value-free `worker::Error` (the caller returns a generic 500; the connection string never reaches
/// the wire — P2). Mirrors `members::build_member_service`'s connect+guard+scope assembly.
async fn build_admin_store(env: &Env) -> Result<PgAuthStore> {
    let client = connect_pg(env).await?;
    ensure_least_privilege(&client)
        .await
        .map_err(|_| worker::Error::RustError("db role misconfigured".into()))?;
    let group_id = load_group_id(env)?;
    Ok(PgAuthStore::new(client, group_id))
}

// ===== request bodies =============================================================================

/// `POST …/invite/resolve` body (the contract `ResolveInviteRequest`). The presented token is in the
/// BODY (never the URL/query — R13).
#[derive(Deserialize)]
struct ResolveInviteWire {
    token: String,
}

/// `POST …/register-complete` body (the contract `RegisterCompleteRequest`). The admin id is derived
/// server-side from the consumed invitation — never carried here.
#[derive(Deserialize)]
struct RegisterCompleteWire {
    token: String,
    credential: NewCredentialWire,
}

/// The new-credential leg of register-complete (the contract `NewAdminCredential`). The byte fields
/// are base64url-no-pad on the wire; decoded to raw `bytea` for the store.
#[derive(Deserialize)]
struct NewCredentialWire {
    credential_id: String,
    public_key: String,
    sign_count: i64,
    transports: Option<Vec<String>>,
    aaguid: Option<String>,
}

impl NewCredentialWire {
    /// Decode the base64url byte fields into the core [`NewAdminCredential`]; `None` if any byte field
    /// is not valid base64url (→ a value-free 400).
    fn into_new_credential(self) -> Option<NewAdminCredential> {
        Some(NewAdminCredential {
            credential_id: decode_b64url(&self.credential_id)?,
            public_key: decode_b64url(&self.public_key)?,
            sign_count: self.sign_count,
            transports: self.transports,
            aaguid: match self.aaguid {
                Some(s) => Some(decode_b64url(&s)?),
                None => None,
            },
        })
    }
}

/// `POST …/credentials/lookup` body (the contract `CredentialLookupRequest`).
#[derive(Deserialize)]
struct CredentialLookupWire {
    credential_id: String,
}

/// `POST …/credentials/{id}/sign-count` body (the contract `SignCountBumpRequest`).
#[derive(Deserialize)]
struct SignCountBumpWire {
    sign_count: i64,
}
