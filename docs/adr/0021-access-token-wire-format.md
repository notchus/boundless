# ADR-0021: Access-token wire format — opaque-random bearer verified by a constant-time store lookup

- **Status:** Accepted
- **Date:** 2026-06-05
- **Author:** notch
- **Deciders:** notch
- **Relates to:** ADR-0016 (sessions/refresh), ADR-0018 (keyed hash / getrandom backend); I3/I4; R5/R6; spec 001 plan §10-D, tasks T07 (T07-shell-B)
- **Resolves:** the access-token **wire format**, left OPEN in `specs/001-onboarding/plan.md` §10-D (which fixed only the ~15-min TTL).

## Context

ADR-0016 D2 / plan §10-D decided the *shape* of the session: an **indefinite** session with a
short-lived (~15 min) access token, a rotating opaque-256-bit refresh credential (stored HMAC-hashed),
replay detection that **kills the whole family**, and device-token binding (I4) with admin-mediated
invalidation. It deliberately left the access token's **wire format** open ("confirm at
implementation"). Two candidates were on the table:

- **(A) opaque-random bearer** — a 32-byte CSPRNG token, verified server-side by a constant-time
  keyed-HMAC **store lookup** (the idiom already proven for the refresh credential).
- **(B) EdDSA-JWT** — a JWT signed with Ed25519 (dryoc), carrying member/family/exp claims, verified
  by **signature** with no per-request store lookup.

The call was made deliberately (a read-only analysis with four grounded readers + four
perspective-diverse judges; 3–1 for opaque, the lone dissent the deliberately-adversarial steelman),
because the choice forecloses options and touches the load-bearing R5 "working admin revoke" control.

The decisive facts, all repo-grounded:

- **The revocation model designs OUT a stale-token window.** `Session::is_live()` is
  *time-independent* — gated solely on the mutable server-side `family_status`
  (`core/auth/src/session.rs`). Every revocation path is written as immediate/atomic: replay
  "revokes the family atomically … every credential in the lineage stops working"
  (`core/server/src/refresh.rs`); device invalidation guarantees "**no stale token survives**"
  (`core/server/src/service.rs`). An opaque store-lookup token re-reads `family_status` on every
  authed request, so a revoke takes effect on the *next* request. A signature-only JWT keeps
  verifying until its ~15-min `exp` **unless** a server-side denylist is added.
