//! The deployable admin member-management routes (spec 008 **T09**) — wasm-only.
//!
//! `/api/admin/members/*` + `/api/admin/audit-log`: the producer side of the closed-group model
//! (issue / list / detail / edit / regenerate-code / audit-log). Composes the **real** core
//! [`MemberService`] over the **real** [`PgMemberStore`] (P4 — issuance logic single-sourced), with a
//! live getrandom-backed CSPRNG injected into [`RngSecretSource`] (ADR-0021: the Worker — not the
//! core — holds randomness) and the per-Group key loaded + unwrapped **per request** from the KEK
//! (no long-lived plaintext key cached in the DO — see `DEFERRED.md` → T09).
//!
//! ## Trust model (ADR-0026)
//!
//! ADR-0017 put admin WebAuthn verification in the SvelteKit tier; the Worker has no admin-session
//! notion. So the WebAuthn-verified SvelteKit **BFF** calls these routes with a server-to-server
//! shared secret (`ADMIN_API_SECRET`) and asserts the verified acting admin id (`X-Admin-Id`). The
//! [`admin_guard`] checks the secret in **constant time** and **fails closed** without it (401); a
//! missing binding is a 500 (misconfig), a missing/bad admin id a 400. `group_id` is the single-
//! install `GROUP_ID` binding (the RLS tenant). The real SvelteKit→Worker call is wired at T10.
//!
//! ## Privacy spine in the shell (P2/R10/I5)
//!
//! - Inbound `name`/`address`/`phone` are wrapped into the **tainted** [`MemberName`]/[`Address`]/
//!   [`PhoneNumber`] (no `Debug`/`Serialize`) the moment the body is parsed; they are never logged and
//!   never echoed — every rejection returns a stable, value-free [`ErrorBody`](err_code) code (R10).
//! - Every admin response body is serialized through the sealed [`admin_response_body`] seam (the I5
//!   gate): the decrypted-PII detail can only reach the wire as a `PiiDisclosure<MemberDetailView>`
//!   minted after the audit committed, and the other bodies are the blessed PII-free envelope views.
//!   The Worker hand-rolls **no** member-PII JSON.

use boundless_auth::UnixSeconds;
use boundless_domain::{Address, MemberId, MemberName, PhoneNumber, Role};
use boundless_server_core::{
    admin_response_body, issuable_roles, AuditLogView, AuditedResponse, DetailRead,
    DuplicatePhoneLinkView, EditMemberInput, EditMemberOutcome, IssueMemberInput,
    IssueMemberOutcome, MemberConfig, MemberError, MemberIssuedView, MemberListView, MemberService,
    RegenerateCodeView, RegenerateOutcome, RngSecretSource,
};
use boundless_server_store::{ensure_least_privilege, PgMemberStore};
use rand_core::{CryptoRng, RngCore};
use serde::Deserialize;
use uuid::Uuid;
use worker::{Env, Headers, Request, Response, Result, RouteContext};

use super::pg::{connect_pg, load_group_id, load_hmac_key, load_kek, WorkerClock};

// ===== the live Worker CSPRNG (ADR-0021) ==========================================================

/// A getrandom-backed CSPRNG, injected into [`RngSecretSource`] so the core stays randomness-free
/// (ADR-0021). getrandom's `wasm_js` backend (Web Crypto) provides workerd entropy. The `.expect()` on
/// each draw is **load-bearing**: ignoring the error could yield all-zero bytes — a catastrophic
/// nonce/key reuse (R1) — so a getrandom failure must panic (fail-closed: the request 500s) rather than
/// ever produce predictable output.
struct GetrandomRng;

impl RngCore for GetrandomRng {
    fn next_u32(&mut self) -> u32 {
        let mut b = [0u8; 4];
        getrandom::fill(&mut b).expect("workerd getrandom (wasm_js) must not fail");
        u32::from_ne_bytes(b)
    }
    fn next_u64(&mut self) -> u64 {
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).expect("workerd getrandom (wasm_js) must not fail");
        u64::from_ne_bytes(b)
    }
    fn fill_bytes(&mut self, dst: &mut [u8]) {
        getrandom::fill(dst).expect("workerd getrandom (wasm_js) must not fail");
    }
}

// The `CryptoRng` marker asserts the generator is cryptographically secure (getrandom is).
impl CryptoRng for GetrandomRng {}

