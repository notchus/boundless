//! The `GroupHub` Durable Object (spec 001 **T07-shell-B slice 1**) — one per Group.
//!
//! Holds the §10-E ephemeral rate-limit / alert-dedup state ([`GroupHubState`]) **in memory**
//! (it survives across requests to the same DO instance; losing it on eviction merely resets a
//! window, which is acceptable for an ephemeral counter), and persists a small durable `served`
//! counter via `state.storage()` — the round-trip that proves the DO storage plumbing (P12
//! operability). The rate-limit *logic* is the core's [`GroupHubState::register_code_attempt`]
//! (P4 — single-sourced, not re-implemented here); only the lock *threshold* is named locally,
//! matching the server-issued Onboarding-Code default (plan §10-D / AC17).
//!
//! This is the skeleton's one DO operation: the Worker forwards `POST /api/auth/bind-device` here
//! to register an Onboarding-Code attempt and learn whether the code is now locked. The full bind
//! (atomic onboarding-code consume + session mint over `PgAuthStore`) is the deferred PG slice.

use std::cell::RefCell;

use boundless_auth::UnixSeconds;
use boundless_domain::MemberId;
use boundless_server_core::GroupHubState;
// Glob import per the workers-rs convention: the `#[durable_object]` macro expands to code that
// references `worker`'s re-exports (incl. `wasm_bindgen`), which a glob brings into scope.
use worker::*;

/// At most 5 Onboarding-Code bind attempts per 15-minute window, then the code locks + the admin is
/// alerted (plan §10-D / AC17 / R4). The authoritative ceiling is the server-issued
/// `OnboardingCodeRow.max_attempts` (default 5) once the real bind path lands; the skeleton uses the
/// default here to demonstrate the window.
const MAX_ATTEMPTS: u32 = 5;

/// One Group's coordination hub: the ephemeral §10-E counters (in memory) + a durable request
/// counter (in `state.storage()`).
#[durable_object]
pub struct GroupHub {
    state: State,
    hub: RefCell<GroupHubState>,
}

impl DurableObject for GroupHub {
    fn new(state: State, _env: Env) -> Self {
        Self {
            state,
            hub: RefCell::new(GroupHubState::new()),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        /// The Worker→DO request: which member is attempting. Server time is read **here** from the
        /// runtime clock — never supplied by the client (a device clock must not be able to choose or
        /// reset the rate-limit window; plan §10 "never a device clock").
        #[derive(serde::Deserialize)]
        struct AttemptReq {
            member: MemberId,
        }

        let body: AttemptReq = match req.json().await {
            Ok(b) => b,
            Err(_) => return Response::error("bad request", 400),
        };
        let now = UnixSeconds::new((Date::now().as_millis() / 1000) as i64);

        // §10-E window via the core (P4). Synchronous: the borrow is released at the end of this
        // statement, before the awaited storage call below (no RefCell borrow held across `.await`).
        let prior = self
            .hub
            .borrow_mut()
            .register_code_attempt(body.member, now);
        let locked = prior >= MAX_ATTEMPTS;

        // Durable operability counter — proves `state.storage()` round-trips across requests.
        let served: u64 = self
            .state
            .storage()
            .get::<u64>("served")
            .await
            .unwrap_or(None)
            .unwrap_or(0)
            + 1;
        self.state.storage().put("served", served).await?;

        Response::from_json(&serde_json::json!({
            "prior_attempts": prior,
            "locked": locked,
            "served": served,
            "error_code": locked.then_some("AUTH_ONBOARDING_CODE_RATE_LIMITED"),
        }))
    }
}
