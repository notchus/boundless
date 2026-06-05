//! Orchestration tests for developer Admin provisioning (spec 001 **T08**, core leg):
//! `authorize_developer` + `AuthService::create_admin` / `reissue_admin_invite` over the in-memory
//! store. Closes the **logic** legs of AC1(a) (dev-only authz) and AC16 (single-use, server-time
//! TTL mint); the HTTP-level authz integration test + Email Workers delivery are the deferred shell.

mod common;

use boundless_crypto::admin_invitation_token_matches;
use boundless_domain::AdminInvitationToken;
use boundless_server_core::{authorize_developer, DevCaller, INVITE_TTL_SECS};

use common::*;

const NOW: i64 = 1_000;

#[test]
fn ac1_admin_creation_rejects_unauth_and_admin() {
    // AC1(a) / I11: only the Developer may create an Admin. Unauthenticated AND admin-authenticated
    // callers are both rejected, with the stable P12 code. (The HTTP-level proof that a real
    // unauth/admin request never reaches the mint is the deferred Worker shell.)
    for caller in [
        DevCaller::Unauthenticated,
        DevCaller::Member,
        DevCaller::Admin,
    ] {
        let err = authorize_developer(caller)
            .expect_err("a non-Developer must be refused at the Admin-creation endpoint");
        assert_eq!(err.error_code(), "DEV_ADMIN_CREATE_FORBIDDEN");
    }
    // The Developer is authorized — and the capability it yields is the only key to `create_admin`.
    assert!(authorize_developer(DevCaller::Developer).is_ok());
}

#[test]
fn ac16_create_admin_mints_single_use_server_ttl_invitation() {
    // AC16: a developer-authorized create provisions exactly one pending Admin with exactly one live
    // invitation, expiring at server-time `now + 72h`, whose minted token verifies against the
    // stored at-rest hash (and a different token does not).
    let mut svc = service(MemStore::new(), NOW);
    let dev = authorize_developer(DevCaller::Developer).expect("developer authorized");

    let invite = block_on(svc.create_admin(&dev)).expect("create_admin is infallible in-memory");
    let admin = invite.admin_id;

    assert!(
        svc.store.admin_exists(admin),
        "a pending Admin was provisioned"
    );
    assert_eq!(
        svc.store.live_invitations(admin),
        1,
        "exactly one live invitation per admin (AC16)"
    );
    assert_eq!(
        invite.expires_at.as_secs(),
        NOW + INVITE_TTL_SECS,
        "TTL is server-time now + 72h (validated against the injected Clock, not the device)"
    );

    let stored = svc
        .store
        .live_invitation_hash(admin)
        .expect("the live invitation has a stored hash");
    assert!(
        admin_invitation_token_matches(&key(), &invite.token, &stored),
        "the minted token verifies against its at-rest hash (consume path, T09, will rely on this)"
    );
    assert!(
        !admin_invitation_token_matches(
            &key(),
            &AdminInvitationToken::new("not-the-token"),
            &stored
        ),
        "a different token must not verify (single-use capability, no oracle)"
    );
    assert_eq!(
        svc.store.live_invitation_expiry(admin).map(|e| e.as_secs()),
        Some(NOW + INVITE_TTL_SECS)
    );
}

#[test]
fn i11_admin_invite_token_single_use() {
    // Single-use + regenerate-invalidates-prior (AC16, ADR-0015 recovery): re-inviting an admin
    // mints a fresh invitation that SUPERSEDES the prior one — still exactly one live invitation,
    // the new token is live, and the OLD token is no longer live (its link is dead).
    let mut svc = service(MemStore::new(), NOW);
    let dev = authorize_developer(DevCaller::Developer).expect("developer authorized");

    let first = block_on(svc.create_admin(&dev)).unwrap();
    let admin = first.admin_id;
    let live_after_create = svc.store.live_invitation_hash(admin).unwrap();
    assert!(admin_invitation_token_matches(
        &key(),
        &first.token,
        &live_after_create
    ));

    let second = block_on(svc.reissue_admin_invite(&dev, admin))
        .unwrap()
        .expect("re-issue mints a new invitation for an existing admin");

    assert_eq!(
        svc.store.live_invitations(admin),
        1,
        "still exactly one live invitation after re-issue (the prior is superseded, not added)"
    );
    assert_eq!(
        svc.store.total_invitations(admin),
        2,
        "the prior row is superseded, not mutated in place (atomic supersede-then-insert)"
    );

    let live_after_reissue = svc.store.live_invitation_hash(admin).unwrap();
    assert!(
        admin_invitation_token_matches(&key(), &second.token, &live_after_reissue),
        "the re-issued token is the live one"
    );
    assert!(
        !admin_invitation_token_matches(&key(), &first.token, &live_after_reissue),
        "the superseded (first) token is no longer live — single-use preserved"
    );
}

#[test]
fn reissue_unknown_admin_is_a_noop() {
    // A re-invite for an id that was never provisioned mints nothing (no stray invitation).
    let mut svc = service(MemStore::new(), NOW);
    let dev = authorize_developer(DevCaller::Developer).expect("developer authorized");
    let unknown = member_id(0xDEAD_BEEF);

    let res = block_on(svc.reissue_admin_invite(&dev, unknown)).unwrap();
    assert!(
        res.is_none(),
        "re-inviting an unknown admin yields no invitation"
    );
    assert_eq!(svc.store.live_invitations(unknown), 0);
    assert!(!svc.store.admin_exists(unknown));
}

#[test]
fn two_creates_are_independent_admins() {
    // Each create provisions a distinct admin (distinct ids, each with its own live invitation).
    let mut svc = service(MemStore::new(), NOW);
    let dev = authorize_developer(DevCaller::Developer).expect("developer authorized");

    let a = block_on(svc.create_admin(&dev)).unwrap();
    let b = block_on(svc.create_admin(&dev)).unwrap();

    assert_ne!(a.admin_id, b.admin_id, "two creates yield distinct admins");
    assert_eq!(svc.store.live_invitations(a.admin_id), 1);
    assert_eq!(svc.store.live_invitations(b.admin_id), 1);
}