/// The composed engine the admin routes drive: the real `PgMemberStore` over Hyperdrive + the live
/// CSPRNG + server time + the per-instance config (HMAC + KEK).
type AdminService = MemberService<PgMemberStore, RngSecretSource<GetrandomRng>, WorkerClock>;

/// Connect over Hyperdrive, run the W2 least-privilege guard (fail closed), and assemble the member
/// engine over the real `PgMemberStore` with the live CSPRNG, the HMAC key, and the KEK. Per-request —
/// a fresh connection + a fresh per-request Group-key unwrap (inside the service), the Hyperdrive pooler
/// pattern. Any failure (binding missing, too-privileged role, bad config) maps to a value-free error.
async fn build_member_service(env: &Env) -> Result<AdminService> {
    let client = connect_pg(env).await?;
    // W2 (sec-audit): refuse a superuser / BYPASSRLS role — RLS would silently not apply → cross-tenant
    // PII. The detail is value-free; the caller sees a generic 500.
    ensure_least_privilege(&client)
        .await
        .map_err(|_| worker::Error::RustError("db role misconfigured".into()))?;
    let group_id = load_group_id(env)?;
    let config = MemberConfig {
        hmac_key: load_hmac_key(env)?,
        kek: load_kek(env)?,
    };
    Ok(MemberService::new(
        PgMemberStore::new(client, group_id),
        RngSecretSource::new(GetrandomRng),
        WorkerClock,
        config,
    ))
}

// ===== the ADR-0026 shared-secret admin gate =====================================================

/// The shared-secret + asserted-admin-id gate (ADR-0026). Returns the verified acting [`MemberId`], or
/// an early reject `Response` (the inner `Err`). Fails **closed**: a missing `ADMIN_API_SECRET` binding
/// → 500 (misconfig); a missing/wrong bearer → 401; a missing/bad `X-Admin-Id` → 400. The secret
/// compare is constant-time (no content-timing leak). The outer `Result` is for genuinely-fallible
/// header/Response construction.
fn admin_guard(req: &Request, env: &Env) -> Result<core::result::Result<MemberId, Response>> {
    let Ok(expected) = env.var("ADMIN_API_SECRET").map(|v| v.to_string()) else {
        // Misconfigured deploy — fail closed with a value-free 500 (never reveal the binding state).
        return Ok(Err(Response::error("admin api not configured", 500)?));
    };
    let presented = req
        .headers()
        .get("authorization")?
        .and_then(|h| h.strip_prefix("Bearer ").map(str::to_string))
        .unwrap_or_default();
    if !constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
        return Ok(Err(err_code("ADMIN_UNAUTHORIZED", 401)?));
    }
    // The BFF-asserted acting admin id (trusted because the shared secret verified) — the I5 audit actor.
    match req
        .headers()
        .get("x-admin-id")?
        .and_then(|s| Uuid::parse_str(s.trim()).ok())
    {
        Some(u) => Ok(Ok(MemberId::from_uuid(u))),
        None => Ok(Err(err_code("ADMIN_BAD_REQUEST", 400)?)),
    }
}

/// Constant-time byte-equality (no early exit on content). The length check leaks only the length of a
/// high-entropy shared secret, not its content — the timing-sensitive part.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Run [`admin_guard`] and bind the acting admin id, or short-circuit the handler with the reject.
macro_rules! guard_admin {
    ($req:expr, $env:expr) => {
        match admin_guard(&$req, &$env)? {
            Ok(id) => id,
            Err(resp) => return Ok(resp),
        }
    };
}

// ===== response helpers ===========================================================================

/// Emit a body already serialized through the sealed [`admin_response_body`] seam (the I5 gate) with a
/// JSON content-type + status. The `R: AuditedResponse + Serialize` bound means a bare PII DTO cannot
/// be sent here — only a blessed envelope or a `PiiDisclosure` (which an audit minted).
fn audited_body<R: AuditedResponse + serde::Serialize>(view: &R, status: u16) -> Result<Response> {
    let body = admin_response_body(view)
        .map_err(|_| worker::Error::RustError("serialize admin body".into()))?;
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    Ok(Response::ok(body)?
        .with_status(status)
        .with_headers(headers))
}

/// A PII-free [`ErrorBody`](https://) envelope — only a stable code, never the submitted value (P2/R10).
fn err_code(code: &str, status: u16) -> Result<Response> {
    Ok(Response::from_json(&serde_json::json!({ "error_code": code }))?.with_status(status))
}