- **R5 is an ADR-committed envelope requiring "working admin revoke"** as one of three compensating
  controls for the indefinite refresh credential (plan §10-D R5 row; "If any control is descoped,
  the residual leaves ADR-0016's accepted envelope → revisit the ADR"). A JWT-without-denylist
  weakens that control for the ~15-min window.
- **Key-management asymmetry.** A JWT needs a *new* server-side Ed25519 signing key provisioned +
  rotated in Secrets Store — a whole subsystem (the analogous manifest signer is ~1 week + quarterly
  rotation, and is itself deferred). An opaque token needs **no** signing key — only the CSPRNG both
  designs already require for the refresh credential.
- **Fit to the as-built port.** `SecretSource::fresh_access()` is arg-less by design, mirroring
  `fresh_refresh()`; `AccessToken` is a tainted `String` newtype with no `Debug`/`Display`/`Serialize`.
  Opaque is a drop-in. JWT forces reshaping the port to take claims **and** reordering
  `mint_session`/`rotate_session` (today the access token is minted *before* the session family id
  exists) **and** updating test doubles in two crates.
- **The JWT edge-cost win is largely illusory here.** Authed group operations already route
  Worker→`GroupHub` DO and frequently already hit Postgres via Hyperdrive for the actual operation;
  QPS is tiny (a rider opens the app ~weekly). To honor "no stale token survives" a JWT would force a
  per-request denylist anyway — reintroducing the lookup that was its only advantage.

## Decision

**The Boundless server access token is a 256-bit (32-byte) CSPRNG opaque bearer**, encoded as an
ASCII string the client carries verbatim in `Authorization` and **treats as opaque** (never decodes).
It is verified server-side by a **constant-time keyed-HMAC store lookup** against the sessions store —
the same idiom already proven for the refresh credential
(`boundless_crypto::access_token_hash`/`access_token_matches`, domain tag
`boundless:access-token:v1`) — re-reading the family's mutable status on every authed request, so
admin-revoke, device-invalidation, and refresh-replay family-kill take effect on the **next** request.

`SecretSource::fresh_access()` stays **arg-less** (no claims, no signing key). The ~15-min access TTL
(already decided) governs only the client's silent-refresh cadence (`needs_refresh`), not server-side
liveness. dryoc Ed25519 remains the **sole signature** implementation, used only for the KV manifest
(server signs / client verifies — the opposite trust direction from a token, which the server both
signs and verifies).

**Reversibility is preserved by the opacity rule:** because the client never parses the token and a
~15-min token fully turns over within minutes, the format is server-internal and can be switched later
behind `SecretSource` with low O1/N-2 exposure. The hard rule: **the client must never decode the
access token.**

## Considered alternatives

### (B) EdDSA-JWT — stateless signature-only verification, member/family/exp claims

**Rejected.** Cannot be revoked before its ~15-min `exp` without a server-side denylist, contradicting
the as-built time-independent, `family_status`-gated revocation model ("no stale token survives") and
weakening the ADR-0016/R5-committed "working admin revoke" control (tripping the plan §10-D "revisit
the ADR" clause). Requires reshaping the arg-less `SecretSource::fresh_access()` port + reordering
`mint_session`/`rotate_session` (the access token is minted before the family id exists) + updating
test doubles in two crates, **plus** a new server-side Ed25519 signing-key subsystem in Secrets Store
with rotation (explicitly deferred). Its only real win — no per-request lookup — is largely moot here
(authed requests already route through the DO and often already hit Postgres; QPS is tiny), and a
revocation denylist would reintroduce the lookup anyway. (P2 note: `member_id`/`family_id` are opaque
ids, *not* in the P2 PII set, so JWT claims would not be a P2 violation in themselves — but JWT claims
are base64url-readable, so an accidentally-logged JWT leaks those ids in cleartext, a strictly larger
blast radius than an opaque blob.)

### Hybrid — short-lived JWT for read-path stateless verification + a killed-family denylist on writes

**Rejected (the strongest alternative).** It correctly targets JWT's win at high-frequency future
rider reads, but it requires explicitly amending the R5 accepted-residual envelope for a bounded
15-min access window, keeps the full signing-key subsystem + the port reshape + the mint/rotate
reorder, and leaves the system with **two** verification idioms (signature + denylist) where the
denylist reintroduces a per-request lookup precisely where revocation matters. Net: more moving parts
for a latency saving this traffic profile cannot justify. **Revisit only** if a future spec (004+)
introduces a genuinely hot, pure-edge rider read **and** a measured cross-region latency problem
appears — and only behind the opacity rule that keeps the switch cheap.

### Reuse the manifest Ed25519 key for token signing

**Rejected — category error.** The manifest key is *server-signs / client-verifies*, so its **public**
half is bundled into every client (ADR-0014). An access-token key is *server-signs / server-verifies*,
so it must stay **server-side** in Secrets Store and never be client-bundled. They are distinct secrets
with opposite distribution; conflating them would break the trust boundary.

## Consequences

### Positive

- **Strengthens the R5/R6 revocation envelope:** outstanding access tokens honor
  admin-revoke / device-invalidation / family-kill on the next request, satisfying the "no stale token
  survives" intent — **no R5-envelope amendment / ADR-0016 revisit needed** (unlike the JWT/hybrid
  paths).
- **Zero new key-management infra:** no signing key, no rotation subsystem — only the CSPRNG
  `SecretSource` (already required for the refresh credential). This removes the single biggest
  deferred-infra blocker, so the mint side + the at-rest hash primitive ship now, fully host-testable.
- **One credential-verification idiom (P4):** both refresh and access verify by constant-time keyed-HMAC
  store lookup. The new `access_token_hash`/`access_token_matches` copy the audited
  `refresh_token_hash`/`_matches` primitive with a distinct domain tag.

### Negative / costs (carried into `DEFERRED.md` for the Worker-runtime slice)

- A per-authed-request **store lookup** is the steady-state cost. **Guard-rail (must be honored when
  the Worker wires verification):** it must NOT be a naive standalone Neon round trip — fold it into
  the request's existing group-scoped RLS transaction, or serve `token-hash → family_status` from
  `GroupHub` DO in-memory state, and on any revoke the DO/Worker cache **must write-through/evict**
  (authoritative-on-revoke, not TTL-expiry), or it recreates the very stale window JWT was rejected
  for. Honest caveat: the Worker authenticates *before* the DO RPC, so "fold into the DO" means
  folding auth into that RPC, not a separate pre-call.
- More `sessions` rows touched per auth than a stateless token (an operability/scale cost under
  indefinite sessions across a large group, not a security cost) — accept and monitor; revisit only at
  measured scale.

### Neutral / follow-ups

- `core/crypto` gains `access_token_hash`/`access_token_matches` + `AccessTokenHash` (T07-shell-B).
- `core/server` gains a reference production `RngSecretSource<R: RngCore + CryptoRng>` (RNG **injected**,
  so the core stays randomness-free + wasm-safe; the Worker injects a getrandom-backed RNG, tests a
  seeded `ChaCha20Rng`).
- The **access-token store column + per-request verify lookup** (migration + `PgAuthStore` method + the
  DO-fold/write-through-on-revoke guard-rail above) lands with the Worker runtime — `DEFERRED.md`.

## References

- `core/auth/src/session.rs` (`is_live` time-independent; refresh rotation/replay) · `core/server/src/{refresh,service}.rs` ("no stale token survives" / atomic revoke)
- `core/server/src/ports.rs` (`SecretSource`, arg-less `fresh_access`) · `core/crypto/src/hashing.rs` (`refresh_token_hash`/`_matches` — the template)
- ADR-0016 D2 (sessions/refresh) · ADR-0018 (getrandom `wasm_js` backend) · plan §10-D (R5/R6; the OPEN format) · `docs/privacy-invariants.md` (P2 PII set; I4)