/// The HTTP status for a validation rejection. Bad input → 400; no Group key → 503 (operator).
fn status_for(e: MemberError) -> u16 {
    match e {
        MemberError::PhoneInvalid | MemberError::AddressInvalid | MemberError::RolesRequired => 400,
        MemberError::GroupKeyMissing => 503,
    }
}

/// Mint a server-minted opaque request-correlation id (never client-echoed — it lands in the audit
/// row, so a client-supplied value would be an injection point). 16 random bytes, lowercase hex.
fn mint_request_id() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("workerd getrandom (wasm_js) must not fail");
    let mut s = String::with_capacity(32);
    for byte in b {
        s.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap_or('0'));
    }
    s
}

/// The path `:id` segment → a [`MemberId`], or `None` on a malformed uuid.
fn path_member_id(ctx: &RouteContext<()>) -> Option<MemberId> {
    Uuid::parse_str(ctx.param("id")?.trim())
        .ok()
        .map(MemberId::from_uuid)
}

/// The optional `?member_id=` audit-log filter.
fn query_member_id(req: &Request) -> Option<MemberId> {
    let url = req.url().ok()?;
    let v = url
        .query_pairs()
        .find(|(k, _)| k == "member_id")
        .map(|(_, v)| v.into_owned())?;
    Uuid::parse_str(v.trim()).ok().map(MemberId::from_uuid)
}

// ===== request bodies =============================================================================

/// `POST /api/admin/members` body (the contract `IssueMemberRequest`). PII is wrapped into tainted
/// types immediately on use; `roles` deserialize to the domain `Role` so an `admin` is rejected
/// server-side by the core `issuable_roles` (I11), not merely by the wire enum.
#[derive(Deserialize)]
struct IssueWire {
    name: String,
    phone: String,
    address: String,
    roles: Vec<Role>,
}

/// `PATCH /api/admin/members/{id}` body (the contract `EditMemberRequest`). Every PII field optional.
#[derive(Deserialize)]
struct EditWire {
    name: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    roles: Option<Vec<Role>>,
    expected_updated_at: i64,
}

// ===== the routes =================================================================================

/// `GET /api/admin/members` — the PII-free member list (AC8; not an audited read). The `search`/`role`/
/// `status` query filters are accepted but **not yet applied** (the core `list_members` takes no filter
/// — server-side search/filter is deferred, see `DEFERRED.md` → T07/T10).
pub(crate) async fn list(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let _admin = guard_admin!(req, ctx.env);
    let mut svc = match build_member_service(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match svc.list_members().await {
        Ok(Ok(members)) => audited_body(&MemberListView::new(members), 200),
        Ok(Err(e)) => err_code(e.error_code(), status_for(e)),
        Err(_) => Response::error("internal", 500),
    }
}

/// `POST /api/admin/members` — issue a Rider/Driver member + mint a one-time Onboarding Code
/// (AC1/AC5). A phone collision surfaces-and-links the existing member (I5-audited in the store).
pub(crate) async fn issue(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let admin = guard_admin!(req, ctx.env);
    let Ok(body) = req.json::<IssueWire>().await else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    // Reject the Admin role server-side (I11/AC10) — the single-source core conversion.
    let roles = match issuable_roles(&body.roles) {
        Ok(r) => r,
        Err(_) => return err_code("ADMIN_MEMBER_ROLE_FORBIDDEN", 400),
    };
    let input = IssueMemberInput {
        name: MemberName::new(body.name),
        phone: PhoneNumber::new(body.phone),
        address: Address::new(body.address),
        roles,
    };
    let mut svc = match build_member_service(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match svc.issue_member(admin, input, mint_request_id()).await {
        Ok(IssueMemberOutcome::Issued {
            member,
            onboarding_code,
            code_expires_at,
        }) => audited_body(
            &MemberIssuedView::new(member, &onboarding_code, code_expires_at),
            201,
        ),
        Ok(IssueMemberOutcome::DuplicatePhone { existing }) => {
            audited_body(&DuplicatePhoneLinkView::new(existing), 409)
        }
        Ok(IssueMemberOutcome::Rejected(e)) => err_code(e.error_code(), status_for(e)),
        Err(_) => Response::error("internal", 500),
    }
}

/// `GET /api/admin/members/{id}` — the audited PII detail read (AC7). The audit row is written
/// atomically with the ciphertext SELECT in the store (I5/§7); the decrypted view reaches the wire only
/// as the `PiiDisclosure` the read minted.
pub(crate) async fn detail(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let admin = guard_admin!(req, ctx.env);
    let Some(id) = path_member_id(&ctx) else {
        return err_code("ADMIN_MEMBER_NOT_FOUND", 404);
    };
    let mut svc = match build_member_service(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match svc.read_detail(admin, id, mint_request_id()).await {
        Ok(DetailRead::Detail(disclosure)) => audited_body(&*disclosure, 200),
        Ok(DetailRead::NotFound) => err_code("ADMIN_MEMBER_NOT_FOUND", 404),
        Ok(DetailRead::GroupKeyMissing) => err_code("ADMIN_GROUP_KEY_MISSING", 503),
        Err(_) => Response::error("internal", 500),
    }
}

/// `PATCH /api/admin/members/{id}` — edit under optimistic concurrency, then return the updated detail
/// (an audited read — AC11). The edit itself is not an audited read; the read-back writes the audit row.
pub(crate) async fn edit(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let admin = guard_admin!(req, ctx.env);
    let Some(id) = path_member_id(&ctx) else {
        return err_code("ADMIN_MEMBER_NOT_FOUND", 404);
    };
    let Ok(body) = req.json::<EditWire>().await else {
        return err_code("ADMIN_BAD_REQUEST", 400);
    };
    let roles = match body.roles {
        Some(rs) => match issuable_roles(&rs) {
            Ok(r) => Some(r),
            Err(_) => return err_code("ADMIN_MEMBER_ROLE_FORBIDDEN", 400),
        },
        None => None,
    };
    let input = EditMemberInput {
        name: body.name.map(MemberName::new),
        phone: body.phone.map(PhoneNumber::new),
        address: body.address.map(Address::new),
        roles,
        expected_updated_at: UnixSeconds::new(body.expected_updated_at),
    };
    let mut svc = match build_member_service(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match svc.edit_member(id, input, mint_request_id()).await {
        Ok(EditMemberOutcome::Updated) => {
            // Return the updated detail — an audited read (the audit row is for this read-back).
            match svc.read_detail(admin, id, mint_request_id()).await {
                Ok(DetailRead::Detail(disclosure)) => audited_body(&*disclosure, 200),
                Ok(DetailRead::NotFound) => err_code("ADMIN_MEMBER_NOT_FOUND", 404),
                Ok(DetailRead::GroupKeyMissing) => err_code("ADMIN_GROUP_KEY_MISSING", 503),
                Err(_) => Response::error("internal", 500),
            }
        }
        Ok(EditMemberOutcome::Stale) => err_code("ADMIN_MEMBER_EDIT_STALE", 409),
        Ok(EditMemberOutcome::Rejected(e)) => err_code(e.error_code(), status_for(e)),
        Err(_) => Response::error("internal", 500),
    }
}

/// `POST /api/admin/members/{id}/regenerate-code` — mint a fresh Onboarding Code, supersede the prior
/// live one atomically (AC6). Not an audited read (no name/phone/address is disclosed).
pub(crate) async fn regenerate(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let _admin = guard_admin!(req, ctx.env);
    let Some(id) = path_member_id(&ctx) else {
        return err_code("ADMIN_MEMBER_NOT_FOUND", 404);
    };
    let mut svc = match build_member_service(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match svc.regenerate_onboarding_code(id).await {
        Ok(RegenerateOutcome::Regenerated {
            onboarding_code,
            code_expires_at,
        }) => audited_body(
            &RegenerateCodeView::new(&onboarding_code, code_expires_at),
            200,
        ),
        Ok(RegenerateOutcome::NotFound) => err_code("ADMIN_MEMBER_NOT_FOUND", 404),
        Err(_) => Response::error("internal", 500),
    }
}

/// `GET /api/admin/audit-log` — the I5 PII-read audit log (AC9), field names only, optionally filtered
/// to one member. Not itself an audited read (no values).
pub(crate) async fn audit_log(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let _admin = guard_admin!(req, ctx.env);
    let member = query_member_id(&req);
    let mut svc = match build_member_service(&ctx.env).await {
        Ok(s) => s,
        Err(_) => return Response::error("internal", 500),
    };
    match svc.read_audit_log(member).await {
        Ok(entries) => audited_body(&AuditLogView::new(entries), 200),
        Err(_) => Response::error("internal", 500),
    }
}
